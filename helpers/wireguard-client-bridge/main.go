package main

import (
	"context"
	"io"
	"log"
	"os"
	"os/signal"
	"syscall"
)

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

	runtime, err := startWireGuard(ctx, cfg)
	if err != nil {
		log.Fatalf("wireguard client bridge: %v", err)
	}
	defer runtime.close()

	if err := runSocksServer(ctx, cfg.Listen, runtime); err != nil && ctx.Err() == nil {
		log.Fatalf("wireguard client bridge socks: %v", err)
	}
}
