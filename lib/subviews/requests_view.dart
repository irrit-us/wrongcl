import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../theme/wrongcl_colors.dart';
import '../widgets/subpage_scaffold.dart';

class RequestsView extends StatelessWidget {
  const RequestsView({
    super.key,
    required this.controller,
    required this.onClose,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final requests = controller.recentRequests.reversed.toList(growable: false);
    return SubpageScaffold(
      title: 'Requests',
      onClose: onClose,
      child: requests.isEmpty
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Text(
                  'No captured requests yet. Send traffic through the local '
                  'proxy and the most recent requests will appear here.',
                  textAlign: TextAlign.center,
                  style: TextStyle(color: palette.text.secondary),
                ),
              ),
            )
          : ListView.separated(
              padding: const EdgeInsets.all(12),
              itemCount: requests.length,
              separatorBuilder: (_, _) => const SizedBox(height: 8),
              itemBuilder: (context, index) {
                final r = requests[index];
                final theme = Theme.of(context);
                final subtitleParts = <String>[];
                if (r.host != null && r.host!.isNotEmpty && r.host != r.target) {
                  subtitleParts.add('Host: ${r.host}');
                }
                final sourceLabel = _formatSource(r.sourceApp, r.sourcePid);
                if (sourceLabel.isNotEmpty) {
                  subtitleParts.add(sourceLabel);
                }
                subtitleParts.add(_formatTime(r.timestamp));
                return Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: palette.surface.surfaceRaised,
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: palette.border.regular),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.center,
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 8,
                              vertical: 2,
                            ),
                            decoration: BoxDecoration(
                              color: palette.surface.surfaceHighlight,
                              borderRadius: BorderRadius.circular(6),
                            ),
                            child: Text(
                              r.method.isEmpty ? '?' : r.method,
                              style: theme.textTheme.labelSmall?.copyWith(
                                fontFeatures: const [
                                  FontFeature.tabularFigures(),
                                ],
                              ),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              r.url == null || r.url!.isEmpty
                                  ? r.target
                                  : r.url!,
                              style: theme.textTheme.bodyMedium,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                      if (subtitleParts.isNotEmpty) ...[
                        const SizedBox(height: 4),
                        Text(
                          subtitleParts.join(' · '),
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: palette.text.secondary,
                          ),
                        ),
                      ],
                    ],
                  ),
                );
              },
            ),
    );
  }

  String _formatSource(String app, int? pid) {
    if (app.isEmpty && pid == null) return '';
    if (app.isEmpty) return 'pid $pid';
    if (pid == null) return app;
    return '$app (pid $pid)';
  }

  String _formatTime(DateTime ts) {
    final local = ts.toLocal();
    String two(int v) => v.toString().padLeft(2, '0');
    return '${two(local.hour)}:${two(local.minute)}:${two(local.second)}';
  }
}
