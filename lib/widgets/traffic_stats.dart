import 'package:flutter/material.dart';

import '../signal_widgets.dart';

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
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFFFBFAF7),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: const Color(0xFFDCD5CA)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _StatRow(
            label: 'Up',
            total: formatSignalBytes(bytesUploaded),
            rate: '${formatSignalBytes(upRatePerSecond)}/s',
          ),
          const SizedBox(height: 14),
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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(label, style: Theme.of(context).textTheme.labelSmall),
        const SizedBox(height: 2),
        Text(total, style: Theme.of(context).textTheme.titleMedium),
        Text(
          'rate $rate',
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: const Color(0xFF8B8579),
          ),
        ),
      ],
    );
  }
}
