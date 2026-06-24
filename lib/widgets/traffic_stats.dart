import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../control_state.dart';
import '../signal_widgets.dart';
import '../theme/wrongcl_colors.dart';

class TrafficStats extends StatelessWidget {
  const TrafficStats({
    super.key,
    required this.bytesUploaded,
    required this.bytesDownloaded,
    required this.uploadSeries,
    required this.downloadSeries,
  });

  final int bytesUploaded;
  final int bytesDownloaded;
  final DashboardTrendSeries uploadSeries;
  final DashboardTrendSeries downloadSeries;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final upRates = _rateSeries(uploadSeries.points);
    final downRates = _rateSeries(downloadSeries.points);
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: palette.surface.surfaceRaised,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: palette.border.regular),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisAlignment: MainAxisAlignment.spaceEvenly,
        children: [
          _StatBlock(
            label: 'Up',
            peak: _peakRate(upRates),
            average: _averageRate(upRates),
            total: bytesUploaded.toDouble(),
          ),
          Divider(height: 1, color: palette.border.subtle),
          _StatBlock(
            label: 'Down',
            peak: _peakRate(downRates),
            average: _averageRate(downRates),
            total: bytesDownloaded.toDouble(),
          ),
        ],
      ),
    );
  }

  static List<double> _rateSeries(List<DashboardSeriesPoint> points) {
    if (points.length < 2) {
      return const [];
    }
    final rates = <double>[];
    for (var i = 1; i < points.length; i++) {
      final a = points[i - 1];
      final b = points[i];
      final dt = b.timestamp.difference(a.timestamp).inMilliseconds / 1000;
      if (dt <= 0) {
        rates.add(0);
      } else {
        final delta = b.value - a.value;
        rates.add(delta < 0 ? 0 : delta / dt);
      }
    }
    return rates;
  }

  static double _peakRate(List<double> rates) {
    if (rates.isEmpty) return 0;
    return rates.fold<double>(0, math.max);
  }

  static double _averageRate(List<double> rates) {
    if (rates.isEmpty) return 0;
    final sum = rates.fold<double>(0, (acc, v) => acc + v);
    return sum / rates.length;
  }
}

class _StatBlock extends StatelessWidget {
  const _StatBlock({
    required this.label,
    required this.peak,
    required this.average,
    required this.total,
  });

  final String label;
  final double peak;
  final double average;
  final double total;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          label,
          textAlign: TextAlign.center,
          style: theme.textTheme.labelSmall,
        ),
        const SizedBox(height: 4),
        _StatLine(
          name: 'Peak',
          value: '${formatSignalBytes(peak)}/s',
          palette: palette,
        ),
        _StatLine(
          name: 'Avg',
          value: '${formatSignalBytes(average)}/s',
          palette: palette,
        ),
        _StatLine(
          name: 'Total',
          value: formatSignalBytes(total),
          palette: palette,
        ),
      ],
    );
  }
}

class _StatLine extends StatelessWidget {
  const _StatLine({
    required this.name,
    required this.value,
    required this.palette,
  });

  final String name;
  final String value;
  final WrongclColors palette;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          name,
          style: theme.textTheme.bodySmall?.copyWith(
            color: palette.text.secondary,
          ),
        ),
        Text(
          value,
          style: theme.textTheme.bodyMedium,
        ),
      ],
    );
  }
}
