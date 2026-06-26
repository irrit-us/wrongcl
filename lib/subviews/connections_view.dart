import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../l10n/app_localizations.dart';
import '../theme/wrongcl_colors.dart';
import '../widgets/subpage_scaffold.dart';

class ConnectionsView extends StatelessWidget {
  const ConnectionsView({
    super.key,
    required this.controller,
    required this.onClose,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final connections = controller.activeConnections;
    return SubpageScaffold(
      title: l10n.navConnections,
      onClose: onClose,
      actions: [
        if (connections.isNotEmpty)
          OutlinedButton.icon(
            onPressed: controller.closeAllConnections,
            icon: const Icon(Icons.close),
            label: Text(l10n.connectionsCloseAll),
          ),
      ],
      child: connections.isEmpty
          ? _EmptyState(
              message: l10n.connectionsEmpty,
            )
          : ListView.separated(
              padding: const EdgeInsets.all(12),
              itemCount: connections.length,
              separatorBuilder: (_, _) => const SizedBox(height: 8),
              itemBuilder: (context, index) {
                final c = connections[index];
                return _ConnectionRow(
                  connection: c,
                  onClose: () => controller.closeConnection(c.id),
                );
              },
            ),
    );
  }
}

class _ConnectionRow extends StatelessWidget {
  const _ConnectionRow({required this.connection, required this.onClose});

  final ConnectionInfo connection;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final l10n = AppLocalizations.of(context);
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: palette.surface.surfaceRaised,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: palette.border.regular),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  connection.target,
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
                if (connection.sourceApp.isNotEmpty)
                  Text(
                    l10n.connectionsVia(connection.sourceApp),
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: palette.text.secondary,
                    ),
                  ),
              ],
            ),
          ),
          IconButton(
            tooltip: l10n.commonClose,
            onPressed: onClose,
            icon: const Icon(Icons.close),
          ),
        ],
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(
          message,
          textAlign: TextAlign.center,
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: palette.text.secondary),
        ),
      ),
    );
  }
}
