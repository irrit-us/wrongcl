import 'package:flutter/material.dart';

import '../../client_home_controller.dart';
import '../../l10n/app_localizations.dart';
import '../../theme/wrongcl_colors.dart';
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
    final l10n = AppLocalizations.of(context);
    return SubpageScaffold(
      title: l10n.navAdvanced,
      onClose: onClose,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Section(
            title: l10n.advancedDiagnostics,
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
                    label: Text(l10n.advancedRefreshStatus),
                  ),
                  OutlinedButton.icon(
                    onPressed: controller.busy
                        ? null
                        : () => controller.runTask(
                            'validate config',
                            controller.validateCurrentConfig,
                          ),
                    icon: const Icon(Icons.fact_check_outlined),
                    label: Text(l10n.advancedValidateConfig),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: l10n.advancedLogLevel,
            children: [
              Text(l10n.advancedLogLevelHint),
              const SizedBox(height: 12),
              DropdownButtonFormField<LogLevelFilter>(
                initialValue: controller.logLevelFilter,
                decoration: InputDecoration(
                  labelText: l10n.advancedLogsPageFilter,
                  border: const OutlineInputBorder(),
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
              Text(l10n.advancedCurrentFilter(controller.logLevelFilter.label)),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: l10n.advancedRawConfigEditor,
            children: [
              Text(l10n.advancedRawConfigEditorHint),
              const SizedBox(height: 12),
              TextField(
                controller: controller.clientConfigPath,
                decoration: InputDecoration(
                  labelText: l10n.advancedConfigFilePath,
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
                    label: Text(l10n.advancedLoadFile),
                  ),
                  OutlinedButton.icon(
                    onPressed: controller.busy
                        ? null
                        : () => controller.runTask(
                            'export config json',
                            controller.exportCurrentConfigJson,
                          ),
                    icon: const Icon(Icons.download_outlined),
                    label: Text(l10n.advancedExportJson),
                  ),
                  OutlinedButton.icon(
                    onPressed: controller.busy
                        ? null
                        : () => controller.runTask(
                            'export config toml',
                            controller.exportCurrentConfigToml,
                          ),
                    icon: const Icon(Icons.description_outlined),
                    label: Text(l10n.advancedExportToml),
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
                  label: Text(l10n.advancedLoadCurrentDraft),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: controller.rawConfigEditor,
                maxLines: 18,
                minLines: 12,
                decoration: InputDecoration(
                  labelText: l10n.advancedRawClientConfigJson,
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
                  label: Text(l10n.advancedApplyJsonToDraft),
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
    final palette = context.wrongclColors;
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: palette.surface.surfaceWarm,
        borderRadius: BorderRadius.circular(18),
        border: Border.all(color: palette.border.muted),
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
