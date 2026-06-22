import 'package:flutter/material.dart';

import '../client_home_controller.dart';
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
    final connections = controller.activeConnections;
    return SubpageScaffold(
      title: 'Connections',
      onClose: onClose,
      actions: [
        if (connections.isNotEmpty)
          OutlinedButton.icon(
            onPressed: controller.closeAllConnections,
            icon: const Icon(Icons.close),
            label: const Text('Close all'),
          ),
      ],
      child: connections.isEmpty
          ? const _EmptyState(
              message:
                  'No active connections. Live entries appear here while '
                  'traffic is flowing through the local proxy.',
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
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFFFBFAF7),
        borderRadius: BorderRadius.circular(12),
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
                  connection.target,
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
                if (connection.sourceApp.isNotEmpty)
                  Text(
                    'via ${connection.sourceApp}',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: const Color(0xFF8B8579),
                    ),
                  ),
              ],
            ),
          ),
          IconButton(
            tooltip: 'Close',
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
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(
          message,
          textAlign: TextAlign.center,
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: const Color(0xFF8B8579)),
        ),
      ),
    );
  }
}
