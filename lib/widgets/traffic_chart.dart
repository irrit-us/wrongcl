import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../control_state.dart';
import '../l10n/app_localizations.dart';
import '../signal_widgets.dart';
import '../theme/wrongcl_colors.dart';

class TrafficChart extends StatelessWidget {
  const TrafficChart({
    super.key,
    required this.uploadSeries,
    required this.downloadSeries,
  });

  final DashboardTrendSeries uploadSeries;
  final DashboardTrendSeries downloadSeries;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final l10n = AppLocalizations.of(context);
    final upRate = _latestRate(uploadSeries.points);
    final downRate = _latestRate(downloadSeries.points);
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: palette.surface.surfaceRaised,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: palette.border.regular),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _ChartRow(
            label: l10n.trafficUp,
            valueText: '${formatSignalBytes(upRate)}/s',
            color: palette.chart.upload,
            gridColor: palette.chart.grid,
            points: uploadSeries.points,
          ),
          const SizedBox(height: 8),
          _ChartRow(
            label: l10n.trafficDown,
            valueText: '${formatSignalBytes(downRate)}/s',
            color: palette.chart.download,
            gridColor: palette.chart.grid,
            points: downloadSeries.points,
          ),
        ],
      ),
    );
  }

  double _latestRate(List<DashboardSeriesPoint> points) {
    if (points.length < 2) {
      return 0;
    }
    final a = points[points.length - 2];
    final b = points[points.length - 1];
    final dtSeconds = b.timestamp.difference(a.timestamp).inMilliseconds / 1000;
    if (dtSeconds <= 0) {
      return 0;
    }
    final dValue = b.value - a.value;
    if (dValue < 0) {
      return 0;
    }
    return dValue / dtSeconds;
  }
}

class _ChartRow extends StatelessWidget {
  const _ChartRow({
    required this.label,
    required this.valueText,
    required this.color,
    required this.gridColor,
    required this.points,
  });

  final String label;
  final String valueText;
  final Color color;
  final Color gridColor;
  final List<DashboardSeriesPoint> points;

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                label,
                style: Theme.of(context).textTheme.labelSmall,
              ),
              const SizedBox(width: 8),
              Text(
                valueText,
                style: Theme.of(context)
                    .textTheme
                    .titleMedium
                    ?.copyWith(color: color),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Expanded(
            child: CustomPaint(
              size: Size.infinite,
              painter: _TrafficLinePainter(
                points: points,
                color: color,
                gridColor: gridColor,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _TrafficLinePainter extends CustomPainter {
  _TrafficLinePainter({
    required this.points,
    required this.color,
    required this.gridColor,
  });

  final List<DashboardSeriesPoint> points;
  final Color color;
  final Color gridColor;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.height <= 0 || size.width <= 0) {
      return;
    }
    final rates = _rateSeries(points);
    final gridPaint = Paint()
      ..color = gridColor
      ..strokeWidth = 1;
    canvas.drawLine(
      Offset(0, size.height - 0.5),
      Offset(size.width, size.height - 0.5),
      gridPaint,
    );
    if (rates.length < 2) {
      return;
    }
    final maxRate = rates.fold<double>(0, math.max);
    final denom = maxRate <= 0 ? 1 : maxRate;
    final stepX = size.width / (rates.length - 1);

    final linePath = Path();
    final fillPath = Path();
    for (var i = 0; i < rates.length; i++) {
      final x = i * stepX;
      final normalized = rates[i] / denom;
      final y = size.height - (normalized * (size.height - 2)) - 1;
      if (i == 0) {
        linePath.moveTo(x, y);
        fillPath.moveTo(x, size.height);
        fillPath.lineTo(x, y);
      } else {
        linePath.lineTo(x, y);
        fillPath.lineTo(x, y);
      }
    }
    fillPath.lineTo(size.width, size.height);
    fillPath.close();

    final fillPaint = Paint()
      ..shader = LinearGradient(
        colors: [color.withAlpha(64), color.withAlpha(0)],
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
      ).createShader(Rect.fromLTWH(0, 0, size.width, size.height));
    canvas.drawPath(fillPath, fillPaint);

    final linePaint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.8
      ..strokeCap = StrokeCap.round;
    canvas.drawPath(linePath, linePaint);
  }

  List<double> _rateSeries(List<DashboardSeriesPoint> points) {
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

  @override
  bool shouldRepaint(covariant _TrafficLinePainter oldDelegate) {
    return oldDelegate.points != points ||
        oldDelegate.color != color ||
        oldDelegate.gridColor != gridColor;
  }
}
