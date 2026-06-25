import 'package:flutter/material.dart';

import '../theme/wrongcl_colors.dart';

enum ChipIconSide { left, right }

/// A dashboard entry tile.
///
/// Layout rule: icon and text sit on the same row. The icon defaults to the
/// leading side (left); flipping to [ChipIconSide.right] swaps it to the
/// trailing side and end-aligns the text — used at runtime for languages that
/// read right-to-left (Arabic, Hebrew, ...).
class EntryChip extends StatelessWidget {
  const EntryChip({
    super.key,
    required this.label,
    required this.onTap,
    this.subtitle,
    this.trailing,
    this.icon,
    this.iconSide = ChipIconSide.left,
  });

  final String label;
  final VoidCallback? onTap;
  final String? subtitle;
  final String? trailing;
  final IconData? icon;
  final ChipIconSide iconSide;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final hasSubtitle = subtitle != null && subtitle!.isNotEmpty;
    final theme = Theme.of(context);
    final iconOnRight = iconSide == ChipIconSide.right;
    final textAlign = iconOnRight ? TextAlign.end : TextAlign.start;
    final crossAxis = iconOnRight
        ? CrossAxisAlignment.end
        : CrossAxisAlignment.start;
    final textBlock = FittedBox(
      fit: BoxFit.scaleDown,
      alignment: iconOnRight ? Alignment.centerRight : Alignment.centerLeft,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: crossAxis,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            label,
            textAlign: textAlign,
            overflow: TextOverflow.ellipsis,
            maxLines: 1,
            style: theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.w600,
              color: palette.text.primary,
            ),
          ),
          if (hasSubtitle) ...[
            const SizedBox(height: 2),
            Text(
              subtitle!,
              maxLines: 1,
              textAlign: textAlign,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: palette.text.secondary,
              ),
            ),
          ],
        ],
      ),
    );
    final iconWidget = icon == null
        ? null
        : Icon(icon, size: 22, color: palette.accent.primary);
    final trailingWidget = trailing == null
        ? null
        : FittedBox(
            fit: BoxFit.scaleDown,
            child: Text(
              trailing!,
              maxLines: 1,
              style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w700,
                color: palette.text.muted,
              ),
            ),
          );
    final rowChildren = <Widget>[
      if (iconWidget != null && !iconOnRight) ...[
        iconWidget,
        const SizedBox(width: 8),
      ],
      if (trailingWidget != null && iconOnRight) ...[
        trailingWidget,
        const SizedBox(width: 8),
      ],
      Expanded(child: textBlock),
      if (trailingWidget != null && !iconOnRight) ...[
        const SizedBox(width: 8),
        trailingWidget,
      ],
      if (iconWidget != null && iconOnRight) ...[
        const SizedBox(width: 8),
        iconWidget,
      ],
    ];
    return SizedBox.expand(
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(14),
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            decoration: BoxDecoration(
              color: palette.surface.surfaceRaised,
              borderRadius: BorderRadius.circular(14),
              border: Border.all(color: palette.border.regular),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: rowChildren,
            ),
          ),
        ),
      ),
    );
  }
}
