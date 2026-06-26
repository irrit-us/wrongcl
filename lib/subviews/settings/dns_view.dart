import 'package:flutter/material.dart';

import '../../client_home_controller.dart';
import '../../l10n/app_localizations.dart';
import '../../theme/wrongcl_colors.dart';
import '../../widgets/subpage_scaffold.dart';
import '../../wrongcl_client.dart';

class DnsSettingsView extends StatefulWidget {
  const DnsSettingsView({
    super.key,
    required this.controller,
    required this.onClose,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  State<DnsSettingsView> createState() => _DnsSettingsViewState();
}

class _DnsSettingsViewState extends State<DnsSettingsView> {
  late DnsBackendKind _kind;
  late TextEditingController _udpController;
  late TextEditingController _dohController;
  bool _saving = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    final settings = widget.controller.currentDnsSettings.normalized();
    _kind = settings.kind;
    _udpController = TextEditingController(text: settings.server ?? '');
    _dohController = TextEditingController(text: settings.url ?? '');
  }

  @override
  void dispose() {
    _udpController.dispose();
    _dohController.dispose();
    super.dispose();
  }

  Future<void> _apply() async {
    final settings = DnsSettingsInput(
      kind: _kind,
      server: _udpController.text,
      url: _dohController.text,
    );
    final validation = settings.validateMessage();
    if (validation != null) {
      setState(() {
        _errorMessage = validation;
      });
      return;
    }
    setState(() {
      _saving = true;
      _errorMessage = null;
    });
    final response = await widget.controller.setDnsSettings(settings);
    if (!mounted) return;
    setState(() {
      _saving = false;
      if (!response.ok) {
        _errorMessage = response.message;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final controller = widget.controller;
    final infoText = controller.running
        ? l10n.dnsApplyImmediately
        : l10n.dnsApplyOnNextStart;
    return SubpageScaffold(
      title: l10n.navDns,
      onClose: widget.onClose,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Section(
            title: l10n.dnsResolverBackend,
            children: [
              Text(infoText, style: Theme.of(context).textTheme.bodyMedium),
              const SizedBox(height: 12),
              DropdownButtonFormField<DnsBackendKind>(
                initialValue: _kind,
                decoration: InputDecoration(
                  labelText: l10n.dnsBackend,
                  border: const OutlineInputBorder(),
                ),
                items: [
                  for (final kind in DnsBackendKind.values)
                    DropdownMenuItem(value: kind, child: Text(kind.label)),
                ],
                onChanged: _saving
                    ? null
                    : (value) {
                        if (value == null) return;
                        setState(() {
                          _kind = value;
                          _errorMessage = null;
                        });
                      },
              ),
              if (_kind == DnsBackendKind.udp) ...[
                const SizedBox(height: 12),
                TextField(
                  controller: _udpController,
                  enabled: !_saving,
                  decoration: InputDecoration(
                    labelText: l10n.dnsUdpServer,
                    hintText: 'udp://1.1.1.1:53',
                    border: const OutlineInputBorder(),
                  ),
                ),
              ],
              if (_kind == DnsBackendKind.doh) ...[
                const SizedBox(height: 12),
                TextField(
                  controller: _dohController,
                  enabled: !_saving,
                  decoration: InputDecoration(
                    labelText: l10n.dnsDohUrl,
                    hintText: 'https://1.1.1.1/dns-query',
                    border: const OutlineInputBorder(),
                  ),
                ),
              ],
              const SizedBox(height: 12),
              Text(
                _helperText(l10n),
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(
                  color: context.wrongclColors.text.muted,
                ),
              ),
              if (controller.dnsStatus.isNotEmpty) ...[
                const SizedBox(height: 12),
                _Banner(message: controller.dnsStatus),
              ],
              if (_errorMessage != null) ...[
                const SizedBox(height: 12),
                Text(
                  _errorMessage!,
                  style: TextStyle(color: context.wrongclColors.text.danger),
                ),
              ],
              const SizedBox(height: 16),
              Align(
                alignment: Alignment.centerRight,
                child: FilledButton.icon(
                  onPressed: _saving ? null : _apply,
                  icon: const Icon(Icons.save_outlined),
                  label: Text(_saving ? l10n.commonApplyingEllipsis : l10n.dnsApplyDnsSettings),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  String _helperText(AppLocalizations l10n) {
    switch (_kind) {
      case DnsBackendKind.system:
        return l10n.dnsHelperSystem;
      case DnsBackendKind.udp:
        return l10n.dnsHelperUdp;
      case DnsBackendKind.doh:
        return l10n.dnsHelperDoh;
    }
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

class _Banner extends StatelessWidget {
  const _Banner({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: palette.surface.surfaceMuted,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text(message),
    );
  }
}
