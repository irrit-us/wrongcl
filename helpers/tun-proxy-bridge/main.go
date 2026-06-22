package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"net"
	"net/netip"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"

	apptun "github.com/v2fly/v2ray-core/v5/app/tun"
	"github.com/v2fly/v2ray-core/v5/app/tun/device"
	"github.com/v2fly/v2ray-core/v5/app/tun/device/gvisor"
	v2buf "github.com/v2fly/v2ray-core/v5/common/buf"
	v2net "github.com/v2fly/v2ray-core/v5/common/net"
	"github.com/v2fly/v2ray-core/v5/features/policy"
	"github.com/v2fly/v2ray-core/v5/features/routing"
	"github.com/v2fly/v2ray-core/v5/transport"
	gonet "gvisor.dev/gvisor/pkg/tcpip/adapters/gonet"
	"gvisor.dev/gvisor/pkg/tcpip"
	"gvisor.dev/gvisor/pkg/tcpip/network/ipv4"
	"gvisor.dev/gvisor/pkg/tcpip/network/ipv6"
	"gvisor.dev/gvisor/pkg/tcpip/stack"
	"gvisor.dev/gvisor/pkg/tcpip/transport/tcp"
	"gvisor.dev/gvisor/pkg/tcpip/transport/udp"
	"gvisor.dev/gvisor/pkg/waiter"
)

const outboundUDPIdleTimeout = 60 * time.Second

type config struct {
	InterfaceName string   `json:"interface_name"`
	MTU           int      `json:"mtu"`
	AddressCIDR   string   `json:"address_cidr"`
	StackCIDR     string   `json:"stack_cidr"`
	Routes        []string `json:"routes"`
	ProxyHost     string   `json:"proxy_host"`
	ProxyPort     int      `json:"proxy_port"`
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lmicroseconds)

	if len(os.Args) != 3 || os.Args[1] != "--config" {
		log.Fatalf("usage: %s --config /path/to/config.json", os.Args[0])
	}

	cfg, err := loadConfig(os.Args[2])
	if err != nil {
		log.Fatalf("load config: %v", err)
	}

	baseCtx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	ctx, cancel := context.WithCancel(baseCtx)
	defer cancel()
	go func() {
		_, _ = io.Copy(io.Discard, os.Stdin)
		cancel()
	}()

	if err := run(ctx, cfg); err != nil && !errors.Is(err, context.Canceled) {
		log.Fatalf("tun proxy bridge: %v", err)
	}
}

func loadConfig(path string) (*config, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var cfg config
	if err := json.Unmarshal(raw, &cfg); err != nil {
		return nil, err
	}
	if cfg.InterfaceName == "" {
		return nil, errors.New("missing interface_name")
	}
	if cfg.MTU <= 0 {
		cfg.MTU = 1400
	}
	if cfg.AddressCIDR == "" {
		cfg.AddressCIDR = "198.18.0.1/15"
	}
	if cfg.StackCIDR == "" {
		cfg.StackCIDR = cfg.AddressCIDR
	}
	if cfg.ProxyHost == "" {
		cfg.ProxyHost = "127.0.0.1"
	}
	if cfg.ProxyPort <= 0 {
		return nil, errors.New("missing proxy_port")
	}
	return &cfg, nil
}

type socksDispatcher struct {
	proxyAddr string
}

func (d *socksDispatcher) Dispatch(_ context.Context, dest v2net.Destination) (*transport.Link, error) {
	if dest.Network != v2net.Network_TCP {
		return nil, fmt.Errorf("unsupported network %s", dest.Network.SystemString())
	}
	addr, err := netip.ParseAddrPort(dest.NetAddr())
	if err != nil {
		return nil, err
	}
	upstream, err := net.Dial("tcp", d.proxyAddr)
	if err != nil {
		return nil, err
	}
	if err := socks5NoAuth(upstream); err != nil {
		_ = upstream.Close()
		return nil, err
	}
	if err := socks5Connect(upstream, addr); err != nil {
		_ = upstream.Close()
		return nil, err
	}
	return &transport.Link{
		Reader: v2buf.NewReader(upstream),
		Writer: v2buf.NewWriter(upstream),
	}, nil
}

func (d *socksDispatcher) Start() error { return nil }
func (d *socksDispatcher) Close() error { return nil }
func (*socksDispatcher) Type() interface{} {
	return routing.DispatcherType()
}

type staticPolicyManager struct{}

func (staticPolicyManager) Start() error { return nil }
func (staticPolicyManager) Close() error { return nil }
func (staticPolicyManager) Type() interface{} {
	return policy.ManagerType()
}
func (staticPolicyManager) ForLevel(_ uint32) policy.Session {
	return policy.SessionDefault()
}
func (staticPolicyManager) ForSystem() policy.System {
	return policy.System{}
}

func run(ctx context.Context, cfg *config) error {
	tunDev, err := gvisor.New(device.Options{
		Name: cfg.InterfaceName,
		MTU:  uint32(cfg.MTU),
	})
	if err != nil {
		return fmt.Errorf("open tun device: %w", err)
	}
	if closer, ok := tunDev.(interface{ Close() }); ok {
		defer closer.Close()
	}

	s, nicID, err := createStack(tunDev, cfg)
	if err != nil {
		return err
	}
	defer func() {
		s.Close()
		s.Wait()
	}()

	proxyAddr := net.JoinHostPort(cfg.ProxyHost, strconv.Itoa(cfg.ProxyPort))
	installOutboundTCPForwarder(s, nicID, proxyAddr)
	installOutboundUDPForwarder(s, nicID, proxyAddr)

	log.Printf("tun proxy bridge active on %s via socks5 %s", cfg.InterfaceName, proxyAddr)
	<-ctx.Done()
	return ctx.Err()
}

func createStack(linkedEndpoint stack.LinkEndpoint, cfg *config) (*stack.Stack, tcpip.NICID, error) {
	addr, err := parseAddressCIDR(cfg.StackCIDR)
	if err != nil {
		return nil, 0, err
	}
	routes, err := parseRoutes(cfg.Routes)
	if err != nil {
		return nil, 0, err
	}

	s := stack.New(stack.Options{
		NetworkProtocols: []stack.NetworkProtocolFactory{
			ipv4.NewProtocol,
			ipv6.NewProtocol,
		},
		TransportProtocols: []stack.TransportProtocolFactory{
			tcp.NewProtocol,
			udp.NewProtocol,
		},
	})

	nicID := s.NextNICID()
	if err := s.CreateNICWithOptions(nicID, linkedEndpoint, stack.NICOptions{Disabled: false}); err != nil {
		s.Close()
		return nil, 0, fmt.Errorf("create nic: %v", err)
	}

	if err := s.AddProtocolAddress(nicID, addr.protocolAddress, stack.AddressProperties{}); err != nil {
		s.Close()
		return nil, 0, fmt.Errorf("add protocol address: %v", err)
	}
	for _, route := range routes {
		if route.Bits() == route.Addr().BitLen() {
			if err := s.AddProtocolAddress(
				nicID,
				tcpip.ProtocolAddress{
					Protocol: addr.protocolAddress.Protocol,
					AddressWithPrefix: tcpip.AddressWithPrefix{
						Address:   tcpip.AddrFromSlice(route.Addr().AsSlice()),
						PrefixLen: route.Bits(),
					},
				},
				stack.AddressProperties{},
			); err != nil {
				s.Close()
				return nil, 0, fmt.Errorf("add host route protocol address: %v", err)
			}
		}
	}
	s.SetRouteTable(routesForNic(nicID, routes))
	if err := s.SetPromiscuousMode(nicID, true); err != nil {
		s.Close()
		return nil, 0, fmt.Errorf("enable promiscuous mode: %v", err)
	}
	if err := s.SetSpoofing(nicID, true); err != nil {
		s.Close()
		return nil, 0, fmt.Errorf("enable spoofing: %v", err)
	}
	return s, nicID, nil
}

type parsedAddress struct {
	protocolAddress tcpip.ProtocolAddress
}

func parseAddressCIDR(value string) (*parsedAddress, error) {
	ip, network, err := net.ParseCIDR(value)
	if err != nil {
		return nil, fmt.Errorf("parse address_cidr %q: %w", value, err)
	}
	prefix, _ := network.Mask.Size()
	addr := tcpip.AddrFromSlice(ip)
	protocol := ipv4.ProtocolNumber
	if ip.To4() == nil {
		protocol = ipv6.ProtocolNumber
	}
	return &parsedAddress{
		protocolAddress: tcpip.ProtocolAddress{
			Protocol: protocol,
			AddressWithPrefix: tcpip.AddressWithPrefix{
				Address:   addr,
				PrefixLen: prefix,
			},
		},
	}, nil
}

func parseRoutes(values []string) ([]netip.Prefix, error) {
	out := make([]netip.Prefix, 0, len(values))
	for _, value := range values {
		prefix, err := netip.ParsePrefix(value)
		if err != nil {
			return nil, fmt.Errorf("parse route %q: %w", value, err)
		}
		out = append(out, prefix)
	}
	return out, nil
}

func routesForNic(nicID tcpip.NICID, routes []netip.Prefix) []tcpip.Route {
	out := make([]tcpip.Route, 0, len(routes))
	for _, prefix := range routes {
		subnet := tcpip.AddressWithPrefix{
			Address:   tcpip.AddrFromSlice(prefix.Addr().AsSlice()),
			PrefixLen: prefix.Bits(),
		}.Subnet()
		out = append(out, tcpip.Route{
			Destination: subnet,
			NIC:         nicID,
		})
	}
	return out
}

func installOutboundTCPForwarder(s *stack.Stack, nicID tcpip.NICID, proxyAddr string) {
	_ = nicID
	dispatcher := &socksDispatcher{proxyAddr: proxyAddr}
	manager := staticPolicyManager{}
	cfg := &apptun.Config{Tag: "wrongcl-tun"}
	if err := apptun.SetTCPHandler(context.Background(), dispatcher, manager, cfg)(s); err != nil {
		log.Printf("tun tcp handler install failed: %v", err)
	}
}

func installOutboundUDPForwarder(s *stack.Stack, nicID tcpip.NICID, proxyAddr string) {
	var localAddrs sync.Map
	forwarder := udp.NewForwarder(s, func(req *udp.ForwarderRequest) bool {
		id := req.ID()
		target, err := addrPortFromID(id.LocalAddress, id.LocalPort)
		if err != nil {
			log.Printf("tun udp target parse: %v", err)
			return true
		}
		if err := ensureLocalAddress(s, nicID, &localAddrs, id.LocalAddress); err != nil {
			log.Printf("tun udp ensure local address %s: %v", id.LocalAddress, err)
			return true
		}
		var wq waiter.Queue
		ep, udpErr := req.CreateEndpoint(&wq)
		if udpErr != nil {
			log.Printf("tun udp endpoint %s: %v", target, udpErr)
			return true
		}
		conn := gonet.NewUDPConn(&wq, ep)
		go relayUDPConnViaSocks(conn, target, proxyAddr, outboundUDPIdleTimeout)
		return true
	})
	s.SetTransportProtocolHandler(udp.ProtocolNumber, forwarder.HandlePacket)
}

func relayConnViaSocks(client net.Conn, target netip.AddrPort, proxyAddr string) {
	defer client.Close()

	upstream, err := net.Dial("tcp", proxyAddr)
	if err != nil {
		log.Printf("tun tcp dial proxy %s: %v", proxyAddr, err)
		return
	}
	defer upstream.Close()

	if err := socks5NoAuth(upstream); err != nil {
		log.Printf("tun tcp socks handshake: %v", err)
		return
	}
	if err := socks5Connect(upstream, target); err != nil {
		log.Printf("tun tcp socks connect %s: %v", target, err)
		return
	}

	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		if _, err := io.Copy(upstream, client); err != nil {
			log.Printf("tun tcp copy client->proxy %s: %v", target, err)
		}
		if tcpConn, ok := upstream.(*net.TCPConn); ok {
			_ = tcpConn.CloseWrite()
		}
	}()
	go func() {
		defer wg.Done()
		if _, err := io.Copy(client, upstream); err != nil {
			log.Printf("tun tcp copy proxy->client %s: %v", target, err)
		}
		if tcpConn, ok := client.(*net.TCPConn); ok {
			_ = tcpConn.CloseWrite()
		}
	}()
	wg.Wait()
}

func relayUDPConnViaSocks(client *gonet.UDPConn, target netip.AddrPort, proxyAddr string, idleTimeout time.Duration) {
	defer client.Close()

	control, err := net.Dial("tcp", proxyAddr)
	if err != nil {
		log.Printf("tun udp dial proxy %s: %v", proxyAddr, err)
		return
	}
	defer control.Close()
	if err := socks5NoAuth(control); err != nil {
		log.Printf("tun udp socks handshake: %v", err)
		return
	}
	relayAddr, err := socks5UDPAssociate(control)
	if err != nil {
		log.Printf("tun udp associate: %v", err)
		return
	}

	upstream, err := net.DialUDP("udp", nil, relayAddr)
	if err != nil {
		log.Printf("tun udp relay dial %s: %v", relayAddr, err)
		return
	}
	defer upstream.Close()

	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		buf := make([]byte, 65535)
		for {
			_ = client.SetReadDeadline(time.Now().Add(idleTimeout))
			n, err := client.Read(buf)
			if err != nil {
				return
			}
			packet, err := encodeSocks5UDPPacket(target, buf[:n])
			if err != nil {
				log.Printf("tun udp encode %s: %v", target, err)
				return
			}
			_ = upstream.SetWriteDeadline(time.Now().Add(idleTimeout))
			if _, err := upstream.Write(packet); err != nil {
				return
			}
		}
	}()
	go func() {
		defer wg.Done()
		buf := make([]byte, 65535)
		for {
			_ = upstream.SetReadDeadline(time.Now().Add(idleTimeout))
			n, err := upstream.Read(buf)
			if err != nil {
				return
			}
			_, payload, err := parseSocks5UDPPacket(buf[:n])
			if err != nil {
				log.Printf("tun udp decode: %v", err)
				return
			}
			_ = client.SetWriteDeadline(time.Now().Add(idleTimeout))
			if _, err := client.Write(payload); err != nil {
				return
			}
		}
	}()
	wg.Wait()
}

func socks5NoAuth(conn net.Conn) error {
	if _, err := conn.Write([]byte{0x05, 0x01, 0x00}); err != nil {
		return err
	}
	reply := make([]byte, 2)
	if _, err := io.ReadFull(conn, reply); err != nil {
		return err
	}
	if reply[0] != 0x05 || reply[1] != 0x00 {
		return fmt.Errorf("unexpected socks auth reply %v", reply)
	}
	return nil
}

func socks5Connect(conn net.Conn, target netip.AddrPort) error {
	addr, err := encodeSocks5Address(target)
	if err != nil {
		return err
	}
	req := append([]byte{0x05, 0x01, 0x00}, addr...)
	if _, err := conn.Write(req); err != nil {
		return err
	}
	return readSocks5Reply(conn)
}

func socks5UDPAssociate(conn net.Conn) (*net.UDPAddr, error) {
	req := []byte{0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0}
	if _, err := conn.Write(req); err != nil {
		return nil, err
	}
	header := make([]byte, 4)
	if _, err := io.ReadFull(conn, header); err != nil {
		return nil, err
	}
	if header[0] != 0x05 || header[1] != 0x00 {
		return nil, fmt.Errorf("unexpected udp associate reply %v", header)
	}
	addr, err := readSocks5Address(conn, header[3])
	if err != nil {
		return nil, err
	}
	resolved, err := net.ResolveUDPAddr("udp", addr.String())
	if err != nil {
		return nil, err
	}
	return resolved, nil
}

func readSocks5Reply(conn net.Conn) error {
	header := make([]byte, 4)
	if _, err := io.ReadFull(conn, header); err != nil {
		return err
	}
	if header[0] != 0x05 || header[1] != 0x00 {
		return fmt.Errorf("unexpected socks reply %v", header)
	}
	if _, err := readSocks5Address(conn, header[3]); err != nil {
		return err
	}
	return nil
}

func encodeSocks5Address(target netip.AddrPort) ([]byte, error) {
	addr := target.Addr()
	port := target.Port()
	switch {
	case addr.Is4():
		ip := addr.As4()
		out := []byte{0x01}
		out = append(out, ip[:]...)
		out = append(out, byte(port>>8), byte(port))
		return out, nil
	case addr.Is6():
		ip := addr.As16()
		out := []byte{0x04}
		out = append(out, ip[:]...)
		out = append(out, byte(port>>8), byte(port))
		return out, nil
	default:
		return nil, fmt.Errorf("unsupported target address %s", target)
	}
}

func readSocks5Address(r io.Reader, atyp byte) (netip.AddrPort, error) {
	switch atyp {
	case 0x01:
		var ip [4]byte
		if _, err := io.ReadFull(r, ip[:]); err != nil {
			return netip.AddrPort{}, err
		}
		var port [2]byte
		if _, err := io.ReadFull(r, port[:]); err != nil {
			return netip.AddrPort{}, err
		}
		return netip.AddrPortFrom(netip.AddrFrom4(ip), uint16(port[0])<<8|uint16(port[1])), nil
	case 0x04:
		var ip [16]byte
		if _, err := io.ReadFull(r, ip[:]); err != nil {
			return netip.AddrPort{}, err
		}
		var port [2]byte
		if _, err := io.ReadFull(r, port[:]); err != nil {
			return netip.AddrPort{}, err
		}
		return netip.AddrPortFrom(netip.AddrFrom16(ip), uint16(port[0])<<8|uint16(port[1])), nil
	case 0x03:
		var size [1]byte
		if _, err := io.ReadFull(r, size[:]); err != nil {
			return netip.AddrPort{}, err
		}
		host := make([]byte, size[0])
		if _, err := io.ReadFull(r, host); err != nil {
			return netip.AddrPort{}, err
		}
		var port [2]byte
		if _, err := io.ReadFull(r, port[:]); err != nil {
			return netip.AddrPort{}, err
		}
		hostPort := net.JoinHostPort(string(host), strconv.Itoa(int(uint16(port[0])<<8|uint16(port[1]))))
		return netip.ParseAddrPort(hostPort)
	default:
		return netip.AddrPort{}, fmt.Errorf("unsupported socks address type %#x", atyp)
	}
}

func encodeSocks5UDPPacket(target netip.AddrPort, payload []byte) ([]byte, error) {
	addr, err := encodeSocks5Address(target)
	if err != nil {
		return nil, err
	}
	out := []byte{0x00, 0x00, 0x00}
	out = append(out, addr...)
	out = append(out, payload...)
	return out, nil
}

func parseSocks5UDPPacket(packet []byte) (netip.AddrPort, []byte, error) {
	if len(packet) < 4 {
		return netip.AddrPort{}, nil, errors.New("short socks5 udp packet")
	}
	if packet[0] != 0x00 || packet[1] != 0x00 {
		return netip.AddrPort{}, nil, errors.New("invalid socks5 udp reserved bytes")
	}
	if packet[2] != 0x00 {
		return netip.AddrPort{}, nil, errors.New("fragmented socks5 udp packets are unsupported")
	}
	reader := bytes.NewReader(packet[4:])
	target, err := readSocks5Address(reader, packet[3])
	if err != nil {
		return netip.AddrPort{}, nil, err
	}
	consumed := len(packet[4:]) - reader.Len()
	return target, packet[4+consumed:], nil
}

func addrPortFromID(addr tcpip.Address, port uint16) (netip.AddrPort, error) {
	host := net.IP(addr.AsSlice()).String()
	return netip.ParseAddrPort(net.JoinHostPort(host, strconv.Itoa(int(port))))
}

func ensureLocalAddress(
	s *stack.Stack,
	nicID tcpip.NICID,
	seen *sync.Map,
	addr tcpip.Address,
) error {
	key := addr.String()
	if _, loaded := seen.LoadOrStore(key, struct{}{}); loaded {
		return nil
	}
	protocol := ipv4.ProtocolNumber
	prefixLen := 32
	if addr.Len() == 16 {
		protocol = ipv6.ProtocolNumber
		prefixLen = 128
	}
	err := s.AddProtocolAddress(
		nicID,
		tcpip.ProtocolAddress{
			Protocol: protocol,
			AddressWithPrefix: tcpip.AddressWithPrefix{
				Address:   addr,
				PrefixLen: prefixLen,
			},
		},
		stack.AddressProperties{},
	)
	if err == nil {
		return nil
	}
	seen.Delete(key)
	errText := err.String()
	if strings.Contains(errText, "duplicate") || strings.Contains(errText, "exists") {
		return nil
	}
	return fmt.Errorf("%v", err)
}
