import 'package:flutter/material.dart';

import 'control_state.dart';
import 'wrongcl_client.dart';

class SectionCard extends StatelessWidget {
  const SectionCard({
    super.key,
    required this.title,
    required this.child,
    this.trailing,
    this.eyebrow,
    this.fillBody = false,
  });

  final String title;
  final Widget child;
  final Widget? trailing;
  final String? eyebrow;
  final bool fillBody;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (eyebrow case final eyebrow?) ...[
              Text(
                eyebrow.toUpperCase(),
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  letterSpacing: 1.2,
                  color: const Color(0xFF5F6B73),
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(height: 8),
            ],
            Row(
              children: [
                Expanded(
                  child: Text(
                    title,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                if (trailing != null) ...[trailing!],
              ],
            ),
            const SizedBox(height: 12),
            if (fillBody) Expanded(child: child) else child,
          ],
        ),
      ),
    );
  }
}

class PanelIntroCard extends StatelessWidget {
  const PanelIntroCard({
    super.key,
    required this.title,
    required this.description,
    this.badges = const [],
  });

  final String title;
  final String description;
  final List<Widget> badges;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        gradient: const LinearGradient(
          colors: [Color(0xFFFBFAF7), Color(0xFFF0ECE3)],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: const Color(0xFFD8D1C5)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 8),
          Text(description, style: Theme.of(context).textTheme.bodyMedium),
          if (badges.isNotEmpty) ...[
            const SizedBox(height: 14),
            Wrap(spacing: 10, runSpacing: 10, children: badges),
          ],
        ],
      ),
    );
  }
}

class InfoBadge extends StatelessWidget {
  const InfoBadge({
    super.key,
    required this.label,
    required this.value,
    this.tone = const Color(0xFF2F4858),
  });

  final String label;
  final String value;
  final Color tone;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: tone.withAlpha(18),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: tone.withAlpha(42)),
      ),
      child: RichText(
        text: TextSpan(
          style: Theme.of(context).textTheme.bodySmall?.copyWith(color: tone),
          children: [
            TextSpan(text: '$label: '),
            TextSpan(
              text: value,
              style: const TextStyle(fontWeight: FontWeight.w700),
            ),
          ],
        ),
      ),
    );
  }
}

class NoticeCard extends StatelessWidget {
  const NoticeCard({
    super.key,
    required this.title,
    required this.message,
    this.tone = const Color(0xFF7A5C1E),
  });

  final String title;
  final String message;
  final Color tone;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: tone.withAlpha(12),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: tone.withAlpha(46)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: Theme.of(context).textTheme.titleSmall?.copyWith(color: tone),
          ),
          const SizedBox(height: 6),
          Text(message, style: Theme.of(context).textTheme.bodySmall),
        ],
      ),
    );
  }
}

class StatusBar extends StatelessWidget {
  const StatusBar({
    super.key,
    required this.running,
    required this.busy,
    required this.status,
    required this.stackSummary,
    required this.nativeInfo,
  });

  final bool running;
  final bool busy;
  final String status;
  final String stackSummary;
  final String nativeInfo;

  @override
  Widget build(BuildContext context) {
    final lowerStatus = status.toLowerCase();
    final failed = lowerStatus.contains('failed') || lowerStatus.contains('error');
    final color = failed
        ? Theme.of(context).colorScheme.error
        : running
            ? const Color(0xFF0B8A6E)
            : const Color(0xFF616161);
    final icon = failed
        ? Icons.error_outline
        : running
            ? Icons.check_circle
            : Icons.radio_button_unchecked;
    return Container(
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        gradient: const LinearGradient(
          colors: [Color(0xFFFBFAF7), Color(0xFFF1EEE7)],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        borderRadius: BorderRadius.circular(24),
        border: Border.all(color: const Color(0xFFD8D1C5)),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 54,
            height: 54,
            decoration: BoxDecoration(
              color: const Color(0xFF111111),
              borderRadius: BorderRadius.circular(18),
            ),
            child: const Icon(Icons.show_chart, color: Colors.white),
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Operations Deck',
                  style: Theme.of(context).textTheme.labelLarge?.copyWith(
                    letterSpacing: 1.1,
                    color: const Color(0xFF5F6B73),
                  ),
                ),
                const SizedBox(height: 6),
                Row(
                  children: [
                    Icon(icon, color: color),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        busy ? 'Working...' : status,
                        style: Theme.of(context).textTheme.headlineSmall,
                      ),
                    ),
                  ],
                ),
                if (stackSummary.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Text(
                    'Stack: $stackSummary',
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                ],
                const SizedBox(height: 8),
                Text(
                  nativeInfo,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: const Color(0xFF5F6B73),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class StatsGrid extends StatelessWidget {
  const StatsGrid({super.key, required this.stats});

  final Map<String, Object?> stats;

  @override
  Widget build(BuildContext context) {
    final items = [
      ('Running', stats['running'] == true ? 'yes' : 'no'),
      ('Active', '${stats['active_connections'] ?? 0}'),
      ('Total', '${stats['total_connections'] ?? 0}'),
      ('Failed', '${stats['failed_connections'] ?? 0}'),
      ('Uploaded', '${stats['bytes_uploaded'] ?? 0} B'),
      ('Downloaded', '${stats['bytes_downloaded'] ?? 0} B'),
    ];

    return Wrap(
      spacing: 14,
      runSpacing: 14,
      children: [
        for (final item in items)
          Container(
            width: 152,
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: const Color(0xFFF4F1EA),
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: const Color(0xFFDCD5CA)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(item.$1, style: Theme.of(context).textTheme.labelMedium),
                const SizedBox(height: 6),
                Text(item.$2, style: Theme.of(context).textTheme.titleMedium),
              ],
            ),
          ),
      ],
    );
  }
}

class ActivityLogView extends StatelessWidget {
  const ActivityLogView({
    super.key,
    required this.entries,
    this.maxEntries = 3,
    this.showPreviewHint = true,
  });

  final List<DashboardActivityEntry> entries;
  final int maxEntries;
  final bool showPreviewHint;

  String _formatTime(DateTime value) {
    final local = value.toLocal();
    final hour = local.hour.toString().padLeft(2, '0');
    final minute = local.minute.toString().padLeft(2, '0');
    final second = local.second.toString().padLeft(2, '0');
    return '$hour:$minute:$second';
  }

  @override
  Widget build(BuildContext context) {
    if (entries.isEmpty) {
      return Text(
        'No activity yet',
        style: Theme.of(context).textTheme.bodySmall,
      );
    }

    final groupedEntries = <({DashboardActivityEntry entry, int count})>[];
    for (final entry in entries) {
      if (groupedEntries.isNotEmpty &&
          groupedEntries.last.entry.title == entry.title &&
          groupedEntries.last.entry.detail == entry.detail &&
          groupedEntries.last.entry.success == entry.success) {
        final last = groupedEntries.removeLast();
        groupedEntries.add((entry: last.entry, count: last.count + 1));
      } else {
        groupedEntries.add((entry: entry, count: 1));
      }
    }

    final previewEntries = groupedEntries.take(maxEntries).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (showPreviewHint && groupedEntries.length > previewEntries.length)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Text(
              'Showing latest ${previewEntries.length} grouped events',
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: const Color(0xFF5F6B73),
              ),
            ),
          ),
        for (final grouped in previewEntries)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(
                  grouped.entry.success
                      ? Icons.check_circle_outline
                      : Icons.error_outline,
                  size: 16,
                  color: grouped.entry.success
                      ? Colors.green.shade700
                      : Theme.of(context).colorScheme.error,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        grouped.entry.title,
                        style: Theme.of(context).textTheme.labelLarge,
                      ),
                      if (grouped.count > 1)
                        Text(
                          '${grouped.count} repeated events',
                          style: Theme.of(context).textTheme.labelSmall,
                        ),
                      const SizedBox(height: 2),
                      Text(
                        grouped.entry.detail,
                        style: Theme.of(context).textTheme.bodySmall,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  _formatTime(grouped.entry.timestamp),
                  style: Theme.of(context).textTheme.labelSmall,
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class WrongsvReportView extends StatelessWidget {
  const WrongsvReportView({super.key, required this.report, this.stackSummary});

  final WrongsvCapabilityReport report;
  final String? stackSummary;

  Color _supportColor(BuildContext context) {
    switch (report.activeSupport) {
      case 'supported':
        return Colors.green.shade700;
      case 'partial':
        return Colors.orange.shade800;
      default:
        return Theme.of(context).colorScheme.error;
    }
  }

  @override
  Widget build(BuildContext context) {
    final activeProfiles = report.profiles.where((profile) => profile.active).toList();
    final WrongsvProfileSupport? activeProfile = activeProfiles.isEmpty
        ? null
        : activeProfiles.first;
    final previewProfiles = report.profiles.take(6).toList();

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.grey.shade50,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.grey.shade300),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _ReportChip(
                label: 'Active profile',
                value: report.activeProfile,
                color: Colors.blueGrey.shade700,
              ),
              _ReportChip(
                label: 'Support',
                value: report.activeSupport,
                color: _supportColor(context),
              ),
              _ReportChip(
                label: 'Payloads',
                value: report.payloadNetworks.join(', '),
                color: Colors.blue.shade700,
              ),
              _ReportChip(
                label: 'Carrier',
                value: report.baseCarriers.join(', '),
                color: Colors.teal.shade700,
              ),
            ],
          ),
          const SizedBox(height: 12),
          Text(report.activeReason, style: Theme.of(context).textTheme.bodyMedium),
          if (stackSummary != null && stackSummary!.isNotEmpty) ...[
            const SizedBox(height: 8),
            Text(
              'Adapted stack: $stackSummary',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
          if (report.missingFields.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(
              'Missing client-side fields',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 6),
            for (final field in report.missingFields)
              Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: Text(
                  '${field.field}: ${field.reason}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
          ],
          if (activeProfile != null &&
              activeProfile.reason.isNotEmpty &&
              activeProfile.reason != report.activeReason) ...[
            const SizedBox(height: 12),
            Text(
              'Profile note: ${activeProfile.reason}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
          if (previewProfiles.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(
              'Recognized profiles',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 6),
            for (final profile in previewProfiles)
              Padding(
                padding: const EdgeInsets.only(bottom: 4),
                child: Text(
                  '${profile.displayName}: ${profile.support}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
          ],
        ],
      ),
    );
  }
}

class ResultView extends StatelessWidget {
  const ResultView({super.key, required this.response});

  final NativeResponse response;

  @override
  Widget build(BuildContext context) {
    final buffer = StringBuffer(response.message);
    if (response.data.isNotEmpty) {
      for (final entry in response.data.entries) {
        buffer.writeln();
        buffer.write('${entry.key}: ${entry.value}');
      }
    }
    return SelectableText(
      buffer.toString(),
      style: const TextStyle(fontFamily: 'monospace'),
    );
  }
}

class _ReportChip extends StatelessWidget {
  const _ReportChip({
    required this.label,
    required this.value,
    required this.color,
  });

  final String label;
  final String value;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
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
}
