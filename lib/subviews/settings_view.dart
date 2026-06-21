import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../home_widgets.dart';

class SettingsView extends StatelessWidget {
  const SettingsView({super.key, required this.controller});

  final ClientHomeController controller;

  @override
  Widget build(BuildContext context) {
    final autostart = controller.autostartStatus;
    final systemProxy = controller.systemProxyStatus;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          PanelIntroCard(
            title: 'Desktop Settings',
            description:
                'Adjust platform integrations, review what the current runtime can control, and keep unsupported capabilities visible with explicit reasons.',
            badges: [
              InfoBadge(
                label: 'Autostart',
                value: autostart == null
                    ? 'Loading'
                    : autostart.supported
                    ? (autostart.enabled ? 'Enabled' : 'Disabled')
                    : 'Unsupported',
              ),
              InfoBadge(
                label: 'System proxy',
                value: systemProxy == null
                    ? 'Loading'
                    : systemProxy.supported
                    ? (systemProxy.enabled ? 'Enabled' : 'Disabled')
                    : 'Unsupported',
                tone: const Color(0xFF0B8A6E),
              ),
            ],
          ),
          const SizedBox(height: 16),
          SectionCard(
            eyebrow: 'System',
            title: 'Desktop Integration',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _settingsBlock(
                  context,
                  icon: Icons.rocket_launch_outlined,
                  title: 'Autostart',
                  message: autostart == null
                      ? 'Autostart: loading...'
                      : 'Autostart: ${autostart.message}',
                  detail: autostart != null && autostart.path.isNotEmpty
                      ? autostart.path
                      : null,
                  actions: [
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
                const SizedBox(height: 14),
                _settingsBlock(
                  context,
                  icon: Icons.settings_ethernet,
                  title: 'System Proxy',
                  message: systemProxy == null
                      ? 'System proxy: loading...'
                      : 'System proxy: ${systemProxy.message}',
                  actions: [
                    FilledButton.icon(
                      onPressed: controller.busy || !(systemProxy?.supported ?? false)
                          ? null
                          : () => controller.runTask(
                                'enable system proxy',
                                controller.enableSystemProxy,
                              ),
                      icon: const Icon(Icons.settings_ethernet),
                      label: const Text('Enable system proxy'),
                    ),
                    OutlinedButton.icon(
                      onPressed: controller.busy || !(systemProxy?.supported ?? false)
                          ? null
                          : () => controller.runTask(
                                'disable system proxy',
                                controller.disableSystemProxy,
                              ),
                      icon: const Icon(Icons.portable_wifi_off),
                      label: const Text('Disable system proxy'),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          const SectionCard(
            eyebrow: 'Capability',
            title: 'TUN',
            child: NoticeCard(
              title: 'Unsupported in current runtime',
              message: 'TUN runtime control is not exposed by wrongcl yet',
              tone: Color(0xFF7A5C1E),
            ),
          ),
          const SizedBox(height: 16),
          const SectionCard(
            eyebrow: 'Capability',
            title: 'Scripts',
            child: NoticeCard(
              title: 'Unsupported in current runtime',
              message: 'Script runtime selection is not exposed by wrongcl yet',
              tone: Color(0xFF7A5C1E),
            ),
          ),
        ],
      ),
    );
  }

  Widget _settingsBlock(
    BuildContext context, {
    required IconData icon,
    required String title,
    required String message,
    String? detail,
    required List<Widget> actions,
  }) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: const Color(0xFFF8F6F1),
        borderRadius: BorderRadius.circular(18),
        border: Border.all(color: const Color(0xFFD8D1C5)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, size: 20, color: const Color(0xFF2F4858)),
              const SizedBox(width: 8),
              Text(title, style: Theme.of(context).textTheme.titleSmall),
            ],
          ),
          const SizedBox(height: 10),
          Text(message),
          if (detail != null) ...[
            const SizedBox(height: 6),
            Text(detail, style: Theme.of(context).textTheme.bodySmall),
          ],
          const SizedBox(height: 12),
          Wrap(spacing: 12, runSpacing: 12, children: actions),
        ],
      ),
    );
  }
}
