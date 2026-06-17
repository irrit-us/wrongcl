import 'package:flutter/material.dart';

class HealthProbeSnapshot {
  const HealthProbeSnapshot({
    required this.bytesRead,
    required this.preview,
    required this.timestamp,
  });

  final int bytesRead;
  final String preview;
  final DateTime timestamp;
}

class HealthErrorSnapshot {
  const HealthErrorSnapshot({
    required this.action,
    required this.message,
    required this.timestamp,
  });

  final String action;
  final String message;
  final DateTime timestamp;
}

class HealthSummaryView extends StatelessWidget {
  const HealthSummaryView({
    super.key,
    required this.running,
    required this.stats,
    required this.lastProbe,
    required this.lastError,
  });

  final bool running;
  final Map<String, Object?> stats;
  final HealthProbeSnapshot? lastProbe;
  final HealthErrorSnapshot? lastError;

  String _formatTime(DateTime value) {
    final local = value.toLocal();
    final hour = local.hour.toString().padLeft(2, '0');
    final minute = local.minute.toString().padLeft(2, '0');
    final second = local.second.toString().padLeft(2, '0');
    return '$hour:$minute:$second';
  }

  Widget _pill(BuildContext context, String label, String value, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: color.withAlpha(20),
        borderRadius: BorderRadius.circular(999),
      ),
      child: RichText(
        text: TextSpan(
          style: Theme.of(context).textTheme.bodySmall?.copyWith(color: color),
          children: [
            TextSpan(text: '$label: '),
            TextSpan(
              text: value,
              style: const TextStyle(fontWeight: FontWeight.w600),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final errorColor = Theme.of(context).colorScheme.error;
    final stateColor = lastError != null
        ? errorColor
        : running
        ? Colors.green.shade700
        : Colors.grey.shade700;
    final stateLabel = lastError != null
        ? 'Needs attention'
        : running
        ? 'Healthy'
        : 'Idle';
    final localHost = stats['local_host'] as String? ?? '';
    final localPort = stats['local_port'];
    final listener = localHost.isNotEmpty && localPort != null
        ? '$localHost:$localPort'
        : 'Not listening';
    final failedConnections = '${stats['failed_connections'] ?? 0}';
    final probeText = lastProbe == null
        ? 'No probe recorded'
        : '${_formatTime(lastProbe!.timestamp)} | ${lastProbe!.bytesRead} bytes | ${lastProbe!.preview}';
    final errorText = lastError == null
        ? 'No error recorded'
        : '${_formatTime(lastError!.timestamp)} | ${lastError!.action}: ${lastError!.message}';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            _pill(context, 'State', stateLabel, stateColor),
            _pill(context, 'Listener', listener, Colors.blueGrey.shade700),
            _pill(context, 'Failed', failedConnections, errorColor),
          ],
        ),
        const SizedBox(height: 12),
        Text(
          'Last successful probe',
          style: Theme.of(context).textTheme.labelLarge,
        ),
        const SizedBox(height: 4),
        Text(probeText, style: Theme.of(context).textTheme.bodySmall),
        const SizedBox(height: 12),
        Text('Last error', style: Theme.of(context).textTheme.labelLarge),
        const SizedBox(height: 4),
        Text(errorText, style: Theme.of(context).textTheme.bodySmall),
      ],
    );
  }
}
