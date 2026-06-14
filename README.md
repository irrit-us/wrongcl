# wrongcl

Flutter desktop client for a wrongsv raw VLESS TCP server.

The Rust core is also a headless client. The Flutter UI calls the same Rust
library through Dart FFI. The native layer can:

- start and stop a local SOCKS5 CONNECT proxy
- tunnel SOCKS5 TCP connections through wrongsv raw VLESS TCP
- run a direct probe through wrongsv and show the first response bytes
- report connection-manager state and byte counters

Current verified scope is Linux desktop and wrongsv `configs/basic-tcp.toml`.

## Build

Build the headless Rust client:

```bash
cargo build --manifest-path rust/Cargo.toml --bin wrongcl-headless
```

Install Flutter, then build the desktop app:

```bash
flutter pub get
flutter build linux
```

The Linux CMake build compiles `rust/Cargo.toml` and bundles
`libwrongcl_native.so` into the Flutter app.

## Test

```bash
cargo test --manifest-path rust/Cargo.toml
flutter test
flutter build linux
```

## Headless Usage

Generate a config:

```bash
cargo run --manifest-path rust/Cargo.toml --bin wrongcl-headless -- config-example > wrongcl.toml
```

Start a local SOCKS5 proxy:

```bash
cargo run --manifest-path rust/Cargo.toml --bin wrongcl-headless -- serve --config wrongcl.toml
```

Run a direct probe through wrongsv:

```bash
cargo run --manifest-path rust/Cargo.toml --bin wrongcl-headless -- probe \
  --config wrongcl.toml \
  --target-host example.com \
  --target-port 80
```

You can also skip the config file and pass `--server-host`, `--server-port`,
`--uuid`, `--listen-host`, and `--listen-port` directly.

## Local Smoke Test

In the wrongsv server repo:

```bash
cargo run -- --config configs/basic-tcp.toml
```

In this repo:

```bash
flutter run -d linux
```

Use the default UUID from `configs/basic-tcp.toml`, set the server host/port,
start the SOCKS5 proxy, then run a probe or point a local SOCKS5-capable client
at `127.0.0.1:1080`.
