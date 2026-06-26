# wrongcl

Flutter and headless client for wrongsv-supported proxy stacks.

The Rust core is also a headless client. The Flutter UI calls the same Rust
library through Dart FFI. The native layer can:

- start and stop a local proxy listener
- accept local SOCKS5 traffic plus HTTP `CONNECT` and absolute-form HTTP forwarding on the same local listener
- tunnel TCP connections and UDP associations through supported wrongsv transports
- inspect and adapt a wrongsv server config into a wrongcl client profile
- load/export wrongcl config files in JSON and TOML
- save, duplicate, safely delete, reset, and reload local client profiles from
  disk through a versioned on-disk profile store that still loads legacy
  profile arrays
- show activity history plus desktop integration state for autostart, system proxy, and tray controls
- run a direct probe through wrongsv and show the first response bytes
- report connection-manager state and byte counters

Current verified scope in this workspace is Linux desktop plus the headless
Rust client. Desktop release-gate verification also exists in CI for Linux,
macOS, and Windows. Android and iOS project/build wiring now exist in-tree,
and FreeBSD support is currently scoped to the headless client rather than a
Flutter UI target.

Verified local proxy coverage currently includes:

- VLESS raw TCP and raw UDP
- VLESS raw TCP with configurable Fragment write-splitting
- VLESS + TLS (TCP and UDP)
- VLESS + AnyTLS (TCP and UDP)
- VLESS + ShadowTLS (TCP and UDP)
- VLESS + REALITY (TCP and UDP, with client public key supplied)
- Hysteria2 over QUIC/TLS (TCP and UDP, including Gecko/Salamander obfs)
- TUIC over QUIC/TLS (TCP and UDP)
- VLESS + QUIC (TCP and UDP)
- VLESS + KCP (TCP and UDP)
- VLESS + Meek, with and without TLS (TCP and UDP)
- VLESS + Google Docs Viewer, with and without TLS (TCP and UDP)
- VLESS + WebTransport (TCP and UDP)
- VLESS + WebSocket, with and without TLS (TCP and UDP)
- VLESS + HTTPUpgrade, with and without TLS (TCP and UDP)
- VLESS + XHTTP, with and without TLS (TCP and UDP)
- VLESS + gRPC, with and without TLS (TCP and UDP)
- VLESS raw TCP with `flow = "xtls-rprx-vision"` for TCP
- Trojan over TLS (TCP and UDP)
- Naive over HTTP/2 CONNECT over TLS (TCP only)
- Shadowsocks AEAD / AEAD-2022 over raw TCP and UDP
- Snell v1 over raw TCP (TCP only)
- Remote mixed proxy backends over SOCKS5 (TCP and UDP) or HTTP CONNECT (TCP)
- WireGuard tunnel service through a built-in userspace runtime (TCP and UDP proxying; imported wrongsv configs still need a client private-key supplied separately)
- Local HTTP `CONNECT` tunneling and absolute-form HTTP forwarding over the same listener as SOCKS5

Catalog coverage for `../protocols.md` is intentionally explicit: wrongcl
implements the wrongsv profile families listed above, reports partial support
for imports that need client-only secrets, and leaves unrelated catalog entries
such as Tor, SSH, Brook, Mieru, Juicity, and TrustTunnel out of the local
proxy runtime rather than advertising unimplemented stacks.

Direct-probe coverage also exists for the same core transport families.

Current remaining gaps are no longer in the implemented TCP/UDP transport
matrix. The main remaining work is:

- desktop product work in Flutter / FFI / persistence / packaging
- client-side prompts for missing fields such as REALITY `public-key`
- host-parity TUN work is still incomplete outside Linux, and imported wrongsv
  WireGuard configs stay partial until the client private-key is supplied

The capability adapter recognizes the rest of wrongsv's profile surface and
reports `supported`, `partial`, or `unsupported` plus structured missing
client-side fields such as REALITY `public-key`. The public wrongsv
inspect/adapt path now resolves the active profile plus payload/base-carrier
shape through shared wrongsv endpoint diagnostics. The loose partial-import
schema, the profile-specific import spec for the currently implemented wrongcl
stacks, the neutral wrongcl config document, the wrongcl support-state /
missing-field overlay, the strict-vs-draft adapt-plan helper, and the final
inspect/adapt result document now all live in shared wrongsv code instead of
duplicate wrongcl parsing/extraction/config-assembly/response-shaping logic.

The Flutter shell now includes:

- a `Profiles` section for saving and reloading local configs
- a `Client Config` section for loading/exporting wrongcl config files
- a `wrongsv Import` section for capability inspection and form adaptation
- an `Activity` section for recent actions
- a `Desktop Integration` section with tray controls plus Linux autostart and system-proxy management

## Development And Validation Model

- Linux is the primary development and first-landing platform.
- Windows is the current primary validation and gap-closure platform.
- macOS and other supported hosts remain explicit follow-on verification
  targets.

This distinction matters for release claims: Linux-complete does not imply
Windows-complete or macOS-complete. Platform status should always be stated
explicitly in docs, handoff notes, and release summaries.

## Platform Verification

Verified in this environment:

- Linux: `cargo test`, focused shared `wrongsv` helper tests for `wrongcl_*`,
  `flutter analyze`, `flutter test`, `flutter build linux`
- Local convenience gate: `bash scripts/verify-local.sh linux`

Host-specific verification entry points:

- macOS host: `bash scripts/verify-macos-host.sh`
- macOS TUN smoke: `bash scripts/smoke-macos-tun.sh`
- Windows dependency prep: `powershell -ExecutionPolicy Bypass -File scripts/setup-windows-deps.ps1`
- Windows host: `powershell -ExecutionPolicy Bypass -File scripts/verify-windows-host.ps1`
- Android host/CI: `bash scripts/verify-android-host.sh`
- iOS host/CI: `bash scripts/verify-ios-host.sh`
- FreeBSD host: `sh scripts/verify-freebsd-headless.sh`

Release bundle entry points:

- Linux: `bash scripts/package-linux-release.sh`
- macOS host: `bash scripts/package-macos-release.sh`
- Windows host: `powershell -ExecutionPolicy Bypass -File scripts/package-windows-release.ps1`
- Android: `bash scripts/package-android-release.sh`
- iOS host: `bash scripts/package-ios-release.sh`
- Linux headless: `bash scripts/package-headless-linux-release.sh`
- macOS headless: `bash scripts/package-headless-macos-release.sh`
- Windows headless: `powershell -ExecutionPolicy Bypass -File scripts/package-headless-windows-release.ps1`
- FreeBSD headless: `sh scripts/package-freebsd-headless.sh`

Current limitations:

- Windows Rust checks from Linux require an actual MSVC-capable Windows host or
  an equivalent toolchain; `cargo check --target x86_64-pc-windows-msvc` is not
  usable here with plain GNU `cc`.
- macOS Rust checks from Linux require Apple-target toolchains/SDKs; Apple
  target `cargo check` is not usable here with the default Linux compiler.
- Some desktop integrations are still host-specific in practice. When a
  control is real on Linux but not yet implemented on Windows or macOS, the UI
  should remain truthful and docs should call out the platform scope directly.

## Windows Dependency Prep

Before Windows verification or packaging, place `wintun.dll` with:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-windows-deps.ps1
```

The script pins the currently verified Wintun package, copies `wintun.dll`
into the repo root plus `windows/runner`, and also refreshes the built release
bundle if it already exists.

Useful overrides:

- `WRONGCL_PROXY=http://127.0.0.1:7897` to download through a local proxy
- `WRONGCL_WINTUN_ZIP=E:\path\to\wintun.zip` to reuse a local archive
- `WRONGCL_WINTUN_URL=https://...` to override the archive source explicitly

## macOS Follow-On Prep

The repo now prewires a macOS TUN seam and a truthful smoke entrypoint, but it
does not claim a runnable macOS TUN implementation yet.

Useful entry points on a real macOS host:

- `bash scripts/verify-macos-host.sh`
- `WRONGCL_RUN_MACOS_TUN_SMOKE=1 bash scripts/verify-macos-host.sh`
- `bash scripts/smoke-macos-tun.sh`

## Build

Build the headless Rust client:

```bash
cargo build --manifest-path rust/Cargo.toml --bin wrongcl-headless
```

Install Flutter, then build the UI app:

```bash
sudo apt-get install -y libayatana-appindicator3-dev
flutter pub get
flutter build linux
```

The desktop targets compile `rust/Cargo.toml` and bundle `wrongcl_native`
through their host build files. Android and iOS now have corresponding native
build hooks in their Gradle/Xcode projects.

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

Start a local proxy listener:

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

Inspect a wrongsv server config:

```bash
cargo run --manifest-path rust/Cargo.toml --bin wrongcl-headless -- capabilities \
  --wrongsv-config ../wrongsv/configs/ws-tcp.toml
```

Adapt a wrongsv config into a wrongcl config/report:

```bash
cargo run --manifest-path rust/Cargo.toml --bin wrongcl-headless -- adapt \
  --wrongsv-config ../wrongsv/configs/httpupgrade.toml \
  --server-host 127.0.0.1
```

`serve` and `probe` can also consume a wrongsv config directly:

```bash
cargo run --manifest-path rust/Cargo.toml --bin wrongcl-headless -- probe \
  --wrongsv-config ../wrongsv/configs/ws-tcp.toml \
  --server-host 127.0.0.1 \
  --target-host example.com \
  --target-port 80
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
start the local proxy, then run a probe or point a local SOCKS5-capable or
HTTP CONNECT-capable client at `127.0.0.1:1080`.

## Desktop Smoke Checklist

- Linux, macOS, Windows: launch the app and confirm the tray icon appears.
- Linux, macOS, Windows: use the tray menu to show or hide the window.
- Linux, macOS, Windows: start the proxy from the window, then stop or refresh it from the tray menu.
- Linux: enable and disable autostart plus system proxy from the app UI.
