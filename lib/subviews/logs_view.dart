import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../widgets/subpage_scaffold.dart';

class LogsView extends StatelessWidget {
  const LogsView({super.key, required this.controller, required this.onClose});

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final logs = controller.visibleLogs;
    final hasSourceLogs = controller.recentLogs.isNotEmpty;
    return SubpageScaffold(
      title: 'Logs',
      onClose: onClose,
      child: logs.isEmpty
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Text(
                  hasSourceLogs
                      ? 'No log entries match the current log-level filter.'
                      : 'No log entries captured yet. Recent runtime events '
                            'will stream here while the proxy is active.',
                  textAlign: TextAlign.center,
                  style: const TextStyle(color: Color(0xFF8B8579)),
                ),
              ),
            )
          : ListView.separated(
              padding: const EdgeInsets.all(12),
              itemCount: logs.length,
              separatorBuilder: (_, _) => const SizedBox(height: 4),
              itemBuilder: (context, index) {
                final entry = logs[index];
                return _LogRow(entry: entry);
              },
            ),
    );
  }
}

class _LogRow extends StatefulWidget {
  const _LogRow({required this.entry});

  final LogEntry entry;

  @override
  State<_LogRow> createState() => _LogRowState();
}

class _LogRowState extends State<_LogRow> {
  bool expanded = false;

  Color _levelColor() {
    switch (widget.entry.level.toUpperCase()) {
      case 'ERROR':
        return const Color(0xFFB00020);
      case 'WARN':
        return const Color(0xFF9A6700);
      case 'INFO':
        return const Color(0xFF2F4858);
      case 'DEBUG':
        return const Color(0xFF6F6A5F);
      default:
        return const Color(0xFF8B8579);
    }
  }

  @override
  Widget build(BuildContext context) {
    final color = _levelColor();
    return InkWell(
      onTap: () => setState(() => expanded = !expanded),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: const Color(0xFFFBFAF7),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: const Color(0xFFE5E2DA)),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 2,
                  ),
                  decoration: BoxDecoration(
                    color: color.withAlpha(28),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    widget.entry.level.toUpperCase(),
                    style: TextStyle(
                      color: color,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    widget.entry.message,
                    maxLines: expanded ? null : 2,
                    overflow: expanded
                        ? TextOverflow.visible
                        : TextOverflow.ellipsis,
                    style: const TextStyle(fontSize: 12),
                  ),
                ),
              ],
            ),
            if (expanded && widget.entry.target.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 4, left: 4),
                child: Text(
                  widget.entry.target,
                  style: const TextStyle(
                    fontSize: 11,
                    color: Color(0xFF8B8579),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
