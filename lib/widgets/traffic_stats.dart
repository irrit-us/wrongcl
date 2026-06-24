import 'package:flutter/material.dart';

import '../signal_widgets.dart';
import '../theme/wrongcl_colors.dart';

class TrafficStats extends StatelessWidget {
  const TrafficStats({
    super.key,
    required this.bytesUploaded,
    required this.bytesDownloaded,
    required this.upRatePerSecond,
    required this.downRatePerSecond,
  });

  final int bytesUploaded;
  final int bytesDownloaded;
  final double upRatePerSecond;
  final double downRatePerSecond;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: palette.surface.surfaceRaised,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: palette.border.regular),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.center,
        mainAxisAlignment: MainAxisAlignment.spaceEvenly,
        children: [
          _StatRow(
            label: 'Up',
            total: formatSignalBytes(bytesUploaded),
            rate: '${formatSignalBytes(upRatePerSecond)}/s',
          ),
          const SizedBox(height: 6),
          Divider(height: 1, color: palette.border.subtle),
          const SizedBox(height: 6),
          _StatRow(
            label: 'Down',
            total: formatSignalBytes(bytesDownloaded),
            rate: '${formatSignalBytes(downRatePerSecond)}/s',
          ),
        ],
      ),
    );
  }
}

class _StatRow extends StatelessWidget {
  const _StatRow({
    required this.label,
    required this.total,
    required this.rate,
  });

  final String label;
  final String total;
  final String rate;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.center,
      mainAxisAlignment: MainAxisAlignment.center,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          label,
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.labelSmall,
        ),
        const SizedBox(height: 2),
        Text(
          total,
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.titleMedium,
        ),
        Text(
          'rate $rate',
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: palette.text.secondary,
          ),
        ),
      ],
    );
  }
}
