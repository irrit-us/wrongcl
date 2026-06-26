import 'package:flutter/material.dart';

import '../control_state.dart';
import '../l10n/app_localizations.dart';
import '../theme/wrongcl_colors.dart';
import 'entry_chip.dart';

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
    this.iconSide = ChipIconSide.left,
  });

  final ControlAvailability systemProxy;
  final ControlAvailability tun;
  final bool running;
  final bool busy;
  final ValueChanged<bool> onSystemProxyChanged;
  final ValueChanged<bool> onRuntimeChanged;
  final ValueChanged<bool> onTunChanged;
  final ChipIconSide iconSide;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Container(
      decoration: BoxDecoration(
        color: palette.surface.surfaceRaised,
        border: Border.all(color: palette.border.contrast, width: 1.5),
        borderRadius: BorderRadius.circular(16),
      ),
      padding: const EdgeInsets.all(8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: _ControlPill(
              label: 'SysProxy',
              value: systemProxy.enabled,
              disabledReason: systemProxy.supported
                  ? ''
                  : systemProxy.disabledReason,
              onChanged: systemProxy.supported && !busy
                  ? onSystemProxyChanged
                  : null,
              iconSide: iconSide,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: _RuntimePill(
              running: running,
              busy: busy,
              onChanged: onRuntimeChanged,
              iconSide: iconSide,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: _ControlPill(
              label: 'TUN',
              value: tun.enabled,
              disabledReason: tun.supported ? '' : tun.disabledReason,
              onChanged: tun.supported && !busy ? onTunChanged : null,
              iconSide: iconSide,
            ),
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
    required this.iconSide,
  });

  final String label;
  final bool value;
  final String disabledReason;
  final ValueChanged<bool>? onChanged;
  final ChipIconSide iconSide;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final enabled = onChanged != null;
    final iconOnRight = iconSide == ChipIconSide.right;
    final textAlign = iconOnRight ? TextAlign.end : TextAlign.start;
    final crossAxis = iconOnRight
        ? CrossAxisAlignment.end
        : CrossAxisAlignment.start;
    final textBlock = Column(
      crossAxisAlignment: crossAxis,
      mainAxisAlignment: MainAxisAlignment.center,
      mainAxisSize: MainAxisSize.min,
      children: [
        FittedBox(
          fit: BoxFit.scaleDown,
          alignment: iconOnRight
              ? Alignment.centerRight
              : Alignment.centerLeft,
          child: Text(
            label,
            textAlign: textAlign,
            style: Theme.of(context).textTheme.labelLarge?.copyWith(
              color: enabled ? null : palette.text.secondary,
            ),
          ),
        ),
        if (!enabled && disabledReason.isNotEmpty) ...[
          const SizedBox(height: 2),
          Text(
            disabledReason,
            maxLines: 1,
            textAlign: textAlign,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: palette.text.secondary,
            ),
          ),
        ],
      ],
    );
    final switchWidget = Switch.adaptive(
      value: value,
      onChanged: onChanged,
    );
    final pill = Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: palette.surface.surfaceMuted,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: palette.border.regular),
      ),
      child: Row(
        children: iconOnRight
            ? [switchWidget, const SizedBox(width: 8), Expanded(child: textBlock)]
            : [Expanded(child: textBlock), const SizedBox(width: 8), switchWidget],
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
    required this.iconSide,
  });

  final bool running;
  final bool busy;
  final ValueChanged<bool> onChanged;
  final ChipIconSide iconSide;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final l10n = AppLocalizations.of(context);
    final iconOnRight = iconSide == ChipIconSide.right;
    final textAlign = iconOnRight ? TextAlign.end : TextAlign.start;
    final crossAxis = iconOnRight
        ? CrossAxisAlignment.end
        : CrossAxisAlignment.start;
    final textBlock = FittedBox(
      fit: BoxFit.scaleDown,
      alignment: iconOnRight ? Alignment.centerRight : Alignment.centerLeft,
      child: Column(
        crossAxisAlignment: crossAxis,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            l10n.runtimeLabel,
            textAlign: textAlign,
            style: TextStyle(
              color: palette.accent.runtimeOn,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            busy
                ? l10n.runtimeWorking
                : running
                    ? l10n.runtimeRunning
                    : l10n.runtimeStopped,
            textAlign: textAlign,
            style: TextStyle(
              color: palette.accent.runtimeOn.withAlpha(180),
              fontSize: 12,
            ),
          ),
        ],
      ),
    );
    final iconWidget = IconButton(
      tooltip: running ? l10n.runtimeStopTooltip : l10n.runtimeStartTooltip,
      onPressed: busy ? null : () => onChanged(!running),
      icon: Icon(
        running ? Icons.stop_circle : Icons.play_circle_fill,
        color: palette.accent.runtimeOn,
        size: 28,
      ),
    );
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: palette.accent.runtime,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Row(
        children: iconOnRight
            ? [iconWidget, const SizedBox(width: 8), Expanded(child: textBlock)]
            : [Expanded(child: textBlock), const SizedBox(width: 8), iconWidget],
      ),
    );
  }
}
