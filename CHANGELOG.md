# Changelog

## 0.1.0 - 2026-06-19

- Added truthful wrongsv-backed capability inspection and adaptation in both the
  Flutter UI and headless client.
- Verified desktop UI and packaging on Linux, macOS, and Windows in CI.
- Added Android and iOS app project scaffolding plus native Rust FFI build
  wiring for mobile builds.
- Added release assets for both the Flutter UI app bundles and the headless
  `wrongcl-headless` terminal client.
- Added WireGuard helper packaging for desktop/headless bundles.
- Fixed server-side TLS relay handling in wrongsv so Naive client support is
  stable on macOS runners.
- Added FreeBSD headless packaging and verification entry points as host-side
  support scripts.
