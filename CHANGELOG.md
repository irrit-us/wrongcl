# Changelog

## 0.1.2 - 2026-06-25

- Replaced the native window chrome on Linux, macOS, and Windows with a
  themed in-app title bar that follows the app palette and adds a pin button
  for "keep on top".
- Shipped a Windows MSIX installer that declares the VCLibs framework
  dependency, fixing first-run `0xc000007b "application was unable to start"`
  errors on machines without the VC++ runtime.
- Reworked the dashboard into separate Inspect and Settings blocks, added a
  time-weighted Avg(1min) traffic stat, and slimmed chip counts so the label
  leads the trailing number.
- Centralized the color palette with runtime theme variants, and added an
  RTL-aware control strip plus a chip-icon-side toggle.
- Shrunk the default window to fit on smaller screens and bumped the base
  font size for readability.
- Hardened FFI handle safety and the on-disk profile store.
- Stabilized macOS CI by widening the gdocsviewer and Meek UDP integration
  test windows.
- Added LICENSE (MIT), Code of Conduct, Contributing, Security policy, and
  GitHub issue templates.

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
