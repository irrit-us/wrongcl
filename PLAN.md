# wrongcl UI Reorganization Plan

Last reviewed: 2026-06-21.

## Goal

Reorganize `wrongcl` into a control-first desktop UI without rewriting the
working client logic. The app should avoid two bad outcomes:

- a generic persistent-navigation shell that dictates the whole product
- an endlessly growing single page that collapses all workflows into one long
  scroll

The target shape is a **primary control surface + secondary panels + heavy work
modes** model. The main surface stays focused on runtime control and runtime
awareness, while lower-frequency or content-heavy workflows leave the default
surface only when they materially reduce clutter.

## Assumptions

- "system agents" means system proxy.
- "FlLaser" refers to the FlClash-like client control surface already present
  in this workspace.
- Controls must be truthful. Unsupported features may be visible, but they
  must not appear enabled or working when the backend/runtime does not support
  them.
- Originality matters. Navigation, if present, must support the product rather
  than turn it into a standard sidebar-driven admin shell.
- "customizable scripts" still means user-managed script entries or presets,
  but no fake runtime script control should appear before real backend support
  exists.

## Current State

- `wrongcl/lib/app.dart` used to be a single, long, scrollable screen driven
  by one `StatefulWidget`.
- The current refactor already extracted the UI into a controller plus views,
  but the product logic still leans on:
  - dashboard-first entry cards
  - full-screen workspace switching
  - returning to dashboard to reach a different tool
- `wrongcl` already has:
  - local proxy start/stop/status
  - stack summary and config validation
  - `wrongsv` inspect/adapt
  - profile persistence
  - health/probe tracking
  - autostart management
  - Linux GNOME system proxy integration
  - tray/window desktop shell integration
- `wrongcl` still does not expose real runtime contracts for:
  - TUN capability/state
  - agent mode selection
  - script listing/selection/management

## Definition Of Done

- The app presents a strong long-lived main control surface by default.
- The main surface shows:
  - running state, stack summary, and connection stats
  - truthful global controls such as system proxy
  - visible disabled TUN/mode/script controls with explicit reasons
  - health, activity, and support summaries
- Low-frequency management flows do not permanently occupy the main surface.
- `Profiles`, `Import`, and `Settings` behave like secondary workflow surfaces
  rather than equal-weight launcher destinations.
- `Editor` and deep runtime diagnostics behave like heavier focused work modes
  that prevent the primary surface from turning into a permanently expanded
  long-form page.
- Existing workflows remain available and still use the current backend paths.
- Widget tests cover:
  - preserved form state
  - truthful controls
  - correct interaction-tier behavior

## Differences From Initial Plan

The initial June 20 plan assumed a more conventional dashboard shell:

- dashboard launcher cards were the main way to reach other areas
- every non-dashboard workflow was treated as a full-page sub interface
- each sub interface returned through a close button
- TUN, mode, and script controls were candidates for backend/API expansion in
  the same broad milestone
- script management was included as a planned feature area

The current plan intentionally changes that direction:

- the dashboard is no longer a launcher hub; it is the primary control surface
- `Profiles`, `Import`, and `Settings` are secondary workflow surfaces rather
  than equal-weight destinations
- `Editor` and `Runtime Diagnostics` are heavy work modes because they are
  content-heavy and would bloat the main surface
- navigation may exist, but it must not become a generic sidebar/admin-shell
  product model
- TUN, mode, and script controls remain visible only as truthful unsupported
  controls until real runtime contracts exist
- script management is deferred to a future milestone instead of being faked
  into the current UI

## Current Implementation Status

Implemented or substantially implemented:

- `app.dart` has been reduced toward shell wiring instead of being the main
  long-form content implementation.
- `ClientHomeController` owns the main home state, form controllers, route
  state, workflow actions, and dashboard snapshot generation.
- `control_state.dart` defines immutable dashboard/control models, including
  unsupported TUN, agent mode, and script placeholders.
- The dashboard now uses runtime controls, truthful capability chips, summary
  cards, modal details, and a runtime signal band.
- Runtime signal history is UI-local, memory-only, and driven only by real
  runtime/probe updates.
- `Profiles`, `Import`, `Editor`, `Runtime Diagnostics`, and `Settings` have
  been extracted into focused view files.
- `Show details` opens dialogs for detail-heavy cards instead of expanding
  dashboard cards and destabilizing the grid.
- Clash/Mihomo YAML detection now gives a friendly wrong-format message
  without pretending to support Clash/Mihomo import.
- Windows launcher/window/taskbar icon handling has been corrected with a
  canonical W icon, explicit big/small window icons, and AppUserModelID.
- macOS AppIcon assets and Linux launcher/window icon assets are generated
  from the same W brand source.
- Linux packaging now includes desktop-entry and hicolor icon metadata.
- Widget/controller tests cover core workflows, truthful controls, runtime
  signal history, detail dialogs, and Clash/Mihomo YAML mismatch handling.

Partially implemented or still pending:

- Linux and macOS visual/icon acceptance still need to run on those host
  platforms or CI artifacts.
- Dashboard layout smoke tests for multiple viewport sizes are still pending.
- Secondary panels can still be visually tightened, especially `Profiles` and
  `Import`.
- `Editor` and `Runtime Diagnostics` can still be matured into stronger
  workbench-style surfaces.
- TUN, mode, and script remain unsupported placeholders because no real
  backend/runtime contract exists yet.
- Script create/edit/delete/select management is deferred to a future
  script-focused milestone.

## Interaction Model

### Primary Surface

The primary surface is the app's default long-lived control workspace.

It should keep:

- runtime core
- start/stop/refresh
- stack summary and native summary
- health summary
- connection metrics
- recent activity
- import/support summary
- truthful global controls such as system proxy

It should not become the dumping ground for every low-frequency workflow.

### Secondary Panels

Secondary panels are lower-frequency management or flow surfaces that should
not permanently live on the main control surface.

These include:

- `Profiles`
- `Import`
- `Settings`

They should preserve awareness of the main surface and reduce clutter rather
than behave like equal-weight top-level destinations.

### Heavy Work Modes

Heavy work modes are intentionally larger, more focused surfaces for content
that would otherwise bloat the main control surface.

These include:

- `Editor`
- deeper runtime diagnostics and probe workflows

They may take over more space temporarily, but they must do so because they
are content-heavy and not because the product defaults to generic page
navigation.

## Screen Model

- `Dashboard`
  - the primary control surface
  - runtime controls
  - runtime summaries
  - lightweight entry points into deeper workflows
- `Profiles`
  - management panel for saved profiles
  - save/load/duplicate/delete
  - profile metadata and current draft context
- `Import`
  - flow panel for `wrongsv` inspect/adapt
  - capability report
  - missing-field prompts
- `Editor`
  - heavy configuration work mode
  - endpoint, local proxy, and protocol-specific fields
- `Runtime Diagnostics`
  - heavy diagnostics work mode
  - probe configuration
  - raw result/error review
- `Settings`
  - low-frequency settings panel
  - desktop integration
  - system proxy details
  - unsupported feature notes

## Control Model

Keep the Dart-side truthful control model and continue using it as the only UI
source of truth for runtime controls.

The model should continue to expose at least:

- `systemProxy`
- `tun`
- `agentMode`
- `selectedScript`
- support and disabled-reason text for each control

Rules that must remain unchanged:

- disabled controls always show a reason
- unsupported controls never look enabled
- no dashboard/global control depends on parsing endpoint editor form fields

## Layout Constraints

### Anti-overflow rules

The main surface must resist growth:

- high-frequency and low-frequency content must not compete equally
- configuration-heavy forms must not remain permanently expanded on the
  control surface
- management workflows must not flatten into a long-scrolling page
- unsupported future controls may remain visible only when they do not
  dominate the hierarchy

### Dashboard-specific rules

- Keep dashboard responsibilities narrow and runtime-oriented.
- Remove or demote large launcher cards as the main navigation structure.
- Any entry point into deeper workflows should be lightweight and
  non-dominant.
- At `1440x900`, the core runtime controls and at least the most important
  summaries should be reachable without excessive scrolling.

### Heavy-mode rules

- `Editor` is the biggest overflow risk and must be treated as a focused work
  surface.
- Deep runtime diagnostics should not permanently share equal space with the
  primary control surface.

## Visual Direction

### Brand anchor

- The icon reference at `E:\irrit-us developer group\favicon_io line` defines
  a useful visual anchor for this milestone:
  - a compact, angled `W` mark
  - horizontal motion lines
  - a restrained dark-on-light treatment
- The desktop UI should borrow those traits at the layout level:
  - directional flow instead of flat dashboard boxes
  - compact, deliberate emphasis instead of many equal-weight cards
  - cleaner monochrome or slate-led surfaces with teal only as an operational
    accent

### UI polish rules

- Do not treat icon replacement as a separate cosmetic task.
- The shell, dashboard hero, and card headers should visually relate to the
  icon language.
- Workflow entry points should look like tool switches or workbench actions,
  not marketing tiles.
- Primary runtime actions should remain the visual center even after brand
  polish.
- Decorative styling must not reduce truthfulness or bury unsupported-state
  explanations.

## Plan

### Phase 1. Keep the truthful control model

Scope:

- Preserve the existing control-state architecture and current backend
  boundaries.

Tasks:

- Keep one orchestration owner/controller.
- Keep controller-owned form state.
- Keep truthful disabled controls for unsupported runtime features.
- Do not add fake TUN/mode/script semantics.

Verify:

- Existing workflow coverage remains valid.
- No UI change implies backend support that does not exist.

### Phase 2. Replace the current launcher-hub assumption

Scope:

- Remove the assumption that dashboard entry cards are the primary navigation
  model.

Tasks:

- Rewrite the shell so the main surface is long-lived by default.
- Reclassify `Profiles`, `Import`, and `Settings` as secondary panels.
- Reclassify `Editor` and deeper runtime workflows as heavy work modes.
- Keep any navigation support subordinate to this interaction model.

Verify:

- Users can reach deeper workflows without turning the app into a generic
  multi-page shell.
- The main surface remains useful on its own.

### Phase 3. Narrow dashboard responsibility

Scope:

- Make `Dashboard` a runtime overview workspace rather than a launcher hub.

Tasks:

- Keep runtime controls above the fold.
- Keep health, connection manager, recent activity, and import/support summary.
- Remove or demote large launcher-card navigation.
- Add only lightweight, non-dominant entry points into deeper workflows.

Verify:

- Dashboard remains the strongest mental center of the app.
- Dashboard no longer needs to carry every workflow equally.

### Phase 4. Rework interaction tiers

Scope:

- Move lower-frequency and heavy-content workflows into better-fitted surfaces.

Tasks:

- `Profiles`
  - move toward list/detail or slide-out management behavior
- `Import`
  - move toward a temporary flow panel that preserves awareness of the primary
    surface
- `Settings`
  - stay low-frequency and system-oriented
- `Editor`
  - stay focused and isolated enough to avoid polluting the main surface
- `Runtime`
  - keep shallow controls on dashboard
  - move deeper diagnostics into a heavier focused surface

Verify:

- The main surface gets smaller in responsibility without losing capability.
- Each workflow surface matches its real usage frequency and content weight.

### Phase 5. Polish hierarchy and density

Scope:

- Improve layout maturity without inventing fake functionality.

Tasks:

- Strengthen the visual distinction between:
  - primary runtime controls
  - summaries
  - secondary workflow entry points
- Reduce repeated helper text and weak launcher-like filler.
- Improve desktop density and first-screen usefulness.

Verify:

- The app feels more deliberate and less like either a dashboard launcher or a
  long settings document.

### Phase 6. Apply brand-led shell polish

Scope:

- Use the new icon reference to tighten the shell and dashboard visual system.

Tasks:

- Introduce a branded app header or title treatment.
- Add a stronger dashboard hero area that combines runtime identity and status.
- Rework workflow switches so they feel operational rather than decorative.
- Tighten section-card styling, spacing rhythm, and surface hierarchy.

Verify:

- The icon direction influences the whole shell, not just an asset swap.
- The app looks more productized without pretending to support new runtime
  features.

### Phase 7. Mature secondary-panel interiors

Scope:

- Make `Profiles`, `Import`, and `Settings` feel like intentional management
  panels instead of extracted form blocks.

Tasks:

- Add panel-level summary headers and short contextual guidance.
- Rework saved-profile presentation into clearer list rows with stronger
  selected-state feedback.
- Rework import flow into step-like sections with clearer report and
  missing-field hierarchy.
- Rework settings into grouped operational surfaces plus explicit placeholder
  notices for unsupported capabilities.

Verify:

- Secondary panels feel lighter than heavy work modes but more structured than
  raw forms.
- Information hierarchy improves without hiding any real capability limits.

### Phase 8. Refine the editor workbench

Scope:

- Turn `Editor` into a clearer multi-zone workbench instead of a single long
  form stack.

Tasks:

- Add a heavy-mode intro that explains the editor as a focused draft surface.
- Separate file I/O, endpoint identity, transport, outer security, and local
  proxy into clearer visual zones.
- Add contextual notes where protocol rules are easy to misunderstand.
- Keep field labels and action wording stable so workflow semantics do not
  drift during the UI refactor.

Verify:

- The editor feels like a purposeful work mode rather than another long panel.
- Users can scan the current draft structure more easily without any fake
  runtime abstraction being introduced.

### Phase 9. Refine runtime diagnostics

Scope:

- Turn `Runtime Diagnostics` into a clearer investigation surface instead of a
  single probe form and raw output block.

Tasks:

- Add a heavy-mode intro that frames diagnostics as a focused inspection tool.
- Separate runtime refresh, probe target, payload, and output review into
  distinct work zones.
- Surface the most recent health and error context near diagnostics so users do
  not need to mentally jump back to the dashboard.
- Keep probe actions truthful and mapped directly to the existing runtime API.

Verify:

- Diagnostics feels like a real troubleshooting workspace.
- Output review becomes easier without inventing new backend semantics.

### Phase 10. Add the signal layer and unify brand assets

Scope:

- Upgrade the dashboard with truthful local runtime trends and finish desktop
  icon source unification across launcher, window, task switcher, tray/menu
  bar, and in-app branding.

Tasks:

- Add controller-owned in-memory history for runtime stats and probe outcomes.
- Extend `DashboardSnapshot` with immutable signal-series and signal-event
  models.
- Add runtime signal cards and compact trend graphics without synthetic data.
- Keep charts empty until real samples exist.
- Move launcher/tray/in-app brand assets onto one repo-owned canonical icon
  family.
- Generate platform-specific desktop icon outputs from one W brand source:
  - Windows `.ico`
  - macOS `AppIcon.appiconset` PNG sizes
  - Linux launcher/window PNG
  - Flutter tray and in-app brand assets
- Keep Android and iOS out of this desktop parity milestone.

Verify:

- Dashboard graphs only reflect locally sampled runtime history.
- Windows, macOS, and Linux desktop shells all use the W brand family.
- Windows launcher icon and runner resource icon remain byte-identical.
- Linux packages include a desktop entry and hicolor app icon metadata.

### Phase 11. Desktop platform parity gate

Scope:

- Treat desktop parity as a release-quality gate after the shared Flutter UI
  has stabilized.

Tasks:

- Keep Windows as the reference implementation for explicit window icon setup.
- Ensure macOS AppIcon assets are regenerated from the same brand source.
- Ensure Linux sets the GTK window icon from a bundled repo-owned PNG.
- Ensure release packaging carries Linux desktop metadata where possible.
- Keep existing CI jobs as the authoritative build verification path for
  Linux, Windows, and macOS.

Verify:

- `flutter analyze`
- `flutter test`
- `flutter build windows`
- existing `scripts/verify-local.sh linux` on Linux or CI
- existing `scripts/verify-macos-host.sh` on macOS or CI
- manual check that titlebar/task switcher/tray/menu bar/in-app icons are not
  Flutter defaults on all desktop platforms.

## File Targets

- `wrongcl/lib/app.dart`
- `wrongcl/lib/dashboard_view.dart`
- `wrongcl/lib/client_home_controller.dart`
- `wrongcl/lib/subviews/...`
- `wrongcl/test/widget_test.dart`

## Implementation Order

1. Revise the plan assumptions and terminology
2. Rework the shell around primary surface / secondary panels / heavy modes
3. Narrow dashboard responsibilities
4. Refit Profiles/Import/Settings as secondary panels
5. Refit Editor and runtime diagnostics as heavy work modes
6. Run polish and regression verification

## Non-Goals

- no new protocol support
- no broad state-management rewrite
- no fake runtime capability expansion
- no major backend rewrite of working flows
- no generic sidebar-first product redesign as the default answer

## Release Gate

Before this work is called complete:

- `flutter analyze`
- `flutter test`
- `flutter build windows`
- desktop platform checks through existing Linux/macOS CI or host scripts
- manual desktop check that:
  - the primary surface stays focused on runtime control
  - secondary panels reduce clutter instead of recreating a long page
  - heavy work modes isolate content-heavy workflows without implying fake
    backend behavior
  - launcher, window, task switcher, tray/menu bar, and in-app icons share the
    same W brand family
