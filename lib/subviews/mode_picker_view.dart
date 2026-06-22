import 'package:flutter/material.dart';

import '../client_home_controller.dart';
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
        _errorMessage = 'Pick a proxy/group to pin this mode to.';
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
    final controller = widget.controller;
    final groups = controller.currentProxyGroups;
    final candidates = <String>[
      for (final e in groups.endpoints) e.name,
      for (final g in groups.groups) g.name,
    ];
    final scripts = controller.currentRouterSnapshot.scripts;
    return SubpageScaffold(
      title: 'Add mode',
      onClose: widget.onClose,
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Container(
            constraints: const BoxConstraints(maxWidth: 520),
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: const Color(0xFFF4F1EA),
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: const Color(0xFFDCD5CA)),
            ),
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    'New user mode',
                    style: TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                      color: Color(0xFF2F4858),
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextFormField(
                    controller: _nameController,
                    decoration: const InputDecoration(
                      labelText: 'Name',
                      border: OutlineInputBorder(),
                    ),
                    validator: (value) {
                      final v = value?.trim() ?? '';
                      if (v.isEmpty) return 'Name is required';
                      if (v == 'global' || v == 'rule' || v == 'direct') {
                        return 'Name conflicts with a built-in mode';
                      }
                      return null;
                    },
                  ),
                  const SizedBox(height: 12),
                  DropdownButtonFormField<String>(
                    initialValue: _proxyName,
                    decoration: const InputDecoration(
                      labelText: 'Proxy',
                      border: OutlineInputBorder(),
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
                    decoration: const InputDecoration(
                      labelText: 'Script (optional)',
                      border: OutlineInputBorder(),
                    ),
                    items: [
                      const DropdownMenuItem<String?>(
                        value: null,
                        child: Text('— none —'),
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
                      style: const TextStyle(color: Color(0xFFB7401E)),
                    ),
                  ],
                  const SizedBox(height: 16),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children: [
                      OutlinedButton(
                        onPressed: _saving ? null : widget.onClose,
                        child: const Text('Cancel'),
                      ),
                      const SizedBox(width: 8),
                      FilledButton(
                        onPressed: _saving ? null : _save,
                        child: Text(_saving ? 'Saving…' : 'Save'),
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
