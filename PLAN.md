# wrongcl Cross-Platform Closure Plan

Last reviewed: 2026-06-22.

## Goal

Keep the milestone aligned with the actual collaboration model:

- Linux remains the main development and first-landing platform.
- Windows is the main verification and gap-closure platform for this cycle.
- macOS and other hosts stay in the follow-on alignment queue unless a Windows
  closure task naturally exposes a reusable abstraction.

This plan is no longer discussion-only. The repo has already moved through the
backend-first audit and into active Windows closure work.

## Current Baseline

- `0.1.1` already shipped the single-screen control surface, routing modes,
  Proxies / Connections / Requests / Logs / DNS views, split Settings views,
  and the Linux Rust TUN runtime.
- Linux remains the main development and first-landing environment.
- Windows remains the main verification and completion environment for desktop
  parity work.
- This cycle already closed several previously open items:
  - KCP UDP relay
  - Hysteria2 fragmented UDP handling
  - Hysteria2 Gecko / Salamander obfuscation
  - client-side missing-field completion flow
  - Windows system proxy integration
  - Windows TUN backend plus elevated smoke validation
  - Windows runtime, packaging, and release-bundle verification
- The main remaining host gap in the routed dataplane story is now macOS TUN,
  while Windows source app lookup and macOS source app lookup remain later
  observability work.

## Phase Rule For This Cycle

- Keep release language tied to both backend truth and host verification.
- Do not collapse "Linux implemented" into "all platforms complete."
- Treat Windows as the primary validation surface for Linux-led backend work in
  this cycle.
- Always distinguish:
  - core backend missing
  - platform adaptation missing
  - UI or docs ahead of backend reality

## Stage Structure

### Stage 0: Core Backend Audit

Status: completed on 2026-06-22.

Purpose: produce a trusted map of what is actually incomplete before choosing
implementation order.

- Build a backlog from repo truth, not from memory or UI shape.
- Classify open items into:
  - `core-backend-gap`
  - `platform-gap`
  - `ui-docs-mismatch`
- Record whether each issue is:
  - Linux working
  - Linux partial
  - Windows blocked
  - macOS blocked
- Output captured in:
  - `BACKEND_AUDIT.md`
  - `EXECUTION_QUEUE.md`
  - `WINDOWS_ALIGNMENT.md`

### Stage 1: Linux-First Backend Completion

Status: substantially completed for the current queue on 2026-06-22.

Purpose: complete the highest-priority shared backend gaps on the main
development platform before expanding platform closure claims.

- Prioritize shared logic and dataplane work that affects all hosts.
- Completed in this stage:
  - KCP UDP relay
  - Hysteria2 fragmented UDP response handling
  - Hysteria2 Gecko / Salamander obfs
  - missing-field completion and draft import flow
- Remaining narrowed work:
  - WireGuard product-truth and routed-tunnel surface still needs continued
    tightening
  - source app lookup is still non-parity on Windows and macOS
- Linux remains the first place these backend features are implemented and
  verified.

### Stage 2: Windows Verification And Completion

Status: active and mostly green on 2026-06-22.

Purpose: validate Linux-led backend work on Windows and close Windows-specific
desktop integration gaps.

- Focus Windows work on:
  - TUN backend and truthful control state
  - system proxy integration
  - runtime smoke validation
  - packaging and release confidence
  - regression checks for FFI-backed pages and controls
- Completed in this stage:
  - Windows system proxy enable / disable path
  - Windows TUN runtime using Wintun
  - elevated TUN smoke validation
  - `verify-windows-host.ps1` end-to-end pass
  - `package-windows-release.ps1` end-to-end pass
  - `setup-windows-deps.ps1` for repeatable Wintun preparation
- Every Windows issue found here must stay categorized as either a platform gap
  or a shared defect.

### Stage 3: macOS And Other Host Alignment

Purpose: expand the same verified behavior model to other supported desktop
hosts without pretending completion early.

- Keep explicit checklists and verification hooks for macOS.
- Reuse abstractions created during Linux and Windows work.
- Do not mark parity complete until host-specific validation exists.
- Current likely lead items:
  - macOS TUN planning or implementation
  - source app lookup parity where still worthwhile

### Stage 4: Release Closure

Purpose: convert backend-complete and host-validated work into a precise
release claim.

- Release notes must state platform scope explicitly.
- "Done" must always mean both:
  - backend behavior is implemented
  - target hosts have been validated

## Public Interfaces / Boundaries

- Keep platform-facing state objects uniform across hosts:
  - `TunStatus`
  - `SystemProxyStatus`
  - `ControlAvailability`
- Keep host-specific branching inside Rust or Dart platform adapters rather
  than spreading it through `main_view.dart` and subviews.
- Any new field added for platform work must be additive and documented with
  host scope.

## Deliverables For This Milestone

- Maintain planning and handoff docs around Linux-first development plus
  Windows-first verification.
- Keep Windows dependency preparation reproducible through
  `scripts/setup-windows-deps.ps1`.

## Remaining Follow-Ons

- WireGuard product-truth and routed-tunnel scope still need a tighter final
  statement before release language is fully settled.
- Source app lookup remains Linux-only and should be treated as later Windows /
  macOS observability work, not as part of the current closure claim.
- macOS host work is prewired but not yet validated:
  - TUN still needs real utun-backed implementation work on a macOS machine
  - desktop packaging / runtime verification still need a native macOS pass

## Completed In This Cycle

- Shared backend closure:
  - KCP UDP relay
  - Hysteria2 fragmented UDP reassembly
  - Hysteria2 Gecko / Salamander obfuscation
  - first missing-field completion flow for client-only required inputs
- Windows closure:
  - system proxy integration
  - TUN runtime with Wintun
  - elevated TUN smoke validation
  - host verification and packaging scripts passing end to end
  - dependency preparation script for Wintun
  - hidden PowerShell route operations so TUN enable no longer flashes a
    script window during normal use
- macOS prewiring:
  - truthful planned-status seams for TUN and system proxy
  - headless `tun-status` entrypoint plus follow-on smoke hook

## Release Gate

Before this milestone is called complete:

- Stage 0 backlog exists and is trusted.
- Linux verification remains green for backend-first work.
- Windows verification remains green through:
  - `powershell -ExecutionPolicy Bypass -File scripts/setup-windows-deps.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/verify-windows-host.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/package-windows-release.ps1`
- Packaging entry points remain present for Linux, macOS, and Windows.
- Docs and release language identify both backend status and host scope.

## Non-Goals

- Replacing Linux as the main development environment.
- Treating Linux success as proof of all-platform completion.
- Treating Windows closure as a reason to skip remaining macOS or later parity
  work.
