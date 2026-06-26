import 'package:flutter/material.dart';

import '../../client_home_controller.dart';
import '../../l10n/app_localizations.dart';
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
    final l10n = AppLocalizations.of(context);
    final autostart = controller.autostartStatus;
    return SubpageScaffold(
      title: l10n.navBasic,
      onClose: onClose,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Block(
            title: l10n.settingsAutostart,
            message: autostart == null
                ? l10n.settingsAutostartLoading
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
                  label: Text(l10n.settingsEnableAutostart),
                ),
                OutlinedButton.icon(
                  onPressed: controller.busy || !(autostart?.supported ?? false)
                      ? null
                      : () => controller.runTask(
                          'disable autostart',
                          controller.disableAutostart,
                        ),
                  icon: const Icon(Icons.logout),
                  label: Text(l10n.settingsDisableAutostart),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          _Block(
            title: l10n.settingsLanguage,
            message: l10n.settingsLanguageHint,
            child: DropdownButtonFormField<String>(
              initialValue: locale.languageCode,
              decoration: InputDecoration(
                labelText: l10n.settingsAppLanguage,
                border: const OutlineInputBorder(),
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
              onChanged: (value) {
                if (value != null) {
                  onLocaleCodeChanged(value);
                }
              },
            ),
          ),
          const SizedBox(height: 16),
          _Block(
            title: l10n.settingsTheme,
            message: l10n.settingsThemeHint,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                DropdownButtonFormField<ThemeMode>(
                  initialValue: themeMode,
                  decoration: InputDecoration(
                    labelText: l10n.settingsThemeMode,
                    border: const OutlineInputBorder(),
                  ),
                  items: [
                    DropdownMenuItem(
                      value: ThemeMode.system,
                      child: Text(l10n.settingsThemeFollowSystem),
                    ),
                    DropdownMenuItem(
                      value: ThemeMode.light,
                      child: Text(l10n.settingsThemeLight),
                    ),
                    DropdownMenuItem(
                      value: ThemeMode.dark,
                      child: Text(l10n.settingsThemeDark),
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
                  decoration: InputDecoration(
                    labelText: l10n.settingsThemePalette,
                    border: const OutlineInputBorder(),
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
            title: l10n.settingsLayout,
            message: l10n.settingsLayoutHint,
            child: DropdownButtonFormField<ChipIconSide>(
              initialValue: chipIconSide,
              decoration: InputDecoration(
                labelText: l10n.settingsChipIconSide,
                border: const OutlineInputBorder(),
              ),
              items: [
                DropdownMenuItem(
                  value: ChipIconSide.left,
                  child: Text(l10n.settingsChipIconLeft),
                ),
                DropdownMenuItem(
                  value: ChipIconSide.right,
                  child: Text(l10n.settingsChipIconRight),
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
