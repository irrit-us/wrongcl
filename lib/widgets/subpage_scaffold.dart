import 'package:flutter/material.dart';

import '../theme/wrongcl_colors.dart';

class SubpageScaffold extends StatelessWidget {
  const SubpageScaffold({
    super.key,
    required this.title,
    required this.onClose,
    required this.child,
    this.actions = const [],
  });

  final String title;
  final VoidCallback onClose;
  final Widget child;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Material(
      color: Theme.of(context).scaffoldBackgroundColor,
      child: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Container(
              padding: const EdgeInsets.fromLTRB(12, 10, 16, 10),
              decoration: BoxDecoration(
                color: palette.topBar.background,
                border: Border(
                  bottom: BorderSide(color: palette.border.subtle, width: 1),
                ),
              ),
              child: Row(
                children: [
                  IconButton(
                    tooltip: 'Close',
                    onPressed: onClose,
                    icon: const Icon(Icons.chevron_left),
                  ),
                  const SizedBox(width: 4),
                  Text(
                    title,
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      color: palette.topBar.foreground,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const Spacer(),
                  ...actions,
                ],
              ),
            ),
            Expanded(child: child),
          ],
        ),
      ),
    );
  }
}
