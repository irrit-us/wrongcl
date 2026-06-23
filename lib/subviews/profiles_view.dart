import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../profile_store.dart';
import '../widgets/subpage_scaffold.dart';

class ProfilesView extends StatelessWidget {
  const ProfilesView({
    super.key,
    required this.controller,
    required this.onClose,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return SubpageScaffold(
      title: 'Profiles',
      onClose: onClose,
      actions: [
        OutlinedButton.icon(
          onPressed: controller.busy ? null : controller.newBlankProfile,
          icon: const Icon(Icons.add),
          label: const Text('New'),
        ),
        const SizedBox(width: 8),
        FilledButton.icon(
          onPressed: controller.busy
              ? null
              : () => controller.runTask(
                  'save profile',
                  controller.saveCurrentProfile,
                ),
          icon: const Icon(Icons.save),
          label: const Text('Save current'),
        ),
      ],
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Section(
            title: 'Current draft',
            children: [
              TextField(
                key: const ValueKey('profile-name'),
                controller: controller.profileName,
                decoration: const InputDecoration(labelText: 'Profile name'),
              ),
              if (controller.profilesStatus.isNotEmpty) ...[
                const SizedBox(height: 12),
                Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: const Color(0xFFF4F1EA),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(controller.profilesStatus),
                ),
              ],
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'wrongsv import',
            children: [
              TextField(
                controller: controller.wrongsvConfigPath,
                decoration: const InputDecoration(
                  labelText: 'wrongsv config path',
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: controller.wrongsvServerHost,
                decoration: const InputDecoration(
                  labelText: 'Server host for adapted client config',
                ),
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: controller.wrongsvListenHost,
                      decoration: const InputDecoration(
                        labelText: 'Local listen host',
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  SizedBox(
                    width: 140,
                    child: TextField(
                      controller: controller.wrongsvListenPort,
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(
                        labelText: 'Local listen port',
                      ),
                    ),
                  ),
                ],
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
                            'inspect wrongsv',
                            controller.inspectWrongsv,
                          ),
                    icon: const Icon(Icons.search),
                    label: const Text('Inspect wrongsv'),
                  ),
                  FilledButton.icon(
                    onPressed: controller.busy
                        ? null
                        : () => controller.runTask(
                            'adapt wrongsv',
                            controller.adaptWrongsv,
                          ),
                    icon: const Icon(Icons.transform),
                    label: const Text('Adapt wrongsv'),
                  ),
                ],
              ),
              if (controller.wrongsvReport != null) ...[
                const SizedBox(height: 12),
                Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: const Color(0xFFF4F1EA),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    controller.wrongsvStatusMessage(controller.wrongsvReport!),
                  ),
                ),
              ],
              if ((controller.wrongsvReport?.missingFields.isNotEmpty ??
                  false)) ...[
                const SizedBox(height: 12),
                for (final field in controller.wrongsvReport!.missingFields)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 10),
                    child: _MissingFieldInput(
                      label: controller.labelForMissingField(field.field),
                      controller: controller.controllerForMissingField(
                        field.field,
                      ),
                    ),
                  ),
                Wrap(
                  spacing: 12,
                  runSpacing: 12,
                  children: [
                    FilledButton.icon(
                      onPressed: controller.busy
                          ? null
                          : () => controller.runTask(
                              'complete wrongsv import',
                              controller.completeWrongsvImport,
                            ),
                      icon: const Icon(Icons.verified_outlined),
                      label: const Text('Complete import'),
                    ),
                  ],
                ),
              ],
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'Saved profiles',
            children: [
              if (controller.savedProfiles.isEmpty)
                const Text(
                  'No saved profiles yet. Save the current draft to create '
                  'a reusable entry.',
                  style: TextStyle(color: Color(0xFF8B8579)),
                )
              else
                Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    for (final profile in controller.savedProfiles)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 10),
                        child: _profileRow(context, profile),
                      ),
                    const SizedBox(height: 4),
                    Wrap(
                      spacing: 12,
                      runSpacing: 12,
                      children: [
                        OutlinedButton.icon(
                          onPressed:
                              controller.busy ||
                                  controller.selectedProfileId == null
                              ? null
                              : controller.loadSelectedProfile,
                          icon: const Icon(Icons.upload_file),
                          label: const Text('Load selected'),
                        ),
                        OutlinedButton.icon(
                          onPressed:
                              controller.busy ||
                                  controller.selectedProfileId == null
                              ? null
                              : () => controller.runTask(
                                  'duplicate profile',
                                  controller.duplicateSelectedProfile,
                                ),
                          icon: const Icon(Icons.copy_all),
                          label: const Text('Duplicate selected'),
                        ),
                        OutlinedButton.icon(
                          onPressed:
                              controller.busy ||
                                  controller.selectedProfileId == null
                              ? null
                              : () => _confirmDelete(context),
                          icon: const Icon(Icons.delete_outline),
                          label: const Text('Delete selected'),
                        ),
                      ],
                    ),
                  ],
                ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _profileRow(BuildContext context, SavedProfile profile) {
    final selected = controller.selectedProfileId == profile.id;
    final borderColor = selected
        ? const Color(0xFF111111)
        : const Color(0xFFD8D1C5);
    final background = selected
        ? const Color(0xFFF2EEE6)
        : const Color(0xFFF8F6F1);
    return InkWell(
      borderRadius: BorderRadius.circular(14),
      onTap: () => controller.selectProfile(profile),
      child: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: background,
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: borderColor, width: selected ? 1.4 : 1),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: const Color(0xFF111111),
                borderRadius: BorderRadius.circular(10),
              ),
              child: const Icon(
                Icons.folder_copy_outlined,
                color: Colors.white,
                size: 18,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          profile.name,
                          style: Theme.of(context).textTheme.titleSmall,
                        ),
                      ),
                      if (selected)
                        const Icon(
                          Icons.check_circle,
                          size: 18,
                          color: Color(0xFF0B8A6E),
                        ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(
                    controller.formatProfileSubtitle(profile),
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _confirmDelete(BuildContext context) async {
    final selected = controller.selectedProfile;
    if (selected == null) {
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) {
        return AlertDialog(
          title: const Text('Delete saved profile?'),
          content: Text(
            'Delete "${selected.name}" from the local profile list? '
            'This does not change the remote wrongsv server.',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(dialogContext).pop(true),
              child: const Text('Delete profile'),
            ),
          ],
        );
      },
    );
    if (confirmed == true) {
      await controller.runTask(
        'delete profile',
        controller.deleteSelectedProfile,
      );
    }
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

class _MissingFieldInput extends StatelessWidget {
  const _MissingFieldInput({required this.label, required this.controller});

  final String label;
  final TextEditingController? controller;

  @override
  Widget build(BuildContext context) {
    if (controller == null) {
      return Text(label);
    }
    return TextField(
      controller: controller,
      decoration: InputDecoration(labelText: label),
    );
  }
}
