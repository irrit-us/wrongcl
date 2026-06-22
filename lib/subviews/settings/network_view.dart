import 'dart:io';

import 'package:flutter/material.dart';

import '../../client_home_controller.dart';
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
    final systemProxy = controller.systemProxyStatus;
    return SubpageScaffold(
      title: 'Network',
      onClose: onClose,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Section(
            title: 'Local proxy listen address',
            children: [
              TextField(
                controller: controller.localHost,
                decoration: const InputDecoration(labelText: 'Listen host'),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: controller.localPort,
                decoration: const InputDecoration(labelText: 'Listen port'),
                keyboardType: TextInputType.number,
              ),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'System proxy',
            children: [
              Text(systemProxy?.message ?? 'Loading...'),
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
                    label: const Text('Enable system proxy'),
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
                    label: const Text('Disable system proxy'),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'TUN setup',
            children: [
              Text(
                controller.tunAvailability.disabledReason.isEmpty
                    ? 'TUN status is available.'
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
                    label: const Text('Prepare TUN interface'),
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
                    label: const Text('Remove prepared interface'),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Text(_tunGuidance()),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'Mixed protocol toggles',
            children: [
              Material(
                color: Colors.transparent,
                child: SwitchListTile.adaptive(
                  contentPadding: EdgeInsets.zero,
                  title: const Text('Enable SOCKS5 listener'),
                  subtitle: const Text(
                    'Accept local SOCKS5 clients on the mixed port.',
                  ),
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
                  title: const Text('Enable HTTP proxy listener'),
                  subtitle: const Text(
                    'Accept HTTP CONNECT and absolute-form proxy requests.',
                  ),
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

  String _tunGuidance() {
    if (Platform.isLinux) {
      return 'Linux requires /dev/net/tun plus CAP_NET_ADMIN. wrongcl will not elevate privileges automatically.';
    }
    if (Platform.isWindows) {
      return 'Windows will need a wintun-backed implementation before the TUN toggle can become active.';
    }
    if (Platform.isMacOS) {
      return 'macOS will need a native utun-backed implementation before the TUN toggle can become active.';
    }
    return 'TUN setup is not implemented for this platform yet.';
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
