import 'package:flutter/material.dart';

import '../../client_home_controller.dart';
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
  });

  final ClientHomeController controller;
  final VoidCallback onClose;
  final ThemeMode themeMode;
  final Future<void> Function(ThemeMode value) onThemeModeChanged;
  final Locale locale;
  final Future<void> Function(String value) onLocaleCodeChanged;

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
            message: 'Theme preference is saved locally.',
            child: DropdownButtonFormField<ThemeMode>(
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
                DropdownMenuItem(value: ThemeMode.light, child: Text('Light')),
                DropdownMenuItem(value: ThemeMode.dark, child: Text('Dark')),
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
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: const Color(0xFFF8F6F1),
        borderRadius: BorderRadius.circular(18),
        border: Border.all(color: const Color(0xFFD8D1C5)),
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
