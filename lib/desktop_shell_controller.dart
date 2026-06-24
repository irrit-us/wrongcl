import 'dart:async';
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

class DesktopShellState {
  const DesktopShellState({
    required this.running,
    required this.busy,
    required this.status,
    required this.profileName,
  });

  final bool running;
  final bool busy;
  final String status;
  final String profileName;
}

class DesktopShellActions {
  const DesktopShellActions({
    required this.startProxy,
    required this.stopProxy,
    required this.refreshStatus,
    required this.prepareForQuit,
  });

  final Future<void> Function() startProxy;
  final Future<void> Function() stopProxy;
  final Future<void> Function() refreshStatus;
  final Future<void> Function() prepareForQuit;
}

abstract interface class DesktopShellController {
  Future<void> bootstrap();

  Future<void> attach({
    required DesktopShellActions actions,
    required DesktopShellState initialState,
  });

  Future<void> sync(DesktopShellState state);

  Future<void> dispose();
}

class NoopDesktopShellController implements DesktopShellController {
  const NoopDesktopShellController();

  @override
  Future<void> attach({
    required DesktopShellActions actions,
    required DesktopShellState initialState,
  }) async {}

  @override
  Future<void> bootstrap() async {}

  @override
  Future<void> dispose() async {}

  @override
  Future<void> sync(DesktopShellState state) async {}
}

class TrayDesktopShellController
    with TrayListener, WindowListener
    implements DesktopShellController {
  static const _showWindowKey = 'show_window';
  static const _hideWindowKey = 'hide_window';
  static const _startProxyKey = 'start_proxy';
  static const _stopProxyKey = 'stop_proxy';
  static const _refreshStatusKey = 'refresh_status';
  static const _quitKey = 'quit_app';

  bool _bootstrapped = false;
  bool _enabled = false;
  bool _attached = false;
  bool _quitting = false;

  DesktopShellActions? _actions;
  DesktopShellState _state = const DesktopShellState(
    running: false,
    busy: false,
    status: 'Stopped',
    profileName: 'default',
  );

  bool get _supportsDesktop =>
      Platform.isLinux || Platform.isMacOS || Platform.isWindows;

  @override
  Future<void> bootstrap() async {
    if (_bootstrapped) {
      return;
    }
    _bootstrapped = true;
    if (!_supportsDesktop) {
      return;
    }

    WidgetsFlutterBinding.ensureInitialized();
    try {
      await windowManager.ensureInitialized();
      _enabled = true;
      const windowOptions = WindowOptions(
        size: Size(768, 552),
        minimumSize: Size(640, 480),
        center: true,
        skipTaskbar: false,
        title: 'Wrongcl',
      );
      unawaited(
        windowManager.waitUntilReadyToShow(windowOptions, () async {
          await windowManager.show();
          await windowManager.focus();
        }),
      );
    } catch (_) {
      _enabled = false;
    }
  }

  @override
  Future<void> attach({
    required DesktopShellActions actions,
    required DesktopShellState initialState,
  }) async {
    await bootstrap();
    if (!_enabled) {
      return;
    }

    _actions = actions;
    _state = initialState;
    if (_attached) {
      await sync(initialState);
      return;
    }

    try {
      trayManager.addListener(this);
      windowManager.addListener(this);
      await windowManager.setPreventClose(true);
      await trayManager.setIcon(_trayIconPath());
      if (!Platform.isLinux) {
        await trayManager.setToolTip(_toolTip());
      }
      _attached = true;
      await _updateTrayMenu();
    } catch (_) {
      _attached = false;
      _enabled = false;
    }
  }

  @override
  Future<void> sync(DesktopShellState state) async {
    if (!_attached) {
      return;
    }
    _state = state;
    await _updateTrayMenu();
    if (!Platform.isLinux) {
      await trayManager.setToolTip(_toolTip());
    }
  }

  @override
  Future<void> dispose() async {
    if (!_attached) {
      return;
    }
    trayManager.removeListener(this);
    windowManager.removeListener(this);
    await trayManager.destroy();
    _attached = false;
  }

  @override
  void onTrayIconMouseDown() {
    unawaited(_showWindow());
  }

  @override
  void onTrayIconRightMouseDown() {
    if (!Platform.isLinux) {
      unawaited(trayManager.popUpContextMenu());
    }
  }

  @override
  Future<void> onTrayMenuItemClick(MenuItem menuItem) async {
    switch (menuItem.key) {
      case _showWindowKey:
        await _showWindow();
        return;
      case _hideWindowKey:
        await _hideWindowToTray();
        return;
      case _startProxyKey:
        await _actions?.startProxy();
        return;
      case _stopProxyKey:
        await _actions?.stopProxy();
        return;
      case _refreshStatusKey:
        await _actions?.refreshStatus();
        return;
      case _quitKey:
        _quitting = true;
        await _actions?.prepareForQuit();
        await windowManager.destroy();
        return;
      default:
        return;
    }
  }

  @override
  void onWindowClose() {
    if (_quitting) {
      return;
    }
    unawaited(_hideWindowToTray());
  }

  @override
  void onWindowMinimize() {
    unawaited(_updateTrayMenu());
  }

  @override
  void onWindowRestore() {
    unawaited(_updateTrayMenu());
  }

  String _trayIconPath() {
    return Platform.isWindows
        ? 'assets/brand/wrongcl_tray.ico'
        : 'assets/brand/wrongcl_tray.png';
  }

  String _toolTip() {
    final state = _state.running ? 'running' : 'stopped';
    return 'Wrongcl ($state)';
  }

  String _profileLabel() {
    final profile = _state.profileName.trim();
    if (profile.isEmpty) {
      return 'Profile: default';
    }
    return 'Profile: $profile';
  }

  String _statusLabel() {
    final status = _state.status.trim();
    if (status.isEmpty) {
      return _state.running ? 'Status: running' : 'Status: stopped';
    }
    return 'Status: $status';
  }

  Future<void> _showWindow() async {
    await windowManager.setSkipTaskbar(false);
    await windowManager.show();
    await windowManager.focus();
    await _updateTrayMenu();
  }

  Future<void> _hideWindowToTray() async {
    await windowManager.setSkipTaskbar(true);
    await windowManager.hide();
    await _updateTrayMenu();
  }

  Future<void> _updateTrayMenu() async {
    if (!_attached) {
      return;
    }
    final isVisible = await windowManager.isVisible();
    final startStopItem = _state.running
        ? MenuItem(
            key: _stopProxyKey,
            label: 'Stop proxy',
            disabled: _state.busy,
          )
        : MenuItem(
            key: _startProxyKey,
            label: 'Start proxy',
            disabled: _state.busy,
          );
    final menu = Menu(
      items: [
        MenuItem(
          key: isVisible ? _hideWindowKey : _showWindowKey,
          label: isVisible ? 'Hide window' : 'Show window',
        ),
        MenuItem.separator(),
        MenuItem(label: _profileLabel(), disabled: true),
        MenuItem(label: _statusLabel(), disabled: true),
        MenuItem.separator(),
        startStopItem,
        MenuItem(
          key: _refreshStatusKey,
          label: 'Refresh status',
          disabled: _state.busy,
        ),
        MenuItem.separator(),
        MenuItem(key: _quitKey, label: 'Quit wrongcl'),
      ],
    );
    await trayManager.setContextMenu(menu);
  }
}
