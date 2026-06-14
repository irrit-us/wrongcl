# wrongcl

Flutter desktop client for a wrongsv raw VLESS TCP server.

The Flutter UI calls a Rust `cdylib` through Dart FFI. The native library can:

- start and stop a local SOCKS5 CONNECT proxy
- tunnel SOCKS5 TCP connections through wrongsv raw VLESS TCP
- run a direct probe through wrongsv and show the first response bytes

Current verified scope is Linux desktop and wrongsv `configs/basic-tcp.toml`.

## Build

Install Flutter, then run:

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
