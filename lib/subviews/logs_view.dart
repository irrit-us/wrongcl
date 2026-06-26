import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../l10n/app_localizations.dart';
import '../theme/wrongcl_colors.dart';
import '../widgets/subpage_scaffold.dart';

class LogsView extends StatelessWidget {
  const LogsView({super.key, required this.controller, required this.onClose});

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final l10n = AppLocalizations.of(context);
    final logs = controller.visibleLogs;
    final hasSourceLogs = controller.recentLogs.isNotEmpty;
    return SubpageScaffold(
      title: l10n.navLogs,
      onClose: onClose,
      child: logs.isEmpty
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Text(
                  hasSourceLogs ? l10n.logsNoMatch : l10n.logsEmpty,
                  textAlign: TextAlign.center,
                  style: TextStyle(color: palette.text.secondary),
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

  Color _levelColor(WrongclColors palette) {
    switch (widget.entry.level.toUpperCase()) {
      case 'ERROR':
        return palette.status.danger;
      case 'WARN':
        return palette.status.warning;
      case 'INFO':
        return palette.status.info;
      case 'DEBUG':
        return palette.text.tertiary;
      default:
        return palette.text.secondary;
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final color = _levelColor(palette);
    return InkWell(
      onTap: () => setState(() => expanded = !expanded),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: palette.surface.surfaceRaised,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: palette.chart.grid),
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
                  style: TextStyle(
                    fontSize: 11,
                    color: palette.text.secondary,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
