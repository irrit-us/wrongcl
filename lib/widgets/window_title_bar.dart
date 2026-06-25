import 'dart:async';

import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

import '../theme/wrongcl_colors.dart';

/// Custom Flutter-rendered window title bar.
///
/// Replaces the native title bar (which we disable via
/// [TitleBarStyle.hidden] in [TrayDesktopShellController]). Renders in the
/// app's theme palette across Linux/Windows/macOS and adds a pin button for
/// the always-on-top toggle next to the standard min/max/close controls.
///
/// Only mount this widget when the desktop shell has actually bootstrapped
/// `window_manager` — see [DesktopShellController.hasNativeWindowShell].
class WindowTitleBar extends StatefulWidget {
  const WindowTitleBar({super.key});

  @override
  State<WindowTitleBar> createState() => _WindowTitleBarState();
}

class _WindowTitleBarState extends State<WindowTitleBar> with WindowListener {
  bool _pinned = false;
  bool _maximized = false;

  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    unawaited(_refresh());
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    super.dispose();
  }

  Future<void> _refresh() async {
    final pinned = await windowManager.isAlwaysOnTop();
    final maximized = await windowManager.isMaximized();
    if (!mounted) return;
    setState(() {
      _pinned = pinned;
      _maximized = maximized;
    });
  }

  @override
  void onWindowMaximize() {
    if (!mounted) return;
    setState(() => _maximized = true);
  }

  @override
  void onWindowUnmaximize() {
    if (!mounted) return;
    setState(() => _maximized = false);
  }

  Future<void> _togglePin() async {
    final next = !_pinned;
    await windowManager.setAlwaysOnTop(next);
    if (!mounted) return;
    setState(() => _pinned = next);
  }

  Future<void> _toggleMaximize() async {
    if (_maximized) {
      await windowManager.unmaximize();
    } else {
      await windowManager.maximize();
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final theme = Theme.of(context);
    return Container(
      height: 32,
      decoration: BoxDecoration(
        color: palette.topBar.background,
        border: Border(
          bottom: BorderSide(color: palette.border.subtle, width: 1),
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: DragToMoveArea(
              child: GestureDetector(
                onDoubleTap: _toggleMaximize,
                child: SizedBox.expand(
                  child: Padding(
                    padding: const EdgeInsetsDirectional.only(start: 12),
                    child: Align(
                      alignment: AlignmentDirectional.centerStart,
                      child: Text(
                        'wrongcl',
                        style: theme.textTheme.labelLarge?.copyWith(
                          color: palette.topBar.foreground,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
          _TitleBarButton(
            icon: _pinned ? Icons.push_pin : Icons.push_pin_outlined,
            tooltip: _pinned ? 'Unpin from top' : 'Keep on top',
            onPressed: _togglePin,
            active: _pinned,
          ),
          _TitleBarButton(
            icon: Icons.remove,
            tooltip: 'Minimize',
            onPressed: () => windowManager.minimize(),
          ),
          _TitleBarButton(
            icon: _maximized ? Icons.filter_none : Icons.crop_square,
            tooltip: _maximized ? 'Restore' : 'Maximize',
            onPressed: _toggleMaximize,
          ),
          _TitleBarButton(
            icon: Icons.close,
            tooltip: 'Close',
            onPressed: () => windowManager.close(),
            danger: true,
          ),
        ],
      ),
    );
  }
}

class _TitleBarButton extends StatefulWidget {
  const _TitleBarButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
    this.active = false,
    this.danger = false,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;
  final bool active;
  final bool danger;

  @override
  State<_TitleBarButton> createState() => _TitleBarButtonState();
}

class _TitleBarButtonState extends State<_TitleBarButton> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    Color? background;
    Color foreground = widget.active
        ? palette.accent.primary
        : palette.topBar.foreground;
    if (_hovered) {
      if (widget.danger) {
        background = palette.status.danger;
        foreground = palette.surface.onAccent;
      } else {
        background = palette.topBar.activeCell;
      }
    } else if (widget.active) {
      background = palette.topBar.activeCell;
    }
    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      cursor: SystemMouseCursors.click,
      child: Tooltip(
        message: widget.tooltip,
        child: GestureDetector(
          onTap: widget.onPressed,
          child: Container(
            width: 44,
            color: background ?? Colors.transparent,
            alignment: Alignment.center,
            child: Icon(widget.icon, size: 14, color: foreground),
          ),
        ),
      ),
    );
  }
}
