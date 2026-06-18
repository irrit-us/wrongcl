package main

import (
	"context"
	"fmt"
	stdnet "net"
	"strings"

	cnet "github.com/v2fly/v2ray-core/v5/common/net"
	"github.com/v2fly/v2ray-core/v5/common/packetswitch"
	"github.com/v2fly/v2ray-core/v5/common/packetswitch/gvisorstack"
	"github.com/v2fly/v2ray-core/v5/common/packetswitch/interconnect"
	"github.com/v2fly/v2ray-core/v5/proxy/wireguard/wgcommon"
	"gvisor.dev/gvisor/pkg/tcpip"
	"gvisor.dev/gvisor/pkg/tcpip/adapters/gonet"
	"gvisor.dev/gvisor/pkg/tcpip/network/ipv4"
	"gvisor.dev/gvisor/pkg/tcpip/network/ipv6"
	"gvisor.dev/gvisor/pkg/tcpip/stack"
	"gvisor.dev/gvisor/pkg/tcpip/transport/icmp"
	"gvisor.dev/gvisor/pkg/tcpip/transport/tcp"
	"gvisor.dev/gvisor/pkg/tcpip/transport/udp"
)

type wireguardRuntime struct {
	stack    *stack.Stack
	device   *wgcommon.WrappedWireguardDevice
	packet   stdnet.PacketConn
	adaptor  *gvisorstack.NetworkLayerDeviceToGvisorLinkEndpointAdaptor
	tunReady bool
}

func startWireGuard(ctx context.Context, cfg *config) (*wireguardRuntime, error) {
	packetConn, err := listenWireGuardUDP(cfg.ServerEndpoint)
	if err != nil {
		return nil, fmt.Errorf("listen udp: %w", err)
	}

	stackValue, tunnelSide, adaptor, err := buildStack(ctx, cfg.ClientAddresses, cfg.MTU)
	if err != nil {
		_ = packetConn.Close()
		return nil, err
	}

	deviceConfig, err := buildDeviceConfig(cfg, packetConn.(cnet.PacketConn))
	if err != nil {
		adaptor.Close()
		_ = packetConn.Close()
		return nil, err
	}
	device, err := wgcommon.NewWrappedWireguardDevice(ctx, deviceConfig)
	if err != nil {
		adaptor.Close()
		_ = packetConn.Close()
		return nil, fmt.Errorf("new wireguard device: %w", err)
	}

	device.SetTunnel(tunnelSide)
	device.SetConn(packetConn.(cnet.PacketConn))
	if err := device.InitDevice(); err != nil {
		device.Close()
		adaptor.Close()
		_ = packetConn.Close()
		return nil, fmt.Errorf("wireguard init: %w", err)
	}
	if err := device.SetupDeviceWithoutPeers(); err != nil {
		device.Close()
		adaptor.Close()
		_ = packetConn.Close()
		return nil, fmt.Errorf("wireguard setup: %w", err)
	}
	if err := device.AddOrReplacePeers(deviceConfig.GetPeers()); err != nil {
		device.Close()
		adaptor.Close()
		_ = packetConn.Close()
		return nil, fmt.Errorf("wireguard peers: %w", err)
	}
	if err := device.Up(); err != nil {
		device.Close()
		adaptor.Close()
		_ = packetConn.Close()
		return nil, fmt.Errorf("wireguard up: %w", err)
	}

	return &wireguardRuntime{
		stack:    stackValue,
		device:   device,
		packet:   packetConn,
		adaptor:  adaptor,
		tunReady: true,
	}, nil
}

func (w *wireguardRuntime) close() {
	if w == nil {
		return
	}
	if w.device != nil {
		_ = w.device.Close()
	}
	if w.adaptor != nil {
		w.adaptor.Close()
	}
	if w.packet != nil {
		_ = w.packet.Close()
	}
}

func buildStack(
	ctx context.Context,
	clientAddresses []string,
	mtu int,
) (*stack.Stack, packetswitch.NetworkLayerDevice, *gvisorstack.NetworkLayerDeviceToGvisorLinkEndpointAdaptor, error) {
	cable, err := interconnect.NewNetworkLayerCable(ctx)
	if err != nil {
		return nil, nil, nil, fmt.Errorf("new network cable: %w", err)
	}
	adaptor := gvisorstack.NewNetworkLayerDeviceToGvisorLinkEndpointAdaptor(ctx, mtu, cable.GetRSideDevice())
	stackValue := stack.New(stack.Options{
		NetworkProtocols: []stack.NetworkProtocolFactory{
			ipv4.NewProtocol,
			ipv6.NewProtocol,
		},
		TransportProtocols: []stack.TransportProtocolFactory{
			tcp.NewProtocol,
			udp.NewProtocol,
			icmp.NewProtocol4,
			icmp.NewProtocol6,
		},
	})

	nicID := stackValue.NextNICID()
	if err := stackValue.CreateNICWithOptions(
		nicID,
		adaptor,
		stack.NICOptions{Disabled: false, QDisc: nil},
	); err != nil {
		adaptor.Close()
		return nil, nil, nil, fmt.Errorf("create nic: %v", err)
	}

	hasIPv4 := false
	hasIPv6 := false
	for _, value := range clientAddresses {
		ip, prefix, err := parseInterfaceCIDR(value)
		if err != nil {
			adaptor.Close()
			return nil, nil, nil, err
		}
		tcpIPAddr := tcpip.AddrFromSlice(ip)
		protocolAddress := tcpip.ProtocolAddress{
			AddressWithPrefix: tcpip.AddressWithPrefix{
				Address:   tcpIPAddr,
				PrefixLen: prefix,
			},
		}
		switch len(ip) {
		case stdnet.IPv4len:
			protocolAddress.Protocol = ipv4.ProtocolNumber
			hasIPv4 = true
		case stdnet.IPv6len:
			protocolAddress.Protocol = ipv6.ProtocolNumber
			hasIPv6 = true
		default:
			adaptor.Close()
			return nil, nil, nil, fmt.Errorf("invalid interface ip length %d", len(ip))
		}
		if err := stackValue.AddProtocolAddress(nicID, protocolAddress, stack.AddressProperties{}); err != nil {
			adaptor.Close()
			return nil, nil, nil, fmt.Errorf("add protocol address: %v", err)
		}
	}

	var routes []tcpip.Route
	if hasIPv4 {
		routes = append(routes, tcpip.Route{Destination: tcpip.AddressWithPrefix{
			Address:   tcpip.AddrFrom4([4]byte{}),
			PrefixLen: 0,
		}.Subnet(), NIC: nicID})
	}
	if hasIPv6 {
		routes = append(routes, tcpip.Route{Destination: tcpip.AddressWithPrefix{
			Address:   tcpip.AddrFrom16([16]byte{}),
			PrefixLen: 0,
		}.Subnet(), NIC: nicID})
	}
	stackValue.SetRouteTable(routes)
	adaptor.SetOnCloseAction(func() {
		stackValue.Close()
	})

	return stackValue, cable.GetLSideDevice(), adaptor, nil
}

func listenWireGuardUDP(serverEndpoint string) (stdnet.PacketConn, error) {
	addr, err := stdnet.ResolveUDPAddr("udp", serverEndpoint)
	if err != nil {
		return nil, err
	}
	if addr.IP != nil && addr.IP.To4() == nil {
		return stdnet.ListenPacket("udp", "[::]:0")
	}
	return stdnet.ListenPacket("udp", "0.0.0.0:0")
}

func parseInterfaceCIDR(value string) ([]byte, int, error) {
	value = strings.TrimSpace(value)
	if value == "" {
		return nil, 0, fmt.Errorf("empty client address")
	}
	if strings.Contains(value, "/") {
		ip, network, err := stdnet.ParseCIDR(value)
		if err != nil {
			return nil, 0, fmt.Errorf("parse client address %q: %w", value, err)
		}
		ip = normalizeIP(ip)
		if ip == nil {
			return nil, 0, fmt.Errorf("unsupported client ip %q", value)
		}
		prefix, _ := network.Mask.Size()
		return ip, prefix, nil
	}
	ip := normalizeIP(stdnet.ParseIP(value))
	if ip == nil {
		return nil, 0, fmt.Errorf("parse client address %q: invalid ip", value)
	}
	if ip.To4() != nil {
		return ip, 32, nil
	}
	return ip, 128, nil
}

func normalizeIP(ip stdnet.IP) stdnet.IP {
	if v4 := ip.To4(); v4 != nil {
		return v4
	}
	if v16 := ip.To16(); v16 != nil {
		return v16
	}
	return nil
}

func (w *wireguardRuntime) dialTCP(
	ctx context.Context,
	target *stdnet.TCPAddr,
) (stdnet.Conn, error) {
	fullAddr, protocol := convertToFullAddr(target)
	return gonet.DialContextTCP(ctx, w.stack, fullAddr, protocol)
}

func (w *wireguardRuntime) dialUDP(target *stdnet.UDPAddr) (*gonet.UDPConn, error) {
	var protocol tcpip.NetworkProtocolNumber
	if ip := normalizeIP(target.IP); ip != nil && ip.To4() != nil {
		protocol = ipv4.ProtocolNumber
	} else {
		protocol = ipv6.ProtocolNumber
	}
	return gonet.DialUDP(w.stack, nil, nil, protocol)
}

func convertToFullAddr(endpoint *stdnet.TCPAddr) (tcpip.FullAddress, tcpip.NetworkProtocolNumber) {
	ip := normalizeIP(endpoint.IP)
	if ip == nil {
		panic("invalid tcp target ip")
	}
	var protoNumber tcpip.NetworkProtocolNumber
	if ip.To4() != nil {
		protoNumber = ipv4.ProtocolNumber
	} else {
		protoNumber = ipv6.ProtocolNumber
	}
	return tcpip.FullAddress{
		NIC:  1,
		Addr: tcpip.AddrFromSlice(ip),
		Port: uint16(endpoint.Port),
	}, protoNumber
}
