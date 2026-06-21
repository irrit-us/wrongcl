# wrongcl UI Upgrade Handoff

Last updated: 2026-06-21.

## Current Objective

Continue polishing `wrongcl` as a control-first Flutter + Rust desktop client.
The current milestone focuses on a stable dashboard, truthful runtime visuals,
better import errors, and desktop-wide icon parity for Windows, macOS, and
Linux.

## Current State

- Dashboard is no longer the original long form page.
- Main shell uses:
  - primary dashboard surface
  - secondary panels for Profiles, Import, Settings
  - heavy work modes for Editor and Diagnostics
- Runtime signal history is UI-local and memory-only.
- TUN, agent mode, and script controls remain unsupported placeholders with
  explicit reasons.
- Clash/Mihomo YAML is not supported by the wrongsv import flow.

## Recent UI Decisions

- Runtime charts belong in the main `Runtime Signals` band and should be large
  enough to read without opening details.
- `Connection Manager` is a summary card only; it must not have `Show details`.
- `Show details` is reserved for text/detail-heavy cards:
  - Health
  - Recent Activity
  - Import / Support State
- Details open in a dialog and must not change dashboard card height.
- Dashboard rows use explicit preview heights to avoid overflow and accidental
  click blocking.

## Icon Work

- Canonical brand generation entry point: `scripts/gen_tray_icon.py`
- The script now generates:
  - legacy `assets/tray_icon.*`
  - `assets/brand/wrongcl_launcher.ico`
  - `assets/brand/wrongcl_tray.ico`
  - `assets/brand/wrongcl_tray.png`
  - `assets/brand/wrongcl_mark.png`
  - `assets/brand/wrongcl_app_mark.png`
  - macOS `AppIcon.appiconset/app_icon_*.png`
  - Linux `linux/runner/resources/wrongcl.png`
- Windows resource icon: `windows/runner/resources/app_icon.ico`
- `windows/runner/resources/app_icon.ico` must remain byte-identical to
  `assets/brand/wrongcl_launcher.ico`.
- Windows runner sets:
  - app resource icon through `Runner.rc`
  - big/small window icons through `WM_SETICON`
  - process AppUserModelID: `us.irrit.wrongcl`
- Linux runner sets the GTK window icon from bundled `data/wrongcl.png`.
- Linux release packaging includes:
  - `share/applications/us.irrit.wrongcl.desktop`
  - `share/icons/hicolor/512x512/apps/us.irrit.wrongcl.png`
- macOS launcher icons are generated into the existing AppIcon asset catalog
  from the same W brand source.
- If Windows still shows the Flutter icon after a clean build, suspect Windows
  icon cache or pinned-taskbar cache first.

## Important Constraints

- Do not change `pubspec.yaml` Dart SDK constraint.
- Rust builds require `PROTOC`:
  `C:\Users\Charon\AppData\Local\Microsoft\WinGet\Packages\Google.Protobuf_Microsoft.Winget.Source_8wekyb3d8bbwe\bin\protoc.exe`
- Flutter engine/download commands may need:
  `HTTP_PROXY=http://127.0.0.1:7897`
  `HTTPS_PROXY=http://127.0.0.1:7897`
- Do not fake support for TUN, mode, script runtime selection, or Windows
  system proxy.
- Do not add Clash/Mihomo importing unless a real adapter is explicitly
  planned and tested.

## Verification Commands

Run from `E:\irrit-us developer group\wrongcl`:

```powershell
flutter analyze
flutter test
$env:PROTOC='C:\Users\Charon\AppData\Local\Microsoft\WinGet\Packages\Google.Protobuf_Microsoft.Winget.Source_8wekyb3d8bbwe\bin\protoc.exe'
$env:HTTP_PROXY='http://127.0.0.1:7897'
$env:HTTPS_PROXY='http://127.0.0.1:7897'
flutter build windows
```

Desktop parity checks:

```powershell
python scripts/gen_tray_icon.py
Get-FileHash assets\brand\wrongcl_launcher.ico,windows\runner\resources\app_icon.ico
```

Run on Linux or CI:

```bash
bash scripts/verify-local.sh linux
bash scripts/package-linux-release.sh
```

Run on macOS or CI:

```bash
bash scripts/verify-macos-host.sh
bash scripts/package-macos-release.sh
```

Manual check:

- Open `build\windows\x64\runner\Release\wrongcl.exe`.
- Confirm title bar, taskbar, Alt-Tab, tray, and in-app icons are not Flutter.
- On Linux, confirm the GTK window icon, task switcher icon, tray icon, and
  packaged desktop metadata use the W brand family.
- On macOS, confirm Finder, Dock, Cmd-Tab, menu bar/tray, and in-app icons use
  the W brand family.
- Confirm dashboard `Show details` opens dialogs for Health, Activity, Import.
- Confirm `Connection Manager` has no `Show details`.
- Confirm Clash/Mihomo YAML in Import shows a friendly format mismatch.

## Next Suggested Work

- Continue visual QA at 1440x900, 768x1024, and 390x844.
- Tighten secondary panel interiors, especially Import and Profiles.
- Consider adding widget layout smoke tests for dashboard overflow.
- Keep reducing explanatory text where the control itself is clear.
