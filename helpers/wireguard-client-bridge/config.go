package main

import (
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	stdnet "net"
	"net/netip"
	"os"

	cnet "github.com/v2fly/v2ray-core/v5/common/net"
	"github.com/v2fly/v2ray-core/v5/proxy/wireguard/wgcommon"
)

type config struct {
	Listen          string   `json:"listen"`
	ServerEndpoint  string   `json:"server_endpoint"`
	PrivateKey      string   `json:"private_key"`
	PeerPublicKey   string   `json:"peer_public_key"`
	PreSharedKey    string   `json:"pre_shared_key,omitempty"`
	ClientAddresses []string `json:"client_addresses"`
	AllowedIPs      []string `json:"allowed_ips"`
	MTU             int      `json:"mtu"`
	KeepAlive       int64    `json:"keep_alive,omitempty"`
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

	if cfg.Listen == "" {
		return nil, errors.New("missing listen")
	}
	if cfg.ServerEndpoint == "" {
		return nil, errors.New("missing server_endpoint")
	}
	if cfg.PrivateKey == "" {
		return nil, errors.New("missing private_key")
	}
	if cfg.PeerPublicKey == "" {
		return nil, errors.New("missing peer_public_key")
	}
	if len(cfg.ClientAddresses) == 0 {
		return nil, errors.New("missing client_addresses")
	}
	if len(cfg.AllowedIPs) == 0 {
		return nil, errors.New("missing allowed_ips")
	}
	if _, err := stdnet.ResolveTCPAddr("tcp", cfg.Listen); err != nil {
		return nil, fmt.Errorf("invalid listen %q: %w", cfg.Listen, err)
	}
	if _, err := stdnet.ResolveUDPAddr("udp", cfg.ServerEndpoint); err != nil {
		return nil, fmt.Errorf("invalid server_endpoint %q: %w", cfg.ServerEndpoint, err)
	}
	if cfg.MTU <= 0 {
		cfg.MTU = 1400
	}
	if cfg.KeepAlive <= 0 {
		cfg.KeepAlive = 25
	}
	return &cfg, nil
}

func parseInterfaceAddrs(values []string) ([]netip.Addr, error) {
	addresses := make([]netip.Addr, 0, len(values))
	for _, value := range values {
		if value == "" {
			return nil, errors.New("empty client address")
		}
		if prefix, err := netip.ParsePrefix(value); err == nil {
			addr := prefix.Addr()
			if prefix.Bits() != addr.BitLen() {
				return nil, fmt.Errorf(
					"client address %q must use /32 for IPv4 or /128 for IPv6",
					value,
				)
			}
			addresses = append(addresses, addr)
			continue
		}
		addr, err := netip.ParseAddr(value)
		if err != nil {
			return nil, fmt.Errorf("invalid client address %q: %w", value, err)
		}
		addresses = append(addresses, addr)
	}
	return addresses, nil
}

func decodeWGKey(value string) ([]byte, error) {
	key, err := base64.StdEncoding.DecodeString(value)
	if err != nil {
		return nil, err
	}
	if len(key) != 32 {
		return nil, fmt.Errorf("expected 32 bytes, got %d", len(key))
	}
	return key, nil
}

func buildDeviceConfig(cfg *config, packetConn cnet.PacketConn) (*wgcommon.DeviceConfig, error) {
	privateKey, err := decodeWGKey(cfg.PrivateKey)
	if err != nil {
		return nil, fmt.Errorf("decode private_key: %w", err)
	}
	peerPublicKey, err := decodeWGKey(cfg.PeerPublicKey)
	if err != nil {
		return nil, fmt.Errorf("decode peer_public_key: %w", err)
	}
	var preSharedKey []byte
	if cfg.PreSharedKey != "" {
		preSharedKey, err = decodeWGKey(cfg.PreSharedKey)
		if err != nil {
			return nil, fmt.Errorf("decode pre_shared_key: %w", err)
		}
	}

	port, err := listenPort(packetConn.LocalAddr())
	if err != nil {
		return nil, err
	}

	return &wgcommon.DeviceConfig{
		PrivateKey: privateKey,
		ListenPort: uint32(port),
		Peers: []*wgcommon.PeerConfig{
			{
				PublicKey:                   peerPublicKey,
				PresharedKey:                preSharedKey,
				AllowedIps:                  cfg.AllowedIPs,
				Endpoint:                    cfg.ServerEndpoint,
				PersistentKeepaliveInterval: cfg.KeepAlive,
			},
		},
		Mtu: uint32(cfg.MTU),
	}, nil
}

func listenPort(addr stdnet.Addr) (uint16, error) {
	switch typed := addr.(type) {
	case *stdnet.UDPAddr:
		return uint16(typed.Port), nil
	default:
		return 0, fmt.Errorf("unexpected listen addr type %T", addr)
	}
}
