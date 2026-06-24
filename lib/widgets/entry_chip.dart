import 'package:flutter/material.dart';

import '../theme/wrongcl_colors.dart';

/// A dashboard entry tile.
///
/// Layout rule: both axes are centered. A title-only chip stacks the icon
/// above the label; a subtitle adds a second centered line beneath. We
/// intentionally omit a trailing chevron — the centered glyph + label is
/// already a strong tap affordance and the chevron was the main contributor
/// to the empty right-edge band on the dashboard.
class EntryChip extends StatelessWidget {
  const EntryChip({
    super.key,
    required this.label,
    required this.onTap,
    this.subtitle,
    this.icon,
  });

  final String label;
  final VoidCallback? onTap;
  final String? subtitle;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final hasSubtitle = subtitle != null && subtitle!.isNotEmpty;
    final theme = Theme.of(context);
    return SizedBox.expand(
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(14),
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            decoration: BoxDecoration(
              color: palette.surface.surfaceRaised,
              borderRadius: BorderRadius.circular(14),
              border: Border.all(color: palette.border.regular),
            ),
            child: Center(
              child: FittedBox(
                fit: BoxFit.scaleDown,
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.center,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (icon != null) ...[
                      Icon(icon, size: 24, color: palette.accent.primary),
                      const SizedBox(height: 6),
                    ],
                    Text(
                      label,
                      textAlign: TextAlign.center,
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
                        textAlign: TextAlign.center,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: palette.text.secondary,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
