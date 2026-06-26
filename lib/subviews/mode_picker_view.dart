import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../l10n/app_localizations.dart';
import '../theme/wrongcl_colors.dart';
import '../widgets/subpage_scaffold.dart';
import '../wrongcl_client.dart';

class ModePickerView extends StatefulWidget {
  const ModePickerView({
    super.key,
    required this.controller,
    required this.onClose,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  State<ModePickerView> createState() => _ModePickerViewState();
}

class _ModePickerViewState extends State<ModePickerView> {
  final _formKey = GlobalKey<FormState>();
  final _nameController = TextEditingController();
  String? _proxyName;
  String? _scriptName;
  String? _errorMessage;
  bool _saving = false;

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    final proxy = _proxyName;
    if (proxy == null) {
      setState(() {
        _errorMessage = AppLocalizations.of(context).modePickProxy;
      });
      return;
    }
    setState(() {
      _saving = true;
      _errorMessage = null;
    });
    final mode = RouterMode(
      name: _nameController.text.trim(),
      kind: 'user',
      proxy: proxy,
      script: _scriptName,
    );
    final response = await widget.controller.upsertUserMode(mode);
    if (!mounted) return;
    setState(() {
      _saving = false;
      if (!response.ok) {
        _errorMessage = response.message;
      }
    });
    if (response.ok) {
      widget.onClose();
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final l10n = AppLocalizations.of(context);
    final controller = widget.controller;
    final groups = controller.currentProxyGroups;
    final candidates = <String>[
      for (final e in groups.endpoints) e.name,
      for (final g in groups.groups) g.name,
    ];
    final scripts = controller.currentRouterSnapshot.scripts;
    return SubpageScaffold(
      title: l10n.modeAddTitle,
      onClose: widget.onClose,
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Container(
            constraints: const BoxConstraints(maxWidth: 520),
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: palette.surface.surfaceMuted,
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: palette.border.regular),
            ),
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    l10n.modeNewUserMode,
                    style: TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                      color: palette.accent.primary,
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextFormField(
                    controller: _nameController,
                    decoration: InputDecoration(
                      labelText: l10n.modeName,
                      border: const OutlineInputBorder(),
                    ),
                    validator: (value) {
                      final v = value?.trim() ?? '';
                      if (v.isEmpty) return l10n.modeNameRequired;
                      if (v == 'global' || v == 'rule' || v == 'direct') {
                        return l10n.modeNameConflictsBuiltin;
                      }
                      return null;
                    },
                  ),
                  const SizedBox(height: 12),
                  DropdownButtonFormField<String>(
                    initialValue: _proxyName,
                    decoration: InputDecoration(
                      labelText: l10n.modeProxy,
                      border: const OutlineInputBorder(),
                    ),
                    items: [
                      for (final c in candidates)
                        DropdownMenuItem(value: c, child: Text(c)),
                    ],
                    onChanged: candidates.isEmpty
                        ? null
                        : (value) => setState(() => _proxyName = value),
                  ),
                  const SizedBox(height: 12),
                  DropdownButtonFormField<String?>(
                    initialValue: _scriptName,
                    decoration: InputDecoration(
                      labelText: l10n.modeScriptOptional,
                      border: const OutlineInputBorder(),
                    ),
                    items: [
                      DropdownMenuItem<String?>(
                        value: null,
                        child: Text(l10n.modeNone),
                      ),
                      for (final s in scripts)
                        DropdownMenuItem<String?>(
                          value: s.name,
                          child: Text(s.name),
                        ),
                    ],
                    onChanged: (value) => setState(() => _scriptName = value),
                  ),
                  if (_errorMessage != null) ...[
                    const SizedBox(height: 12),
                    Text(
                      _errorMessage!,
                      style: TextStyle(color: palette.text.danger),
                    ),
                  ],
                  const SizedBox(height: 16),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children: [
                      OutlinedButton(
                        onPressed: _saving ? null : widget.onClose,
                        child: Text(l10n.commonCancel),
                      ),
                      const SizedBox(width: 8),
                      FilledButton(
                        onPressed: _saving ? null : _save,
                        child: Text(_saving ? l10n.commonSavingEllipsis : l10n.commonSave),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
