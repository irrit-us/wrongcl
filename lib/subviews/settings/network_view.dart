import 'package:flutter/material.dart';

import '../../client_home_controller.dart';
import '../../l10n/app_localizations.dart';
import '../../theme/wrongcl_colors.dart';
import '../../widgets/subpage_scaffold.dart';

class NetworkSettingsView extends StatelessWidget {
  const NetworkSettingsView({
    super.key,
    required this.controller,
    required this.onClose,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final systemProxy = controller.systemProxyStatus;
    return SubpageScaffold(
      title: l10n.navNetwork,
      onClose: onClose,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Section(
            title: l10n.networkLocalProxyListenAddress,
            children: [
              TextField(
                controller: controller.localHost,
                decoration: InputDecoration(labelText: l10n.networkListenHost),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: controller.localPort,
                decoration: InputDecoration(labelText: l10n.networkListenPort),
                keyboardType: TextInputType.number,
              ),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: l10n.networkSystemProxy,
            children: [
              Text(systemProxy?.message ?? l10n.commonLoadingEllipsis),
              const SizedBox(height: 12),
              Wrap(
                spacing: 12,
                runSpacing: 12,
                children: [
                  FilledButton.icon(
                    onPressed:
                        controller.busy || !(systemProxy?.supported ?? false)
                        ? null
                        : () => controller.runTask(
                            'enable system proxy',
                            controller.enableSystemProxy,
                          ),
                    icon: const Icon(Icons.settings_ethernet),
                    label: Text(l10n.networkEnableSystemProxy),
                  ),
                  OutlinedButton.icon(
                    onPressed:
                        controller.busy || !(systemProxy?.supported ?? false)
                        ? null
                        : () => controller.runTask(
                            'disable system proxy',
                            controller.disableSystemProxy,
                          ),
                    icon: const Icon(Icons.portable_wifi_off),
                    label: Text(l10n.networkDisableSystemProxy),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: l10n.networkTunSetup,
            children: [
              Text(
                controller.tunAvailability.disabledReason.isEmpty
                    ? l10n.networkTunStatusAvailable
                    : controller.tunAvailability.disabledReason,
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 12,
                runSpacing: 12,
                children: [
                  FilledButton.icon(
                    onPressed:
                        controller.busy ||
                            !controller.tunPreparationAvailable ||
                            controller.tunAvailability.enabled
                        ? null
                        : () => controller.runTask(
                            'prepare tun interface',
                            controller.enableTun,
                          ),
                    icon: const Icon(Icons.shield_outlined),
                    label: Text(l10n.networkPrepareTunInterface),
                  ),
                  OutlinedButton.icon(
                    onPressed:
                        controller.busy || !controller.tunAvailability.enabled
                        ? null
                        : () => controller.runTask(
                            'remove tun interface',
                            controller.disableTun,
                          ),
                    icon: const Icon(Icons.link_off),
                    label: Text(l10n.networkRemovePreparedInterface),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Text(controller.tunGuidance),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: l10n.networkMixedProtocolToggles,
            children: [
              Material(
                color: Colors.transparent,
                child: SwitchListTile.adaptive(
                  contentPadding: EdgeInsets.zero,
                  title: Text(l10n.networkEnableSocks5Listener),
                  subtitle: Text(l10n.networkEnableSocks5Subtitle),
                  value: controller.localSocksEnabled,
                  onChanged: controller.busy
                      ? null
                      : controller.setLocalSocksEnabled,
                ),
              ),
              Material(
                color: Colors.transparent,
                child: SwitchListTile.adaptive(
                  contentPadding: EdgeInsets.zero,
                  title: Text(l10n.networkEnableHttpProxyListener),
                  subtitle: Text(l10n.networkEnableHttpProxySubtitle),
                  value: controller.localHttpEnabled,
                  onChanged: controller.busy
                      ? null
                      : controller.setLocalHttpEnabled,
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
