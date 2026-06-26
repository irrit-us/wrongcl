import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../control_state.dart';
import '../l10n/app_localizations.dart';
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
    final l10n = AppLocalizations.of(context);
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
            label: l10n.trafficUp,
            peak: _peakRate(upRates),
            recentAverage: _recentAverageRate(
              uploadSeries.points,
              _averageWindow,
            ),
            total: bytesUploaded.toDouble(),
          ),
          Divider(height: 1, color: palette.border.subtle),
          _StatBlock(
            label: l10n.trafficDown,
            peak: _peakRate(downRates),
            recentAverage: _recentAverageRate(
              downloadSeries.points,
              _averageWindow,
            ),
            total: bytesDownloaded.toDouble(),
          ),
        ],
      ),
    );
  }

  static const Duration _averageWindow = Duration(minutes: 1);

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

  static double _recentAverageRate(
    List<DashboardSeriesPoint> points,
    Duration window,
  ) {
    if (points.length < 2) return 0;
    final cutoff = points.last.timestamp.subtract(window);
    var totalDelta = 0.0;
    var totalSeconds = 0.0;
    for (var i = 1; i < points.length; i++) {
      final a = points[i - 1];
      final b = points[i];
      if (b.timestamp.isBefore(cutoff)) continue;
      final dt = b.timestamp.difference(a.timestamp).inMilliseconds / 1000;
      if (dt <= 0) continue;
      final delta = b.value - a.value;
      if (delta <= 0) continue;
      totalDelta += delta;
      totalSeconds += dt;
    }
    if (totalSeconds <= 0) return 0;
    return totalDelta / totalSeconds;
  }
}

class _StatBlock extends StatelessWidget {
  const _StatBlock({
    required this.label,
    required this.peak,
    required this.recentAverage,
    required this.total,
  });

  final String label;
  final double peak;
  final double recentAverage;
  final double total;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
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
          name: l10n.trafficPeak,
          value: '${formatSignalBytes(peak)}/s',
          palette: palette,
        ),
        _StatLine(
          name: l10n.trafficTotal,
          value: formatSignalBytes(total),
          palette: palette,
        ),
        _StatLine(
          name: l10n.trafficAvg1Min,
          value: '${formatSignalBytes(recentAverage)}/s',
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
        Flexible(
          child: Text(
            name,
            style: theme.textTheme.bodySmall?.copyWith(
              color: palette.text.secondary,
            ),
            overflow: TextOverflow.ellipsis,
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
