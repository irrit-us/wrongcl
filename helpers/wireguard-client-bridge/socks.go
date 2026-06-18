package main

import (
	"context"
	"errors"
	"fmt"
	"io"
	stdnet "net"
	"strings"
	"sync"
	"time"
)

const socksHandshakeTimeout = 10 * time.Second

func runSocksServer(ctx context.Context, listen string, runtime *wireguardRuntime) error {
	listener, err := stdnet.Listen("tcp", listen)
	if err != nil {
		return err
	}
	defer listener.Close()

	go func() {
		<-ctx.Done()
		_ = listener.Close()
	}()

	for {
		conn, err := listener.Accept()
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return err
		}
		go func(conn stdnet.Conn) {
			defer conn.Close()
			if err := handleSocksConn(ctx, conn, runtime); err != nil && !errors.Is(err, io.EOF) {
				// local helper only; keep logs concise
				_ = err
			}
		}(conn)
	}
}

func handleSocksConn(ctx context.Context, conn stdnet.Conn, runtime *wireguardRuntime) error {
	_ = conn.SetDeadline(time.Now().Add(socksHandshakeTimeout))
	version := make([]byte, 2)
	if _, err := io.ReadFull(conn, version); err != nil {
		return err
	}
	if version[0] != 0x05 {
		return fmt.Errorf("unsupported socks version %d", version[0])
	}
	methods := make([]byte, int(version[1]))
	if _, err := io.ReadFull(conn, methods); err != nil {
		return err
	}
	if _, err := conn.Write([]byte{0x05, 0x00}); err != nil {
		return err
	}

	header := make([]byte, 4)
	if _, err := io.ReadFull(conn, header); err != nil {
		return err
	}
	if header[0] != 0x05 {
		return fmt.Errorf("invalid request version %d", header[0])
	}
	target, err := readSocksTarget(conn, header[3])
	if err != nil {
		return err
	}

	switch header[1] {
	case 0x01:
		return handleConnect(ctx, conn, runtime, target)
	case 0x03:
		return handleUDPAssociate(ctx, conn, runtime)
	default:
		_, _ = conn.Write([]byte{0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return fmt.Errorf("unsupported socks command %d", header[1])
	}
}

func handleConnect(
	ctx context.Context,
	conn stdnet.Conn,
	runtime *wireguardRuntime,
	target targetAddr,
) error {
	remote, err := resolveTCPTarget(target)
	if err != nil {
		_, _ = conn.Write([]byte{0x05, 0x04, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return err
	}
	upstream, err := runtime.dialTCP(ctx, remote)
	if err != nil {
		_, _ = conn.Write([]byte{0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return err
	}
	defer upstream.Close()

	if _, err := conn.Write([]byte{0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0}); err != nil {
		return err
	}
	_ = conn.SetDeadline(time.Time{})
	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		_, _ = io.Copy(upstream, conn)
		if tcpConn, ok := upstream.(*stdnet.TCPConn); ok {
			_ = tcpConn.CloseWrite()
		}
	}()
	go func() {
		defer wg.Done()
		_, _ = io.Copy(conn, upstream)
		if tcpConn, ok := conn.(*stdnet.TCPConn); ok {
			_ = tcpConn.CloseWrite()
		}
	}()
	wg.Wait()
	return nil
}

func handleUDPAssociate(
	ctx context.Context,
	conn stdnet.Conn,
	runtime *wireguardRuntime,
) error {
	control, ok := conn.(*stdnet.TCPConn)
	if !ok {
		return fmt.Errorf("expected TCP control connection")
	}
	bindIP := control.LocalAddr().(*stdnet.TCPAddr).IP
	if bindIP == nil || bindIP.IsUnspecified() {
		bindIP = stdnet.IPv4(127, 0, 0, 1)
	}
	relay, err := stdnet.ListenUDP("udp", &stdnet.UDPAddr{IP: bindIP, Port: 0})
	if err != nil {
		_, _ = conn.Write([]byte{0x05, 0x01, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return err
	}
	defer relay.Close()

	addr := relay.LocalAddr().(*stdnet.UDPAddr)
	if ip4 := addr.IP.To4(); ip4 != nil {
		reply := []byte{0x05, 0x00, 0x00, 0x01}
		reply = append(reply, ip4...)
		reply = append(reply, byte(addr.Port>>8), byte(addr.Port))
		if _, err := conn.Write(reply); err != nil {
			return err
		}
	} else {
		reply := []byte{0x05, 0x00, 0x00, 0x04}
		reply = append(reply, addr.IP.To16()...)
		reply = append(reply, byte(addr.Port>>8), byte(addr.Port))
		if _, err := conn.Write(reply); err != nil {
			return err
		}
	}

	_ = conn.SetDeadline(time.Time{})
	_ = control.SetReadDeadline(time.Now().Add(100 * time.Millisecond))

	sessions := map[string]*udpSession{}
	var clientPeer *stdnet.UDPAddr
	buf := make([]byte, 64*1024)

	for {
		if closed, err := controlClosed(control); err != nil {
			return err
		} else if closed {
			return nil
		}

		_ = relay.SetReadDeadline(time.Now().Add(50 * time.Millisecond))
		n, peer, err := relay.ReadFromUDP(buf)
		if err != nil {
			if ne, ok := err.(stdnet.Error); ok && ne.Timeout() {
				continue
			}
			return err
		}
		clientPeer = peer
		target, payload, err := parseSocksUDPPacket(buf[:n])
		if err != nil {
			continue
		}
		key := target.key()
		session, ok := sessions[key]
		if !ok {
			session, err = newUDPSession(runtime, target, relay, clientPeer)
			if err != nil {
				continue
			}
			sessions[key] = session
			go session.run(ctx)
		}
		if err := session.send(payload); err != nil {
			delete(sessions, key)
		}
	}
}

type udpSession struct {
	runtime    *wireguardRuntime
	target     targetAddr
	resolved   *stdnet.UDPAddr
	relay      *stdnet.UDPConn
	clientPeer *stdnet.UDPAddr
	conn       gonetUDPConn
	tx         chan []byte
}

type gonetUDPConn interface {
	ReadFrom(b []byte) (int, stdnet.Addr, error)
	WriteTo(b []byte, addr stdnet.Addr) (int, error)
	SetReadDeadline(t time.Time) error
	Close() error
}

func newUDPSession(
	runtime *wireguardRuntime,
	target targetAddr,
	relay *stdnet.UDPConn,
	clientPeer *stdnet.UDPAddr,
) (*udpSession, error) {
	resolved, err := resolveUDPTarget(target)
	if err != nil {
		return nil, err
	}
	conn, err := runtime.dialUDP(resolved)
	if err != nil {
		return nil, err
	}
	return &udpSession{
		runtime:    runtime,
		target:     target,
		resolved:   resolved,
		relay:      relay,
		clientPeer: clientPeer,
		conn:       conn,
		tx:         make(chan []byte, 64),
	}, nil
}

func (s *udpSession) send(payload []byte) error {
	select {
	case s.tx <- append([]byte(nil), payload...):
		return nil
	default:
		return fmt.Errorf("udp session queue full")
	}
}

func (s *udpSession) run(ctx context.Context) {
	defer s.conn.Close()

	go func() {
		for {
			select {
			case <-ctx.Done():
				return
			case payload := <-s.tx:
				if _, err := s.conn.WriteTo(payload, s.resolved); err != nil {
					return
				}
			}
		}
	}()

	buf := make([]byte, 64*1024)
	for {
		_ = s.conn.SetReadDeadline(time.Now().Add(200 * time.Millisecond))
		n, _, err := s.conn.ReadFrom(buf)
		if err != nil {
			if ne, ok := err.(stdnet.Error); ok && ne.Timeout() {
				select {
				case <-ctx.Done():
					return
				default:
					continue
				}
			}
			return
		}
		packet, err := encodeSocksUDPPacket(s.target, buf[:n])
		if err != nil {
			return
		}
		if _, err := s.relay.WriteToUDP(packet, s.clientPeer); err != nil {
			return
		}
	}
}

func controlClosed(conn *stdnet.TCPConn) (bool, error) {
	var byteBuf [1]byte
	_, err := conn.Read(byteBuf[:])
	if err == nil {
		return false, nil
	}
	if errors.Is(err, io.EOF) {
		return true, nil
	}
	if ne, ok := err.(stdnet.Error); ok && ne.Timeout() {
		return false, nil
	}
	return false, err
}

type targetAddr struct {
	host string
	port uint16
}

func (t targetAddr) key() string {
	return fmt.Sprintf("%s:%d", t.host, t.port)
}

func readSocksTarget(conn io.Reader, atyp byte) (targetAddr, error) {
	switch atyp {
	case 0x01:
		buf := make([]byte, 6)
		if _, err := io.ReadFull(conn, buf); err != nil {
			return targetAddr{}, err
		}
		host := stdnet.IP(buf[:4]).String()
		port := uint16(buf[4])<<8 | uint16(buf[5])
		return targetAddr{host: host, port: port}, nil
	case 0x03:
		var lenBuf [1]byte
		if _, err := io.ReadFull(conn, lenBuf[:]); err != nil {
			return targetAddr{}, err
		}
		buf := make([]byte, int(lenBuf[0])+2)
		if _, err := io.ReadFull(conn, buf); err != nil {
			return targetAddr{}, err
		}
		host := string(buf[:lenBuf[0]])
		port := uint16(buf[lenBuf[0]])<<8 | uint16(buf[lenBuf[0]+1])
		return targetAddr{host: host, port: port}, nil
	case 0x04:
		buf := make([]byte, 18)
		if _, err := io.ReadFull(conn, buf); err != nil {
			return targetAddr{}, err
		}
		host := stdnet.IP(buf[:16]).String()
		port := uint16(buf[16])<<8 | uint16(buf[17])
		return targetAddr{host: host, port: port}, nil
	default:
		return targetAddr{}, fmt.Errorf("unsupported atyp %#x", atyp)
	}
}

func parseSocksUDPPacket(packet []byte) (targetAddr, []byte, error) {
	if len(packet) < 4 {
		return targetAddr{}, nil, fmt.Errorf("short socks udp packet")
	}
	if packet[0] != 0 || packet[1] != 0 {
		return targetAddr{}, nil, fmt.Errorf("invalid socks udp reserved bytes")
	}
	if packet[2] != 0 {
		return targetAddr{}, nil, fmt.Errorf("fragmented socks udp packets unsupported")
	}
	target, headerLen, err := parseSocksUDPTarget(packet[3:])
	if err != nil {
		return targetAddr{}, nil, err
	}
	return target, packet[3+headerLen:], nil
}

func parseSocksUDPTarget(packet []byte) (targetAddr, int, error) {
	if len(packet) < 1 {
		return targetAddr{}, 0, fmt.Errorf("missing udp atyp")
	}
	switch packet[0] {
	case 0x01:
		if len(packet) < 7 {
			return targetAddr{}, 0, fmt.Errorf("short udp ipv4 target")
		}
		host := stdnet.IP(packet[1:5]).String()
		port := uint16(packet[5])<<8 | uint16(packet[6])
		return targetAddr{host: host, port: port}, 7, nil
	case 0x03:
		if len(packet) < 2 {
			return targetAddr{}, 0, fmt.Errorf("short udp domain target")
		}
		domainLen := int(packet[1])
		if len(packet) < 4+domainLen {
			return targetAddr{}, 0, fmt.Errorf("short udp domain target")
		}
		host := string(packet[2 : 2+domainLen])
		port := uint16(packet[2+domainLen])<<8 | uint16(packet[3+domainLen])
		return targetAddr{host: host, port: port}, 4 + domainLen, nil
	case 0x04:
		if len(packet) < 19 {
			return targetAddr{}, 0, fmt.Errorf("short udp ipv6 target")
		}
		host := stdnet.IP(packet[1:17]).String()
		port := uint16(packet[17])<<8 | uint16(packet[18])
		return targetAddr{host: host, port: port}, 19, nil
	default:
		return targetAddr{}, 0, fmt.Errorf("unsupported udp atyp %#x", packet[0])
	}
}

func encodeSocksUDPPacket(target targetAddr, payload []byte) ([]byte, error) {
	packet := []byte{0x00, 0x00, 0x00}
	if ip := stdnet.ParseIP(target.host); ip != nil {
		if ip4 := ip.To4(); ip4 != nil {
			packet = append(packet, 0x01)
			packet = append(packet, ip4...)
		} else {
			packet = append(packet, 0x04)
			packet = append(packet, ip.To16()...)
		}
	} else {
		if len(target.host) == 0 || len(target.host) > 255 {
			return nil, fmt.Errorf("invalid target host")
		}
		packet = append(packet, 0x03, byte(len(target.host)))
		packet = append(packet, target.host...)
	}
	packet = append(packet, byte(target.port>>8), byte(target.port))
	packet = append(packet, payload...)
	return packet, nil
}

func resolveTCPTarget(target targetAddr) (*stdnet.TCPAddr, error) {
	return stdnet.ResolveTCPAddr("tcp", fmt.Sprintf("%s:%d", normalizeConnectHost(target.host), target.port))
}

func resolveUDPTarget(target targetAddr) (*stdnet.UDPAddr, error) {
	return stdnet.ResolveUDPAddr("udp", fmt.Sprintf("%s:%d", normalizeConnectHost(target.host), target.port))
}

func normalizeConnectHost(host string) string {
	if strings.Contains(host, ":") && !strings.HasPrefix(host, "[") {
		return "[" + host + "]"
	}
	return host
}
