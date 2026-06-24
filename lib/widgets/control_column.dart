import 'package:flutter/material.dart';

import '../control_state.dart';
import '../theme/wrongcl_colors.dart';

class ControlColumn extends StatelessWidget {
  const ControlColumn({
    super.key,
    required this.systemProxy,
    required this.tun,
    required this.running,
    required this.busy,
    required this.onSystemProxyChanged,
    required this.onRuntimeChanged,
    required this.onTunChanged,
  });

  final ControlAvailability systemProxy;
  final ControlAvailability tun;
  final bool running;
  final bool busy;
  final ValueChanged<bool> onSystemProxyChanged;
  final ValueChanged<bool> onRuntimeChanged;
  final ValueChanged<bool> onTunChanged;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Container(
      decoration: BoxDecoration(
        color: palette.surface.surfaceRaised,
        border: Border.all(color: palette.border.contrast, width: 1.5),
        borderRadius: BorderRadius.circular(16),
      ),
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _ControlPill(
            label: 'SysProxy',
            value: systemProxy.enabled,
            disabledReason: systemProxy.supported
                ? ''
                : systemProxy.disabledReason,
            onChanged: systemProxy.supported && !busy
                ? onSystemProxyChanged
                : null,
          ),
          const SizedBox(height: 12),
          _RuntimePill(
            running: running,
            busy: busy,
            onChanged: onRuntimeChanged,
          ),
          const SizedBox(height: 6),
          Container(
            height: 1,
            color: palette.border.subtle,
          ),
          const SizedBox(height: 6),
          _ControlPill(
            label: 'TUN',
            value: tun.enabled,
            disabledReason: tun.supported ? '' : tun.disabledReason,
            onChanged: tun.supported && !busy ? onTunChanged : null,
          ),
        ],
      ),
    );
  }
}

class _ControlPill extends StatelessWidget {
  const _ControlPill({
    required this.label,
    required this.value,
    required this.disabledReason,
    required this.onChanged,
  });

  final String label;
  final bool value;
  final String disabledReason;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final enabled = onChanged != null;
    final pill = Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: palette.surface.surfaceMuted,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: palette.border.regular),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.center,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  label,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.labelLarge?.copyWith(
                    color: enabled ? null : palette.text.secondary,
                  ),
                ),
                if (!enabled && disabledReason.isNotEmpty) ...[
                  const SizedBox(height: 2),
                  Text(
                    disabledReason,
                    maxLines: 1,
                    textAlign: TextAlign.center,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: palette.text.secondary,
                    ),
                  ),
                ],
              ],
            ),
          ),
          Switch.adaptive(
            value: value,
            onChanged: onChanged,
          ),
        ],
      ),
    );
    if (!enabled && disabledReason.isNotEmpty) {
      return Tooltip(message: disabledReason, child: pill);
    }
    return pill;
  }
}

class _RuntimePill extends StatelessWidget {
  const _RuntimePill({
    required this.running,
    required this.busy,
    required this.onChanged,
  });

  final bool running;
  final bool busy;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: palette.accent.runtime,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.center,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  'Runtime',
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    color: palette.accent.runtimeOn,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  busy
                      ? 'Working...'
                      : running
                          ? 'Running'
                          : 'Stopped',
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    color: palette.accent.runtimeOn.withAlpha(180),
                    fontSize: 12,
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            tooltip: running ? 'Stop' : 'Start',
            onPressed: busy ? null : () => onChanged(!running),
            icon: Icon(
              running ? Icons.stop_circle : Icons.play_circle_fill,
              color: palette.accent.runtimeOn,
              size: 28,
            ),
          ),
        ],
      ),
    );
  }
}
