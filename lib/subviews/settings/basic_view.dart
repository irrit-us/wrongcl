import 'package:flutter/material.dart';

import '../../client_home_controller.dart';
import '../../theme/wrongcl_colors.dart';
import '../../widgets/entry_chip.dart';
import '../../widgets/subpage_scaffold.dart';

class BasicSettingsView extends StatelessWidget {
  const BasicSettingsView({
    super.key,
    required this.controller,
    required this.onClose,
    required this.themeMode,
    required this.onThemeModeChanged,
    required this.locale,
    required this.onLocaleCodeChanged,
    required this.themeVariant,
    required this.onThemeVariantChanged,
    required this.chipIconSide,
    required this.onChipIconSideChanged,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;
  final ThemeMode themeMode;
  final Future<void> Function(ThemeMode value) onThemeModeChanged;
  final Locale locale;
  final Future<void> Function(String value) onLocaleCodeChanged;
  final WrongclThemeVariant themeVariant;
  final Future<void> Function(WrongclThemeVariant value) onThemeVariantChanged;
  final ChipIconSide chipIconSide;
  final Future<void> Function(ChipIconSide value) onChipIconSideChanged;

  @override
  Widget build(BuildContext context) {
    final autostart = controller.autostartStatus;
    return SubpageScaffold(
      title: 'Basic',
      onClose: onClose,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Block(
            title: 'Autostart',
            message: autostart == null
                ? 'Loading autostart status...'
                : autostart.message,
            child: Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                FilledButton.icon(
                  onPressed: controller.busy || !(autostart?.supported ?? false)
                      ? null
                      : () => controller.runTask(
                          'enable autostart',
                          controller.enableAutostart,
                        ),
                  icon: const Icon(Icons.login),
                  label: const Text('Enable autostart'),
                ),
                OutlinedButton.icon(
                  onPressed: controller.busy || !(autostart?.supported ?? false)
                      ? null
                      : () => controller.runTask(
                          'disable autostart',
                          controller.disableAutostart,
                        ),
                  icon: const Icon(Icons.logout),
                  label: const Text('Disable autostart'),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          _Block(
            title: 'Language',
            message: 'Language preference is saved locally.',
            child: DropdownButtonFormField<String>(
              initialValue: locale.languageCode,
              decoration: const InputDecoration(
                labelText: 'App language',
                border: OutlineInputBorder(),
              ),
              items: const [
                DropdownMenuItem(value: 'en', child: Text('English')),
                DropdownMenuItem(value: 'zh', child: Text('简体中文')),
                DropdownMenuItem(value: 'es', child: Text('Español')),
                DropdownMenuItem(
                  value: 'ar',
                  child: Text(
                    'العربية',
                    textDirection: TextDirection.rtl,
                  ),
                ),
                DropdownMenuItem(value: 'fr', child: Text('Français')),
              ],
              onChanged: controller.busy
                  ? null
                  : (value) {
                      if (value != null) {
                        controller.runTask(
                          'update language',
                          () => onLocaleCodeChanged(value),
                        );
                      }
                    },
            ),
          ),
          const SizedBox(height: 16),
          _Block(
            title: 'Theme',
            message: 'Theme mode and palette are saved locally.',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                DropdownButtonFormField<ThemeMode>(
                  initialValue: themeMode,
                  decoration: const InputDecoration(
                    labelText: 'Theme mode',
                    border: OutlineInputBorder(),
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: ThemeMode.system,
                      child: Text('Follow system'),
                    ),
                    DropdownMenuItem(
                      value: ThemeMode.light,
                      child: Text('Light'),
                    ),
                    DropdownMenuItem(
                      value: ThemeMode.dark,
                      child: Text('Dark'),
                    ),
                  ],
                  onChanged: controller.busy
                      ? null
                      : (value) {
                          if (value != null) {
                            controller.runTask(
                              'update theme',
                              () => onThemeModeChanged(value),
                            );
                          }
                        },
                ),
                const SizedBox(height: 12),
                DropdownButtonFormField<WrongclThemeVariant>(
                  initialValue: themeVariant,
                  decoration: const InputDecoration(
                    labelText: 'Theme palette',
                    border: OutlineInputBorder(),
                  ),
                  items: [
                    for (final variant in WrongclThemeVariant.values)
                      DropdownMenuItem(
                        value: variant,
                        child: Text(variant.label),
                      ),
                  ],
                  onChanged: controller.busy
                      ? null
                      : (value) {
                          if (value != null) {
                            controller.runTask(
                              'update palette',
                              () => onThemeVariantChanged(value),
                            );
                          }
                        },
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          _Block(
            title: 'Layout',
            message: 'Move chip icons to the right when reading right-to-left '
                '(Arabic, Hebrew, ...).',
            child: DropdownButtonFormField<ChipIconSide>(
              initialValue: chipIconSide,
              decoration: const InputDecoration(
                labelText: 'Chip icon side',
                border: OutlineInputBorder(),
              ),
              items: const [
                DropdownMenuItem(
                  value: ChipIconSide.left,
                  child: Text('Left (default)'),
                ),
                DropdownMenuItem(
                  value: ChipIconSide.right,
                  child: Text('Right (RTL)'),
                ),
              ],
              onChanged: controller.busy
                  ? null
                  : (value) {
                      if (value != null) {
                        controller.runTask(
                          'update chip icon side',
                          () => onChipIconSideChanged(value),
                        );
                      }
                    },
            ),
          ),
        ],
      ),
    );
  }
}

class _Block extends StatelessWidget {
  const _Block({
    required this.title,
    required this.message,
    required this.child,
  });

  final String title;
  final String message;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: palette.surface.surfaceWarm,
        borderRadius: BorderRadius.circular(18),
        border: Border.all(color: palette.border.muted),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 8),
          Text(message),
          const SizedBox(height: 12),
          child,
        ],
      ),
    );
  }
}
