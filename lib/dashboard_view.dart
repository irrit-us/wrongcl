import 'package:flutter/material.dart';

import 'client_home_controller.dart';
import 'control_state.dart';
import 'health_view.dart';
import 'home_widgets.dart';
import 'signal_widgets.dart';

class DashboardView extends StatefulWidget {
  const DashboardView({
    super.key,
    required this.controller,
    required this.snapshot,
  });

  final ClientHomeController controller;
  final DashboardSnapshot snapshot;

  @override
  State<DashboardView> createState() => _DashboardViewState();
}

class _DashboardViewState extends State<DashboardView> {
  @override
  Widget build(BuildContext context) {
    final snapshot = widget.snapshot;
    final controller = widget.controller;
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 1380),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              StatusBar(
                running: snapshot.running,
                busy: snapshot.busy,
                status: snapshot.statusText,
                stackSummary: snapshot.stackSummary,
                nativeInfo: snapshot.nativeInfo,
              ),
              const SizedBox(height: 16),
              SectionCard(
                eyebrow: 'Primary Surface',
                title: 'Control Center',
                child: _buildControlCenter(context, controller, snapshot),
              ),
              const SizedBox(height: 16),
              SectionCard(
                eyebrow: 'Signals',
                title: 'Runtime Signals',
                child: _buildSignalBand(context, snapshot),
              ),
              const SizedBox(height: 16),
              LayoutBuilder(
                builder: (context, constraints) {
                  if (constraints.maxWidth >= 1040) {
                    return _desktopGrid(context, controller, snapshot);
                  }

                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _buildHealthCard(context, controller, snapshot),
                      const SizedBox(height: 12),
                      _buildConnectionCard(context, controller, snapshot),
                      const SizedBox(height: 12),
                      _buildActivityCard(context, controller, snapshot),
                      const SizedBox(height: 12),
                      _buildImportCard(context, controller, snapshot),
                      const SizedBox(height: 12),
                      SectionCard(
                        eyebrow: 'Workbench',
                        title: 'Workflow Access',
                        child: _workflowAccessView(context),
                      ),
                    ],
                  );
                },
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _desktopGrid(
    BuildContext context,
    ClientHomeController controller,
    DashboardSnapshot snapshot,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _alignedCardRow(
          _buildHealthCard(context, controller, snapshot, fillBody: true),
          _buildConnectionCard(context, controller, snapshot),
          height: 306,
        ),
        const SizedBox(height: 12),
        _alignedCardRow(
          _buildActivityCard(context, controller, snapshot, fillBody: true),
          _buildImportCard(context, controller, snapshot, fillBody: true),
          height: 326,
        ),
        const SizedBox(height: 12),
        SectionCard(
          eyebrow: 'Workbench',
          title: 'Workflow Access',
          child: _workflowAccessView(context, compact: true),
        ),
      ],
    );
  }

  Widget _alignedCardRow(Widget left, Widget right, {required double height}) {
    return SizedBox(
      height: height,
      child: Row(
        children: [
          Expanded(child: SizedBox.expand(child: left)),
          const SizedBox(width: 12),
          Expanded(child: SizedBox.expand(child: right)),
        ],
      ),
    );
  }

  Widget _buildControlCenter(
    BuildContext context,
    ClientHomeController controller,
    DashboardSnapshot snapshot,
  ) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final showInlineProxy = constraints.maxWidth >= 1080;
        final actionArea = Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                FilledButton.icon(
                  onPressed: snapshot.busy ? null : controller.startProxy,
                  icon: const Icon(Icons.play_arrow),
                  label: const Text('Start proxy'),
                ),
                OutlinedButton.icon(
                  onPressed: snapshot.busy ? null : controller.stopProxy,
                  icon: const Icon(Icons.stop),
                  label: const Text('Stop'),
                ),
                OutlinedButton.icon(
                  onPressed: snapshot.busy ? null : controller.refreshStatus,
                  icon: const Icon(Icons.refresh),
                  label: const Text('Refresh'),
                ),
              ],
            ),
            const SizedBox(height: 14),
            Text('Capabilities', style: Theme.of(context).textTheme.labelLarge),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _capabilityChip(context, 'TUN', snapshot.tun.disabledReason),
                _capabilityChip(
                  context,
                  'Mode',
                  snapshot.agentModeDisabledReason,
                ),
                _capabilityChip(
                  context,
                  'Script',
                  snapshot.scriptSelection.disabledReason,
                ),
              ],
            ),
          ],
        );
        final systemProxyCard = _controlPill(
          context,
          'System Proxy',
          snapshot.systemProxy.supported
              ? snapshot.systemProxy.enabled
                    ? 'Enabled'
                    : 'Disabled'
              : 'Unsupported',
          snapshot.systemProxy.supported
              ? Colors.teal.shade700
              : Theme.of(context).colorScheme.error,
          reason: snapshot.systemProxy.disabledReason,
          actions: snapshot.systemProxy.supported
              ? [
                  TextButton(
                    onPressed: snapshot.busy
                        ? null
                        : controller.enableSystemProxy,
                    child: const Text('Enable'),
                  ),
                  TextButton(
                    onPressed: snapshot.busy
                        ? null
                        : controller.disableSystemProxy,
                    child: const Text('Disable'),
                  ),
                ]
              : const [],
        );

        if (!showInlineProxy) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [actionArea, const SizedBox(height: 12), systemProxyCard],
          );
        }

        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(flex: 10, child: actionArea),
            const SizedBox(width: 16),
            Expanded(flex: 7, child: systemProxyCard),
          ],
        );
      },
    );
  }

  Widget _buildSignalBand(BuildContext context, DashboardSnapshot snapshot) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth >= 980) {
          return Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: TrendSummaryTile(
                  label: 'Connection activity',
                  valueText:
                      '${snapshot.stats['active_connections'] ?? 0} active',
                  series: snapshot.signalSnapshot.activeConnectionsSeries,
                  emptyText: 'Waiting for runtime samples',
                  tone: DashboardSignalTone.healthy,
                  chartHeight: 84,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: TrendSummaryTile(
                  label: 'Transfer movement',
                  valueText:
                      '${formatSignalBytes((snapshot.stats['bytes_downloaded'] as num?) ?? 0)} down',
                  series: snapshot.signalSnapshot.downloadedBytesSeries,
                  emptyText: 'Waiting for runtime samples',
                  tone: DashboardSignalTone.accent,
                  chartHeight: 84,
                ),
              ),
              const SizedBox(width: 12),
              SizedBox(width: 300, child: _eventLaneSurface(context, snapshot)),
            ],
          );
        }

        final itemWidth = _tileWidthFor(
          constraints.maxWidth,
          minWidth: 280,
          gap: 12,
        );
        return Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            SizedBox(
              width: itemWidth,
              child: TrendSummaryTile(
                label: 'Connection activity',
                valueText:
                    '${snapshot.stats['active_connections'] ?? 0} active',
                series: snapshot.signalSnapshot.activeConnectionsSeries,
                emptyText: 'Waiting for runtime samples',
                tone: DashboardSignalTone.healthy,
                chartHeight: 72,
              ),
            ),
            SizedBox(
              width: itemWidth,
              child: TrendSummaryTile(
                label: 'Transfer movement',
                valueText:
                    '${formatSignalBytes((snapshot.stats['bytes_downloaded'] as num?) ?? 0)} down',
                series: snapshot.signalSnapshot.downloadedBytesSeries,
                emptyText: 'Waiting for runtime samples',
                tone: DashboardSignalTone.accent,
                chartHeight: 72,
              ),
            ),
            SizedBox(
              width: itemWidth,
              child: _eventLaneSurface(context, snapshot),
            ),
          ],
        );
      },
    );
  }

  Widget _eventLaneSurface(BuildContext context, DashboardSnapshot snapshot) {
    return Container(
      padding: const EdgeInsets.all(14),
      constraints: const BoxConstraints(minHeight: 154),
      decoration: BoxDecoration(
        color: const Color(0xFFF4F1EA),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: const Color(0xFFDCD5CA)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Probe / error lane',
            style: Theme.of(context).textTheme.labelMedium,
          ),
          const SizedBox(height: 8),
          SignalEventLane(
            events: snapshot.signalSnapshot.recentProbeOutcomes,
            emptyText: 'Waiting for runtime samples',
          ),
        ],
      ),
    );
  }

  Widget _buildHealthCard(
    BuildContext context,
    ClientHomeController controller,
    DashboardSnapshot snapshot, {
    bool fillBody = false,
  }) {
    return SectionCard(
      eyebrow: 'Runtime',
      title: 'Health',
      fillBody: fillBody,
      trailing: _openButton(
        context,
        'Diagnostics',
        () => controller.openRoute(HomeRoute.runtime),
      ),
      child: _interactiveCardBody(
        context,
        previewHeight: 128,
        fillAvailableHeight: fillBody,
        onShowDetails: () => _showDetailsDialog(
          context,
          title: 'Health',
          actionLabel: 'Diagnostics',
          onAction: () => controller.openRoute(HomeRoute.runtime),
          child: HealthSummaryView(
            running: snapshot.running,
            stats: snapshot.stats,
            lastProbe: snapshot.healthSnapshot,
            lastError: snapshot.lastError,
            signalSnapshot: snapshot.signalSnapshot,
            expanded: true,
          ),
        ),
        child: HealthSummaryView(
          running: snapshot.running,
          stats: snapshot.stats,
          lastProbe: snapshot.healthSnapshot,
          lastError: snapshot.lastError,
          signalSnapshot: snapshot.signalSnapshot,
          expanded: false,
        ),
      ),
    );
  }

  Widget _buildConnectionCard(
    BuildContext context,
    ClientHomeController controller,
    DashboardSnapshot snapshot,
  ) {
    return SectionCard(
      eyebrow: 'Runtime',
      title: 'Connection Manager',
      trailing: _openButton(
        context,
        'Diagnostics',
        () => controller.openRoute(HomeRoute.runtime),
      ),
      child: _connectionManagerView(context, snapshot),
    );
  }

  Widget _buildActivityCard(
    BuildContext context,
    ClientHomeController controller,
    DashboardSnapshot snapshot, {
    bool fillBody = false,
  }) {
    return SectionCard(
      eyebrow: 'Signal',
      title: 'Recent Activity',
      fillBody: fillBody,
      trailing: _openButton(
        context,
        'All activity',
        () => controller.openRoute(HomeRoute.runtime),
      ),
      child: _interactiveCardBody(
        context,
        previewHeight: 134,
        fillAvailableHeight: fillBody,
        onShowDetails: () => _showDetailsDialog(
          context,
          title: 'Recent Activity',
          actionLabel: 'All activity',
          onAction: () => controller.openRoute(HomeRoute.runtime),
          child: ActivityLogView(
            entries: snapshot.activityEntries.take(16).toList(),
            maxEntries: 10,
            showPreviewHint: false,
          ),
        ),
        child: ActivityLogView(
          entries: snapshot.activityEntries.take(6).toList(),
          maxEntries: 2,
          showPreviewHint: true,
        ),
      ),
    );
  }

  Widget _buildImportCard(
    BuildContext context,
    ClientHomeController controller,
    DashboardSnapshot snapshot, {
    bool fillBody = false,
  }) {
    return SectionCard(
      eyebrow: 'Import',
      title: 'Import / Support State',
      fillBody: fillBody,
      trailing: _openButton(
        context,
        'Open import',
        () => controller.openRoute(HomeRoute.importView),
      ),
      child: _interactiveCardBody(
        context,
        previewHeight: 96,
        fillAvailableHeight: fillBody,
        onShowDetails: () => _showDetailsDialog(
          context,
          title: 'Import / Support State',
          actionLabel: 'Open import',
          onAction: () => controller.openRoute(HomeRoute.importView),
          child: _importSummaryView(context, snapshot, expanded: true),
        ),
        child: Align(
          alignment: Alignment.topLeft,
          child: _importSummaryView(context, snapshot, expanded: false),
        ),
      ),
    );
  }

  Widget _clippedPreview({required double height, required Widget child}) {
    return SizedBox(
      height: height,
      child: ClipRect(child: child),
    );
  }

  Widget _detailActionBar(
    BuildContext context, {
    required VoidCallback onShowDetails,
  }) {
    return Material(
      color: const Color(0xFFF4F1EA),
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: onShowDetails,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.open_in_full, size: 17),
              const SizedBox(width: 6),
              Text(
                'Show details',
                style: Theme.of(context).textTheme.labelLarge,
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _connectionManagerView(
    BuildContext context,
    DashboardSnapshot snapshot,
  ) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final metricWidth = _tileWidthFor(
          constraints.maxWidth,
          minWidth: 150,
          gap: 12,
          maxColumns: 3,
        );
        return Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            SizedBox(
              width: metricWidth,
              child: _summarySurface(
                context,
                'Total',
                '${snapshot.stats['total_connections'] ?? 0}',
              ),
            ),
            SizedBox(
              width: metricWidth,
              child: _summarySurface(
                context,
                'Uploaded',
                formatSignalBytes(
                  (snapshot.stats['bytes_uploaded'] as num?) ?? 0,
                ),
              ),
            ),
            SizedBox(
              width: metricWidth,
              child: _summarySurface(
                context,
                'Downloaded',
                formatSignalBytes(
                  (snapshot.stats['bytes_downloaded'] as num?) ?? 0,
                ),
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _summaryMetric(BuildContext context, String label, String value) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: Theme.of(context).textTheme.labelMedium),
        const SizedBox(height: 4),
        Text(value, style: Theme.of(context).textTheme.titleMedium),
      ],
    );
  }

  Widget _summarySurface(BuildContext context, String label, String value) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xFFF4F1EA),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: const Color(0xFFDCD5CA)),
      ),
      child: _summaryMetric(context, label, value),
    );
  }

  Widget _importSummaryView(
    BuildContext context,
    DashboardSnapshot snapshot, {
    required bool expanded,
  }) {
    final summary = snapshot.importSummary.trim();
    final empty = summary.toLowerCase().contains(
      'no wrongsv import inspected yet',
    );
    final text = empty
        ? 'No wrongsv import inspected yet. Open Import when you need to inspect or adapt an existing config.'
        : summary;
    return Text(
      text,
      style: Theme.of(context).textTheme.bodyMedium,
      maxLines: expanded ? 8 : 3,
      overflow: TextOverflow.ellipsis,
    );
  }

  Widget _workflowAccessView(BuildContext context, {bool compact = false}) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (!compact) ...[
          Text(
            'Keep the control surface focused. Open management panels when you need them, and enter heavier modes only for deeper editing or diagnostics.',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(height: 12),
        ],
        Text('Secondary panels', style: Theme.of(context).textTheme.labelLarge),
        const SizedBox(height: 8),
        Wrap(
          spacing: 10,
          runSpacing: 10,
          children: [
            _workflowButton(
              icon: Icons.folder_copy_outlined,
              label: 'Profiles',
              onTap: () => widget.controller.openRoute(HomeRoute.profiles),
            ),
            _workflowButton(
              icon: Icons.sync_alt,
              label: 'Import',
              onTap: () => widget.controller.openRoute(HomeRoute.importView),
            ),
            _workflowButton(
              icon: Icons.settings,
              label: 'Settings',
              onTap: () => widget.controller.openRoute(HomeRoute.settings),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Text(
          'Focused work modes',
          style: Theme.of(context).textTheme.labelLarge,
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 10,
          runSpacing: 10,
          children: [
            _workflowButton(
              icon: Icons.tune,
              label: 'Editor',
              emphasized: true,
              onTap: () => widget.controller.openRoute(HomeRoute.editor),
            ),
            _workflowButton(
              icon: Icons.terminal,
              label: 'Diagnostics',
              emphasized: true,
              onTap: () => widget.controller.openRoute(HomeRoute.runtime),
            ),
          ],
        ),
        const SizedBox(height: 10),
        Text(
          compact
              ? 'Use panels for management and focused modes for deeper edits.'
              : 'These tools stay secondary so runtime status and control remain visible first.',
          style: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(color: const Color(0xFF5F6B73)),
        ),
      ],
    );
  }

  Widget _openButton(BuildContext context, String label, VoidCallback onTap) {
    return TextButton(onPressed: onTap, child: Text(label));
  }

  Widget _interactiveCardBody(
    BuildContext context, {
    required double previewHeight,
    required bool fillAvailableHeight,
    required VoidCallback onShowDetails,
    required Widget child,
  }) {
    if (fillAvailableHeight) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: ClipRect(
              child: Align(alignment: Alignment.topLeft, child: child),
            ),
          ),
          const SizedBox(height: 12),
          _detailActionBar(context, onShowDetails: onShowDetails),
        ],
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _clippedPreview(height: previewHeight, child: child),
        const SizedBox(height: 12),
        _detailActionBar(context, onShowDetails: onShowDetails),
      ],
    );
  }

  Future<void> _showDetailsDialog(
    BuildContext context, {
    required String title,
    required Widget child,
    required String actionLabel,
    required VoidCallback onAction,
  }) {
    return showDialog<void>(
      context: context,
      builder: (dialogContext) {
        return Dialog(
          insetPadding: const EdgeInsets.symmetric(
            horizontal: 48,
            vertical: 40,
          ),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 760, maxHeight: 640),
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          title,
                          style: Theme.of(context).textTheme.titleLarge,
                        ),
                      ),
                      TextButton(
                        onPressed: () {
                          Navigator.of(dialogContext).pop();
                          onAction();
                        },
                        child: Text(actionLabel),
                      ),
                      IconButton(
                        tooltip: 'Close',
                        onPressed: () => Navigator.of(dialogContext).pop(),
                        icon: const Icon(Icons.close),
                      ),
                    ],
                  ),
                  const Divider(height: 20),
                  Flexible(child: SingleChildScrollView(child: child)),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _workflowButton({
    required IconData icon,
    required String label,
    required VoidCallback onTap,
    bool emphasized = false,
  }) {
    if (emphasized) {
      return FilledButton.icon(
        onPressed: onTap,
        icon: Icon(icon),
        label: Text(label),
      );
    }
    return OutlinedButton.icon(
      onPressed: onTap,
      icon: Icon(icon),
      label: Text(label),
    );
  }

  Widget _controlPill(
    BuildContext context,
    String title,
    String value,
    Color color, {
    String reason = '',
    List<Widget> actions = const [],
  }) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: color.withAlpha(14),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: color.withAlpha(46)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: Theme.of(context).textTheme.labelLarge),
          const SizedBox(height: 4),
          Text(
            value,
            style: TextStyle(color: color, fontWeight: FontWeight.w700),
          ),
          if (reason.isNotEmpty) ...[
            const SizedBox(height: 6),
            Text(reason, style: Theme.of(context).textTheme.bodySmall),
          ],
          if (actions.isNotEmpty) ...[
            const SizedBox(height: 6),
            Wrap(spacing: 8, children: actions),
          ],
        ],
      ),
    );
  }

  double _tileWidthFor(
    double availableWidth, {
    required double minWidth,
    required double gap,
    int maxColumns = 3,
  }) {
    for (var columns = maxColumns; columns >= 1; columns--) {
      final width = (availableWidth - gap * (columns - 1)) / columns;
      if (width >= minWidth) {
        return width;
      }
    }
    return availableWidth;
  }

  Widget _capabilityChip(BuildContext context, String label, String reason) {
    return Tooltip(
      message: reason,
      child: Container(
        constraints: const BoxConstraints(minWidth: 130),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.error.withAlpha(10),
          borderRadius: BorderRadius.circular(999),
          border: Border.all(
            color: Theme.of(context).colorScheme.error.withAlpha(36),
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(label, style: Theme.of(context).textTheme.labelLarge),
            const SizedBox(width: 8),
            Text(
              'Unsupported',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.error,
                fontWeight: FontWeight.w700,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
