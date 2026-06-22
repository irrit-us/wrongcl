import 'package:flutter/material.dart';

import '../../client_home_controller.dart';
import '../../widgets/subpage_scaffold.dart';

class AdvancedSettingsView extends StatelessWidget {
  const AdvancedSettingsView({
    super.key,
    required this.controller,
    required this.onClose,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return SubpageScaffold(
      title: 'Advanced',
      onClose: onClose,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Section(
            title: 'Diagnostics',
            children: [
              Text(controller.nativeInfo),
              const SizedBox(height: 12),
              Wrap(
                spacing: 12,
                runSpacing: 12,
                children: [
                  OutlinedButton.icon(
                    onPressed: controller.busy
                        ? null
                        : controller.refreshStatus,
                    icon: const Icon(Icons.refresh),
                    label: const Text('Refresh status'),
                  ),
                  OutlinedButton.icon(
                    onPressed: controller.busy
                        ? null
                        : () => controller.runTask(
                            'validate config',
                            controller.validateCurrentConfig,
                          ),
                    icon: const Icon(Icons.fact_check_outlined),
                    label: const Text('Validate config'),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'Log level',
            children: [
              const Text(
                'This filters what the Logs page displays. It does not change '
                'the native tracing emission level yet.',
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<LogLevelFilter>(
                initialValue: controller.logLevelFilter,
                decoration: const InputDecoration(
                  labelText: 'Logs page filter',
                  border: OutlineInputBorder(),
                ),
                items: [
                  for (final option in LogLevelFilter.values)
                    DropdownMenuItem(value: option, child: Text(option.label)),
                ],
                onChanged: controller.busy
                    ? null
                    : (value) {
                        if (value != null) {
                          controller.setLogLevelFilter(value);
                        }
                      },
              ),
              const SizedBox(height: 12),
              Text('Current filter: ${controller.logLevelFilter.label}'),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'Raw config editor',
            children: [
              const Text(
                'Edit the current draft as JSON. TOML is supported for file '
                'export, while file loading accepts whatever the native client '
                'can parse.',
              ),
              const SizedBox(height: 12),
              TextField(
                controller: controller.clientConfigPath,
                decoration: const InputDecoration(
                  labelText: 'Config file path',
                  hintText: '/tmp/wrongcl-config.json',
                ),
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 12,
                runSpacing: 12,
                children: [
                  OutlinedButton.icon(
                    onPressed: controller.busy
                        ? null
                        : () => controller.runTask(
                            'load config file',
                            controller.loadClientConfigFile,
                          ),
                    icon: const Icon(Icons.file_open_outlined),
                    label: const Text('Load file'),
                  ),
                  OutlinedButton.icon(
                    onPressed: controller.busy
                        ? null
                        : () => controller.runTask(
                            'export config json',
                            controller.exportCurrentConfigJson,
                          ),
                    icon: const Icon(Icons.download_outlined),
                    label: const Text('Export JSON'),
                  ),
                  OutlinedButton.icon(
                    onPressed: controller.busy
                        ? null
                        : () => controller.runTask(
                            'export config toml',
                            controller.exportCurrentConfigToml,
                          ),
                    icon: const Icon(Icons.description_outlined),
                    label: const Text('Export TOML'),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Align(
                alignment: Alignment.centerLeft,
                child: OutlinedButton.icon(
                  onPressed: controller.busy
                      ? null
                      : controller.syncRawConfigEditorFromDraft,
                  icon: const Icon(Icons.sync_alt),
                  label: const Text('Load current draft'),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: controller.rawConfigEditor,
                maxLines: 18,
                minLines: 12,
                decoration: const InputDecoration(
                  labelText: 'Raw client config (JSON)',
                  alignLabelWithHint: true,
                ),
              ),
              const SizedBox(height: 12),
              Align(
                alignment: Alignment.centerRight,
                child: FilledButton.icon(
                  onPressed: controller.busy
                      ? null
                      : () => controller.runTask(
                          'apply raw config',
                          controller.applyRawConfigEditorJson,
                        ),
                  icon: const Icon(Icons.playlist_add_check_circle_outlined),
                  label: const Text('Apply JSON to draft'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({required this.title, required this.children});

  final String title;
  final List<Widget> children;

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
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(title, style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 12),
          ...children,
        ],
      ),
    );
  }
}
