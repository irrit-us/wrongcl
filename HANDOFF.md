# wrongcl Cross-Platform Handoff

Last updated: 2026-06-22.

## Current Objective

Carry forward the Linux-first / Windows-verification milestone without losing
the distinction between shared backend truth, Windows closure, and later host
alignment. This repo is no longer in pure planning mode; the current handoff
starts from a partially executed queue with Windows closure largely validated.

## Collaboration Reality

- Primary developer environment: Linux
- Primary verification and gap-closure environment in this cycle: Windows
- Secondary alignment targets: macOS and other supported hosts

When describing progress, always distinguish:

- implemented on Linux
- verified on Windows
- pending on macOS or other platforms

Do not compress these states into one generic completion claim.

## Current State

- The single-screen control surface and its subviews are already in-tree.
- Linux is still the most complete host for runtime-backed functionality.
- Windows verification has already advanced from planning into execution.
- The repo now includes:
  - Windows system proxy integration
  - Windows TUN runtime via Wintun
  - `scripts/smoke-windows-tun.ps1` for elevated host smoke validation
  - `scripts/setup-windows-deps.ps1` for repeatable Wintun dependency setup
  - green Windows host verify and packaging runs on 2026-06-22
- The remaining open work is narrower than before and should be judged against
  the backlog docs instead of restarting discovery from scratch.

## Current Priorities

- Keep a clean issue split between:
  - core backend gaps
  - platform adaptation gaps
  - docs or acceptance mismatches
- Keep Linux verification green while continuing host closure.
- Treat the next execution step as one of:
  - remaining shared backend truthfulness work
  - macOS TUN planning / closure
  - source app lookup parity
- Avoid reopening already verified Windows work unless a regression appears.

## Important Constraints

- Do not change `pubspec.yaml` Dart SDK constraint.
- Rust builds require `PROTOC`:
  `C:\Users\Charon\AppData\Local\Microsoft\WinGet\Packages\Google.Protobuf_Microsoft.Winget.Source_8wekyb3d8bbwe\bin\protoc.exe`
- Flutter engine/download commands may need:
  `HTTP_PROXY=http://127.0.0.1:7897`
  `HTTPS_PROXY=http://127.0.0.1:7897`
- Windows dependency prep can use:
  `WRONGCL_PROXY=http://127.0.0.1:7897`
  `WRONGCL_WINTUN_ZIP=E:\path\to\wintun.zip`
- Do not fake support for TUN or system proxy on hosts where the backend is not
  implemented.
- Do not describe a feature as "complete" without host-specific validation.

## Verification Commands

Run from `E:\irrit-us developer group\wrongcl`.

Linux baseline:

```bash
cargo test --manifest-path rust/Cargo.toml
flutter analyze
flutter test
bash scripts/verify-local.sh linux
```

Windows closure path:

```powershell
flutter analyze
flutter test
$env:PROTOC='C:\Users\Charon\AppData\Local\Microsoft\WinGet\Packages\Google.Protobuf_Microsoft.Winget.Source_8wekyb3d8bbwe\bin\protoc.exe'
$env:HTTP_PROXY='http://127.0.0.1:7897'
$env:HTTPS_PROXY='http://127.0.0.1:7897'
$env:WRONGCL_PROXY='http://127.0.0.1:7897'
powershell -ExecutionPolicy Bypass -File scripts/setup-windows-deps.ps1
powershell -ExecutionPolicy Bypass -File scripts/verify-windows-host.ps1
powershell -ExecutionPolicy Bypass -File scripts/package-windows-release.ps1
```

Elevated Windows TUN smoke:

```powershell
Start-Process powershell -Verb RunAs -ArgumentList '-ExecutionPolicy Bypass -File scripts/smoke-windows-tun.ps1 -OutputPath .tmp/windows-tun-smoke-admin.json'
```

Other host hooks:

```bash
bash scripts/verify-macos-host.sh
bash scripts/package-macos-release.sh
```

## Manual Validation Focus

- When validating, first decide whether a failure is shared backend, platform
  adaptation, or docs mismatch.
- On Windows, verify whether Linux-complete features are actually runnable, not
  merely rendered.
- Confirm TUN and system proxy controls either work for real or show accurate
  disabled reasons.
- Record any newly discovered gaps directly in `PLAN.md` / `HANDOFF.md` unless
  a future milestone intentionally reintroduces separate backlog files.

## Supporting Docs

- Planning baseline: `PLAN.md`
- Current release / host scope summary: `README.md`

## Next Suggested Work

- Keep Linux verification green while preserving current Windows pass state.
- Use `BACKEND_AUDIT.md` plus `EXECUTION_QUEUE.md` to choose the next real gap
  instead of restarting platform triage.
- Prefer one of:
  - macOS TUN planning / implementation
  - source app lookup parity
  - remaining WireGuard product-truth closure
