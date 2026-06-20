# wrongcl UI Reorganization Plan

Last reviewed: 2026-06-20.

## Goal

Reorganize `wrongcl` into a control-first desktop UI. The main page becomes a
dashboard with direct runtime controls and large buttons that open full-page
sub interfaces. The control surface should cover the requested FlClash-style
essentials: start/stop, system proxy switch, TUN switch, mode selection
(`Rule`, `Global`, `Direct`, `Script`), and dense status cards.

## Assumptions

- "system agents" means system proxy.
- "FlLaser" refers to the FlClash-like client control surface already present
  in this workspace.
- Sub interfaces are in-app full-page panels inside one window, not extra
  native windows.
- "customizable scripts" means user-managed script entries or script presets
  that can be selected from the main page and edited in a dedicated sub
  interface.
- Controls must be truthful. Unsupported features may be visible, but they
  must not appear enabled or working when the backend/runtime does not support
  them.

## Current State

- `wrongcl/lib/app.dart` is a single, long, scrollable screen driven by one
  `StatefulWidget`.
- `wrongcl` already has Linux system proxy support, profile persistence,
  `wrongsv` inspect/adapt, local proxy start/stop, health/probe, and desktop
  shell integration.
- `wrongcl` does not yet expose TUN runtime control, proxy-mode control, or
  script control in Dart/FFI.
- Because of that, this work is not only layout work. It needs a small
  control model plus any missing backend/API support.

## Definition Of Done

- The main dashboard shows:
  - running state, stack summary, and connection stats
  - system proxy switch
  - TUN switch
  - mode selector: `Rule`, `Global`, `Direct`, `Script`
  - script selector or quick script action when `Script` mode is active
  - buttons/cards that open full-page sub interfaces
- Each sub interface covers the main content area and includes a close button
  in the upper-left corner.
- The interface feels full on desktop: multiple cards above the fold, compact
  spacing, and no large empty regions.
- Current workflows remain available: profiles, `wrongsv` import/adapt,
  endpoint editing, runtime controls, probe, and health/activity.
- Unsupported controls are disabled with an explicit reason instead of
  pretending to work.
- Widget tests cover dashboard navigation, close-button behavior, and control
  truthfulness.

## Proposed Screen Model

- `Dashboard`
  - persistent runtime controls
  - status cards
  - entry buttons to sub interfaces
- `Profiles`
  - saved profiles, save/load/duplicate/delete
- `Import`
  - `wrongsv` inspect/adapt and missing-field prompts
- `Editor`
  - endpoint, local proxy, and protocol-specific fields
- `Runtime`
  - start/stop, refresh, probe, stats, raw result/error
- `Settings`
  - desktop integration, system proxy details, TUN details, script management

## Plan

### Phase 1. Create a truthful control model

Scope:

- Separate the dashboard control state from the large profile form.

Tasks:

- Introduce a small Dart-side model for:
  - `systemProxy`
  - `tun`
  - `agentMode`
  - `selectedScript`
  - support and disabled-reason text for each control
- Add a single state holder/controller for `ClientHome` instead of expanding
  the current single widget further.
- Extend the native/Dart surface only where needed so the UI can read and
  write control state without inferring it from unrelated fields.

Verify:

- The UI can render a single control snapshot from test fakes.
- Disabled controls always show a reason.
- No dashboard control depends on parsing the endpoint editor form.

### Phase 2. Add the missing backend/API surface

Scope:

- Provide real semantics for the requested main-page controls.

Tasks:

- Keep the existing system proxy support, but normalize its state into the new
  control model.
- Add explicit TUN capability reporting first.
- Add control actions/status for:
  - `setSystemProxyEnabled`
  - `setTunEnabled`
  - `setAgentMode`
  - `listScripts`
  - `selectScript`
- If TUN, mode, or script semantics cannot be implemented in the same
  milestone, keep the control visible but disabled with a truthful support
  reason. Do not ship fake toggles.

Verify:

- The fake/test client and the real client expose the same control contract.
- Unsupported TUN platforms report a stable disabled state instead of a UI
  guess.
- Mode changes round-trip through the client surface without touching endpoint
  form fields.

### Phase 3. Replace the one-page layout with a dashboard shell

Scope:

- Turn `ClientHome` into a shell with a dashboard and full-page sub interfaces.

Tasks:

- Split the screen into:
  - persistent app shell
  - dashboard view
  - active sub interface view
- Default to the dashboard.
- Add large buttons/cards for `Profiles`, `Import`, `Editor`, `Runtime`, and
  `Settings`.
- When a sub interface opens, it replaces the main content area and shows a
  close button in the upper-left corner.
- Keep form state alive between view switches.
- Prefer a small local routing enum. Do not add a heavy navigation framework.

Verify:

- Each dashboard button opens the correct view.
- The close button returns to the dashboard and preserves unsaved edits.
- Widget tests cover every open/close path.

### Phase 4. Build the main dashboard

Scope:

- Put high-frequency controls and status above the fold.

Tasks:

- Add a top control strip with:
  - start/stop
  - system proxy
  - TUN
  - mode selector
  - script selector
- Add status cards for:
  - health
  - connection manager
  - recent activity
  - import/support state
- Use denser card grids on desktop and wrapped stacks on narrower widths.
- Keep key controls above the fold at common desktop sizes.

Verify:

- At `1440x900`, the control strip and at least three summary cards are
  visible without scrolling.
- At `768x1024` and `390x844`, there is no overflow and controls remain
  tappable.
- Busy state disables only the actions that truly cannot run.

### Phase 5. Extract focused sub interfaces

Scope:

- Move existing sections out of `wrongcl/lib/app.dart` into dedicated widgets
  and files.

Tasks:

- Extract dedicated views for:
  - dashboard
  - profiles
  - import
  - editor
  - runtime
  - settings
- Keep the current profile store, `wrongsv` import/adapt, probe, and stats
  logic. Do not rewrite working backend logic.
- Only extract shared helpers where they are already duplicated.

Verify:

- `app.dart` becomes a shell instead of a large single-screen page.
- Existing profile/import/probe workflows still pass widget tests.
- New widget tests cover at least one happy path per sub interface.

### Phase 6. Add script management

Scope:

- Support the requested customizable-script flow without putting script editing
  directly on the dashboard.

Tasks:

- Define the minimum viable script model:
  - `id`
  - `name`
  - `source` or `reference`
  - enabled/selected state
- Add a sub interface for listing, selecting, creating, editing, and deleting
  scripts.
- Show only a selector or quick action on the dashboard. Editing belongs in
  the script/settings sub interface.
- Keep script failures local to the script area. They must not break the main
  page.

Verify:

- Scripts can be created, selected, persisted, and reloaded from tests/fakes.
- `Script` mode on the dashboard reflects the selected script or a missing
  script warning.
- Invalid or missing scripts do not crash the main page.

## File Targets

- `wrongcl/lib/app.dart`
- new UI files under `wrongcl/lib/` such as:
  - `home_shell.dart`
  - `dashboard_view.dart`
  - `control_state.dart`
  - `subviews/...`
- `wrongcl/lib/wrongcl_client.dart`
- `wrongcl/lib/system_proxy_manager.dart`
- native/FFI files only where required for truthful TUN, mode, or script state
- `wrongcl/test/widget_test.dart`
- new focused widget tests for dashboard navigation and control state

## Implementation Order

1. Control model and fake/test surface
2. Dashboard shell and full-page sub interface routing
3. Main dashboard controls and dense layout
4. Extraction of existing sections into sub interfaces
5. Backend/API completion for TUN, mode, and script semantics
6. Final polish and regression tests

## Non-Goals

- no new protocol support
- no broad state-management rewrite
- no major theme redesign unrelated to the structural UI change

## Release Gate

Before this work is called complete:

- `flutter analyze`
- `flutter test`
- `flutter build linux`
- manual desktop check that every dashboard button opens a full-page sub
  interface and every sub interface closes back to the dashboard through the
  upper-left close button
