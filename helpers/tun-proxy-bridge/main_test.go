package main

import (
	"net/netip"
	"testing"
)

func TestEncodeDecodeSocks5UDPPacket(t *testing.T) {
	target := netip.MustParseAddrPort("203.0.113.9:53")
	payload := []byte("ping")

	packet, err := encodeSocks5UDPPacket(target, payload)
	if err != nil {
		t.Fatalf("encodeSocks5UDPPacket: %v", err)
	}

	decodedTarget, decodedPayload, err := parseSocks5UDPPacket(packet)
	if err != nil {
		t.Fatalf("parseSocks5UDPPacket: %v", err)
	}

	if decodedTarget != target {
		t.Fatalf("target mismatch: got %v want %v", decodedTarget, target)
	}
	if string(decodedPayload) != "ping" {
		t.Fatalf("payload mismatch: got %q", decodedPayload)
	}
}
