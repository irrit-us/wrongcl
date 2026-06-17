# wrongcl Development Plan

Last reviewed: 2026-06-17.

## Goal

Make `wrongcl` a truthful, production-grade client for `wrongsv`, not just a
collection of protocol probes.

Definition of done:

- headless Rust and Flutter desktop use the same capability model
- any stack marked "supported" works in local proxy mode, not only in `probe`
- payload capability (`tcp`, `udp`, `ip`) is represented honestly
- high-value `wrongsv` stacks import cleanly from server config and have
  end-to-end tests
- Linux, macOS, and Windows desktop builds are reproducible in CI

## Constraints

- Keep the Rust headless client first-class. Flutter is a frontend, not the
  only client.
- Borrow wire formats and helpers from `wrongsv` crates when possible. Do not
  re-derive transport framing unless there is no reusable source.
- Preserve the `proxy -> transport -> outer security` separation from
  `wrongsv/docs/design_constraint.md`.
- Do not claim VMess compatibility beyond `wrongcl <-> wrongsv`.
- GFW-only remote failures are tracking signals, not local correctness
  blockers.
- Do not mark a stack "supported" based on direct `probe` coverage alone.

## Audit Snapshot

What works today:

- Rust core implements `VLESS`, `Trojan`, remote `Mixed`, and
  `Shadowsocks`.
- Implemented transports / wrappers in the Rust core:
  `raw`, `WebSocket`, `HTTPUpgrade`, `XHTTP`, `gRPC`, `TLS`, `REALITY`,
  `AnyTLS`, and `Vision`.
- The local proxy relay now uses split read/write halves instead of depending
  only on whole-tunnel cloning. Proxy-mode integration coverage now exists for
  raw VLESS, VLESS+TLS, AnyTLS, REALITY, Vision, HTTPUpgrade, WebSocket,
  WebSocket+TLS, raw/TLS XHTTP, raw/TLS gRPC, Trojan/TLS, and Shadowsocks TCP.
- Local SOCKS5 `UDP ASSOCIATE` is now verified for the currently implemented
  proxy families: raw VLESS, VLESS+TLS, AnyTLS, REALITY, raw/TLS WebSocket,
  raw/TLS HTTPUpgrade, raw/TLS XHTTP, raw/TLS gRPC, Trojan/TLS, and
  Shadowsocks classic / AEAD-2022 UDP.
- `cargo test --manifest-path wrongcl/rust/Cargo.toml` passed on 2026-06-17.

What is still incomplete:

- The local Flutter SDK in `.tools/flutter` is usable here. The current
  Flutter app now passes `flutter analyze`, `flutter test`, and
  `flutter build linux`.
- The adapter still uses a local parser instead of shared `wrongsv`
  endpoint diagnostics for some adaptation decisions, but the public
  inspect/adapt path now takes active-profile, payload-network, and
  base-carrier truth from a shared `wrongsv` resolver, and the remaining loose
  import schema plus the profile-specific neutral import spec for currently
  implemented wrongcl stacks now also live in `wrongsv` instead of `wrongcl`.
  The wrongcl-specific support-state and missing-field overlay for those
  implemented stacks now also comes from shared `wrongsv` helpers, and the
  strict-vs-draft adapt-plan choice now also comes from a shared helper. The
  neutral wrongcl config document for the currently implemented stacks is now
  also produced in shared `wrongsv` code and deserialized by wrongcl instead of
  being reassembled layer-by-layer in the client. The final inspect/adapt
  report/result document is now also shared instead of being re-shaped in
  wrongcl. A narrow fallback still exists for partial configs that are missing
  client-only fields. It still reports `payload_networks`, `base_carriers`,
  `supported` / `partial` / `unsupported`, and structured `missing_fields`.
- The local runtime now has a real SOCKS5 `UDP ASSOCIATE` path plus local HTTP
  `CONNECT` and absolute-form HTTP forwarding support for the currently
  implemented proxy families. There is still no TUN/system-proxy mode.
- Flutter is Linux-only in-tree, but it now has a saved-profile workflow plus
  a `wrongsv` inspect/adapt section wired to the native FFI surface, direct
  wrongcl config file import/export, an activity log, and Linux desktop
  integration for autostart and system proxy management. The saved-profile
  store now writes a versioned on-disk format while still loading legacy array
  payloads, and profile deletion now has a confirmation step instead of
  immediate removal. Windows/macOS project scaffolding and initial native-
  loading/build hooks now exist, but only the Linux desktop build is verified
  here.
  Host-specific verification scripts now exist for macOS and Windows, and
  host packaging scripts plus CI artifact upload steps now exist for all three
  desktop platforms, but non-Linux build execution is still pending deeper
  verification.

## Immediate Conclusions

- Do not add more long-tail protocols first. The client already has more
  protocol code than product/runtime truthfulness.
- The shortest path to a mature client is:
  1. truthful capability reporting
  2. stream relay that works for every claimed TCP stack
  3. UDP runtime and local proxy completeness
  4. config import, persistence, and desktop product features
  5. remaining protocol backlog

## Main Gaps

### 1. Capability truthfulness

Current status:

- `wrongcl` now reports `payload_networks`, `base_carriers`, active
  `supported` / `partial` / `unsupported` state, and structured
  `missing_fields` for client-only inputs such as REALITY `public-key`.
- The public `inspect_wrongsv_config` / `adapt_wrongsv_config` path now uses a
  shared `wrongsv` resolver for active profile detection and resolved
  payload/base-carrier data, and a fixture audit checks the real
  `wrongsv/configs/` corpus for drift in those fields.
- The loose import schema used for partial and draft adaptation now lives in
  shared `wrongsv` code instead of duplicated `wrongcl` structs.
- The profile-specific import spec for the currently implemented wrongcl stacks
  is now produced in shared `wrongsv` code, and wrongcl maps that neutral spec
  into its local endpoint types instead of re-extracting each transport/security
  shape itself.
- The wrongcl-specific support-state and missing-field overlay for the active
  stack is now also produced from shared `wrongsv` helpers instead of client-
  local logic.
- The strict-vs-draft adapt-plan decision for the active stack is now also
  produced from shared `wrongsv` helpers instead of client-local orchestration.
- The neutral wrongcl config document for the active stack is now also produced
  from shared `wrongsv` helpers, and wrongcl deserializes that shared document
  instead of hand-assembling proxy/transport/outer-security layers locally.
- The final inspect/adapt report/result document for the active stack is now
  also produced from shared `wrongsv` types instead of client-local response
  shaping.
- UDP-bearing configs for the currently implemented families can now report
  `supported` when required client-only inputs are present.

Remaining outcome:

- replace the remaining local glue around the shared wrongsv surface only where
  it still materially reduces drift, instead of only sharing the resolved
  profile/payload/base-carrier path, import schema, neutral import spec,
  neutral config document, active support-state overlay, strict-vs-draft plan
  selection, and final report/result document

### 2. Probe-mode vs proxy-mode mismatch

Current status:

- The local relay in `wrongcl/rust/src/proxy.rs` now splits tunnels into
  dedicated read/write halves instead of requiring whole-tunnel cloning.
- `Vision`, `XHTTP`, `gRPC`, `REALITY`, `AnyTLS`, `Shadowsocks`, TLS, raw
  WebSocket, WebSocket+TLS, HTTPUpgrade, and Trojan/TLS now pass
  proxy-mode tests for TCP traffic.

Remaining outcome:

- Any stack marked `supported` must pass through `wrongcl-headless serve`, not
  only through `wrongcl-headless probe`. The remaining runtime gap is no longer
  the implemented TCP/UDP transport matrix; it is the unimplemented protocol
  families and product surface above the Rust core.

### 3. Missing UDP runtime

Current status:

- The Rust runtime now models both streams and datagrams for the currently
  implemented families, and local SOCKS5 `UDP ASSOCIATE` has verified coverage
  across the supported transport matrix.

Remaining outcome:

- extend that same level of UDP/runtime support when new protocol families such
  as QUIC/WebTransport/Hysteria2/TUIC/KCP are implemented

### 4. Missing client product surface

Current status:

- The Flutter app now has a first saved-profile workflow and a `wrongsv`
  inspect/adapt section backed by native FFI exports for capability inspection
  and config adaptation, plus direct wrongcl config file import/export,
  activity history, Linux autostart management, and a versioned saved-profile
  store with legacy compatibility and safe-delete confirmation.
- The native surface now exposes version / start / stop / status / probe /
  stack-summary plus wrongsv capability inspection / adaptation, client-config
  loading, and TOML export.
- Flutter-side execution is now verified in this shell through
  `flutter analyze`, `flutter test`, and `flutter build linux`.

Remaining outcome:

- The desktop client must be able to import a `wrongsv` config, explain what is
  supported, prompt for any missing client-only fields, persist and manage the
  resulting profiles cleanly, and grow into broader desktop/system integration
  and verified non-Linux builds.

## Roadmap

### Phase 0. Use `wrongsv` endpoint diagnostics as the source of truth

Scope:

- eliminate capability drift between `wrongsv` and `wrongcl`

Tasks:

- Extract or share the resolved endpoint model from `wrongsv` instead of
  maintaining a separate coarse parser in `wrongcl`.
- Minimum shared data:
  `protocol`, `payload_networks`, `transport`, `outer_security`,
  `base_carriers`, active components, and a stack summary.
- Add a `missing_fields` concept for client-only inputs such as REALITY
  `public_key`.
- Replace boolean `implemented` reporting with:
  `supported`, `partial`, `unsupported`, and a precise reason string.
- Add a fixture-driven audit that walks every config in `wrongsv/configs/`.

Verify:

- every config in `wrongsv/configs/` resolves deterministically
- UDP-bearing configs are never reported as fully supported while `wrongcl`
  remains TCP-only
- REALITY configs without `public_key` surface a structured missing-field state
  rather than a generic parse failure

### Phase 1. Make every claimed TCP stack work through the local proxy

Scope:

- remove the current `probe` vs `serve` split

Tasks:

- Replace the `Tunnel::try_clone_box()` relay contract with a design that does
  not require cloning the upstream connection.
- Acceptable directions:
  - split read/write halves at the tunnel boundary
  - run a tunnel-owner worker that pumps both directions
  - introduce a `StreamConnection` type with explicit reader/writer ownership
- Reclassify support until proxy-mode tests exist.
- Add proxy-mode integration coverage for:
  - raw VLESS
  - TLS VLESS
  - AnyTLS VLESS
  - REALITY VLESS
  - Vision VLESS
  - WebSocket (+ TLS)
  - HTTPUpgrade (+ TLS)
  - XHTTP
  - gRPC
  - Trojan over TLS
  - Shadowsocks classic and AEAD-2022

Verify:

- `wrongcl-headless serve` passes end-to-end echo tests for every TCP stack
  listed as `supported`
- no supported stack still depends on `probe`-only validation

### Phase 2. Add UDP-capable runtime and local proxy completeness

Scope:

- make `wrongcl` a real client for UDP-capable `wrongsv` deployments

Tasks:

- Introduce a datagram-capable runtime API alongside stream connections.
- Implement local SOCKS5 `UDP ASSOCIATE`.
- Add UDP relay coverage for the server stacks that already support it:
  - VLESS raw / TLS / REALITY / AnyTLS where UDP is enabled
  - VLESS WebSocket / HTTPUpgrade / XHTTP / gRPC where UDP is enabled
  - Trojan UDP
  - Shadowsocks UDP and AEAD-2022 UDP
- After SOCKS5 UDP is stable, consider a local mixed inbound
  (SOCKS5 + HTTP CONNECT) only if desktop/system integration needs it.

Verify:

- local SOCKS5 `UDP ASSOCIATE` round-trips against UDP-capable `wrongsv`
  configs
- the capability layer upgrades applicable configs from `partial` to
  `supported`

### Phase 3. Expose adaptation and diagnostics through FFI and Flutter

Scope:

- turn the existing headless tooling into a usable desktop workflow

Tasks:

- Add FFI exports for:
  - capability inspection
  - `wrongsv` config adaptation
  - config validation
  - structured error codes / missing fields
  - streaming runtime events or log snapshots
- Add Dart models for the shared capability schema.
- Add profile persistence in Flutter.
- Add import flows for:
  - existing `wrongcl` JSON/TOML
  - `wrongsv` server config + adaptation result
- Add UI for missing-field prompts, especially REALITY `public_key` and similar
  future client-only inputs.

Verify:

- widget tests for import, adapt, save, load, start, and stop
- Rust tests for FFI JSON contracts

### Phase 4. Add desktop product features only after runtime truth is fixed

Scope:

- make the client practical to run daily

Tasks:

- system proxy integration
- tray / menu bar controls
- start-on-login
- connection health / logs / last error view
- profile duplication and safe delete
- release packaging for Linux first, then macOS and Windows

Verify:

- manual smoke checklist per platform
- stable upgrade path between saved profile versions

### Phase 5. Finish the protocol backlog in dependency order

Only start this phase after Phases 0-4 are in place.

Priority order:

1. `ShadowTLS`
2. `Hysteria2`
3. `TUIC`
4. `VMess` (`wrongsv`-only)
5. `QUIC`
6. `KCP`
7. `WebTransport`
8. `Meek`
9. `Google Docs Viewer`
10. `WireGuard`
11. `Naive`

Notes:

- `ShadowTLS` should reuse shared code from `wrongsv`; do not build on the
  evaluator stub if its wire protocol diverges.
- `Hysteria2`, `TUIC`, `QUIC`, and `WebTransport` depend on the UDP/runtime work
  from Phase 2 plus additional async transport work.
- `VMess` must stay explicitly labeled as `wrongsv`-only unless proven
  otherwise.
- `WireGuard` and `Naive` are product-expansion items, not blockers for a
  mature proxy client v1.

Verify:

- every new protocol lands with:
  - adapter coverage
  - `probe` coverage
  - local proxy coverage
  - UI wiring
  - docs update

## First Four Concrete PRs

If the work starts now, this is the recommended sequence:

1. Replace the stale capability report with a resolved-endpoint-based report
   that can represent `supported` / `partial` / `unsupported`.
2. Refactor the stream relay so non-cloneable tunnels work through
   `wrongcl-headless serve`.
3. Add local SOCKS5 `UDP ASSOCIATE` and the datagram runtime.
4. Expose adapt/capabilities through FFI and build a persisted profile manager
   in Flutter.

## Release Gate

Before calling `wrongcl` "well-developed", require all of the following:

- `cargo fmt --all -- --check`
- `cargo clippy --manifest-path wrongcl/rust/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path wrongcl/rust/Cargo.toml`
- focused shared `wrongsv` helper tests covering the `wrongcl_*` unit surface
- `flutter analyze`
- `flutter test`
- `flutter build linux`
- equivalent desktop build verification for macOS and Windows once added
- docs updated so `wrongcl/README.md` reflects the actual supported surface
