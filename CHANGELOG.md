# Changelog

## 0.1.1 - 2026-06-22

- Reworked the desktop app into the planned single-screen control surface with
  real Proxies, Connections, Requests, Logs, DNS, and split Settings views.
- Added rule/mode storage, live routing controls, and persistence for theme,
  language, autostart, proxy listener, and DNS settings.
- Replaced the temporary Go TUN bridge with a pure Rust Linux TUN runtime and
  kept the routed dataplane covered by namespace integration tests.
- Hardened local and CI verification so strict Rust/Flutter checks stay green
  across Linux, macOS, Windows, Android, and iOS hosts.

## 0.1.0 - 2026-06-19

- Added truthful wrongsv-backed capability inspection and adaptation in both the
  Flutter UI and headless client.
- Verified desktop UI and packaging on Linux, macOS, and Windows in CI.
- Added Android and iOS app project scaffolding plus native Rust FFI build
  wiring for mobile builds.
- Added release assets for both the Flutter UI app bundles and the headless
  `wrongcl-headless` terminal client.
- Moved the WireGuard client runtime into the main Rust crate.
- Fixed server-side TLS relay handling in wrongsv so Naive client support is
  stable on macOS runners.
- Added FreeBSD headless packaging and verification entry points as host-side
  support scripts.
