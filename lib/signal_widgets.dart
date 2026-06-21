import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'control_state.dart';

String formatSignalNumber(num value) {
  if (value >= 1000000) {
    return '${(value / 1000000).toStringAsFixed(1)}M';
  }
  if (value >= 1000) {
    return '${(value / 1000).toStringAsFixed(1)}K';
  }
  return value.toStringAsFixed(value % 1 == 0 ? 0 : 1);
}

String formatSignalBytes(num bytes) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  var value = bytes.toDouble();
  var unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  final digits = value >= 10 || unitIndex == 0 ? 0 : 1;
  return '${value.toStringAsFixed(digits)} ${units[unitIndex]}';
}

Color signalToneColor(BuildContext context, DashboardSignalTone tone) {
  switch (tone) {
    case DashboardSignalTone.accent:
      return const Color(0xFF2F4858);
    case DashboardSignalTone.healthy:
      return const Color(0xFF0B8A6E);
    case DashboardSignalTone.warning:
      return const Color(0xFF9A6700);
    case DashboardSignalTone.danger:
      return Theme.of(context).colorScheme.error;
    case DashboardSignalTone.neutral:
      return const Color(0xFF616161);
  }
}

class TrendSummaryTile extends StatelessWidget {
  const TrendSummaryTile({
    super.key,
    required this.label,
    required this.valueText,
    required this.series,
    required this.emptyText,
    required this.tone,
    this.chartHeight = 40,
  });

  final String label;
  final String valueText;
  final DashboardTrendSeries series;
  final String emptyText;
  final DashboardSignalTone tone;
  final double chartHeight;

  @override
  Widget build(BuildContext context) {
    final color = signalToneColor(context, tone);
    final hasEnoughPoints = series.points.length >= 2;
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xFFF4F1EA),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: const Color(0xFFDCD5CA)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: Theme.of(context).textTheme.labelMedium),
          const SizedBox(height: 6),
          Text(
            valueText,
            style: Theme.of(context).textTheme.titleLarge?.copyWith(color: color),
          ),
          const SizedBox(height: 10),
          SizedBox(
            height: chartHeight,
            child: hasEnoughPoints
                ? SignalSparkline(series: series, color: color)
                : Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      emptyText,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ),
          ),
        ],
      ),
    );
  }
}

class SignalEventLane extends StatelessWidget {
  const SignalEventLane({
    super.key,
    required this.events,
    required this.emptyText,
  });

  final List<DashboardSignalEvent> events;
  final String emptyText;

  @override
  Widget build(BuildContext context) {
    if (events.isEmpty) {
      return Text(emptyText, style: Theme.of(context).textTheme.bodySmall);
    }
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: [
        for (final event in events.take(10))
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
            decoration: BoxDecoration(
              color: signalToneColor(context, event.tone).withAlpha(18),
              borderRadius: BorderRadius.circular(999),
              border: Border.all(
                color: signalToneColor(context, event.tone).withAlpha(42),
              ),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  event.success ? Icons.check_circle : Icons.error,
                  size: 14,
                  color: signalToneColor(context, event.tone),
                ),
                const SizedBox(width: 6),
                Text(
                  event.label,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: signalToneColor(context, event.tone),
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class SignalSparkline extends StatelessWidget {
  const SignalSparkline({
    super.key,
    required this.series,
    required this.color,
  });

  final DashboardTrendSeries series;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: _SignalSparklinePainter(
        points: series.points,
        color: color,
      ),
      size: const Size(double.infinity, 40),
    );
  }
}

class _SignalSparklinePainter extends CustomPainter {
  const _SignalSparklinePainter({
    required this.points,
    required this.color,
  });

  final List<DashboardSeriesPoint> points;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (points.length < 2) {
      return;
    }
    final values = points.map((point) => point.value).toList();
    final minValue = values.reduce(math.min);
    final maxValue = values.reduce(math.max);
    final valueRange = maxValue - minValue;

    final gridPaint = Paint()
      ..color = const Color(0xFFD8D1C5)
      ..strokeWidth = 1;
    canvas.drawLine(
      Offset(0, size.height - 1),
      Offset(size.width, size.height - 1),
      gridPaint,
    );

    final linePaint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.2
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;

    final fillPaint = Paint()
      ..shader = LinearGradient(
        colors: [color.withAlpha(48), color.withAlpha(4)],
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
      ).createShader(Rect.fromLTWH(0, 0, size.width, size.height));

    double normalizedY(double value) {
      if (valueRange == 0) {
        return size.height * 0.5;
      }
      final normalized = (value - minValue) / valueRange;
      return size.height - (normalized * (size.height - 6)) - 3;
    }

    final path = Path();
    final fillPath = Path();
    for (var i = 0; i < points.length; i++) {
      final x = points.length == 1 ? 0.0 : (i / (points.length - 1)) * size.width;
      final y = normalizedY(points[i].value);
      if (i == 0) {
        path.moveTo(x, y);
        fillPath.moveTo(x, size.height);
        fillPath.lineTo(x, y);
      } else {
        path.lineTo(x, y);
        fillPath.lineTo(x, y);
      }
    }
    fillPath.lineTo(size.width, size.height);
    fillPath.close();

    canvas.drawPath(fillPath, fillPaint);
    canvas.drawPath(path, linePaint);

    final lastPoint = points.last;
    final lastX = size.width;
    final lastY = normalizedY(lastPoint.value);
    canvas.drawCircle(
      Offset(lastX, lastY),
      3.5,
      Paint()..color = color,
    );
  }

  @override
  bool shouldRepaint(covariant _SignalSparklinePainter oldDelegate) {
    return oldDelegate.points != points || oldDelegate.color != color;
  }
}
