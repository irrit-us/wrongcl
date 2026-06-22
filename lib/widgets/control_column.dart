import 'package:flutter/material.dart';

import '../control_state.dart';

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
    return Container(
      decoration: BoxDecoration(
        color: const Color(0xFFFBFAF7),
        border: Border.all(color: const Color(0xFF1F2933), width: 1.5),
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
            color: const Color(0xFFD7D2C8),
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
    final enabled = onChanged != null;
    final pill = Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: const Color(0xFFF4F1EA),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: const Color(0xFFDCD5CA)),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  label,
                  style: Theme.of(context).textTheme.labelLarge?.copyWith(
                    color: enabled ? null : const Color(0xFF8B8579),
                  ),
                ),
                if (!enabled && disabledReason.isNotEmpty) ...[
                  const SizedBox(height: 2),
                  Text(
                    disabledReason,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: const Color(0xFF8B8579),
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
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: const Color(0xFF111111),
        borderRadius: BorderRadius.circular(14),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text(
                  'Runtime',
                  style: TextStyle(
                    color: Colors.white,
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
                  style: const TextStyle(color: Colors.white70, fontSize: 12),
                ),
              ],
            ),
          ),
          IconButton(
            tooltip: running ? 'Stop' : 'Start',
            onPressed: busy ? null : () => onChanged(!running),
            icon: Icon(
              running ? Icons.stop_circle : Icons.play_circle_fill,
              color: Colors.white,
              size: 28,
            ),
          ),
        ],
      ),
    );
  }
}
