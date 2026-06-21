import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../home_widgets.dart';

class ProfilesView extends StatelessWidget {
  const ProfilesView({super.key, required this.controller});

  final ClientHomeController controller;

  @override
  Widget build(BuildContext context) {
    final selected = controller.selectedProfile;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          PanelIntroCard(
            title: 'Profile Management',
            description:
                'Save the current draft, return to earlier stacks, and keep imported or manual configurations organized without leaving the control surface.',
            badges: [
              InfoBadge(
                label: 'Saved',
                value: '${controller.savedProfiles.length}',
              ),
              InfoBadge(
                label: 'Selected',
                value: selected?.name ?? 'Draft only',
                tone: const Color(0xFF0B8A6E),
              ),
            ],
          ),
          const SizedBox(height: 16),
          SectionCard(
            eyebrow: 'Draft',
            title: 'Current Draft',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _field(
                  controller.profileName,
                  'Profile name',
                  key: const ValueKey('profile-name'),
                ),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 12,
                  runSpacing: 12,
                  children: [
                    OutlinedButton.icon(
                      onPressed: controller.busy ? null : controller.newBlankProfile,
                      icon: const Icon(Icons.add_circle_outline),
                      label: const Text('New blank'),
                    ),
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
                ),
                if (controller.profilesStatus.isNotEmpty) ...[
                  const SizedBox(height: 12),
                  NoticeCard(
                    title: 'Draft status',
                    message: controller.profilesStatus,
                    tone: const Color(0xFF2F4858),
                  ),
                ],
              ],
            ),
          ),
          const SizedBox(height: 16),
          SectionCard(
            eyebrow: 'Library',
            title: 'Saved Profiles',
            child: controller.savedProfiles.isEmpty
                ? const NoticeCard(
                    title: 'No saved profiles',
                    message:
                        'Save the current draft to create a reusable local profile entry.',
                  )
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      for (final profile in controller.savedProfiles)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 10),
                          child: _profileRow(context, profile),
                        ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 12,
                        runSpacing: 12,
                        children: [
                          OutlinedButton.icon(
                            onPressed: controller.busy || controller.selectedProfileId == null
                                ? null
                                : controller.loadSelectedProfile,
                            icon: const Icon(Icons.upload_file),
                            label: const Text('Load selected'),
                          ),
                          OutlinedButton.icon(
                            onPressed: controller.busy || controller.selectedProfileId == null
                                ? null
                                : () => controller.runTask(
                                      'duplicate profile',
                                      controller.duplicateSelectedProfile,
                                    ),
                            icon: const Icon(Icons.copy_all),
                            label: const Text('Duplicate selected'),
                          ),
                          OutlinedButton.icon(
                            onPressed: controller.busy || controller.selectedProfileId == null
                                ? null
                                : () => _confirmDelete(context),
                            icon: const Icon(Icons.delete_outline),
                            label: const Text('Delete selected'),
                          ),
                        ],
                      ),
                    ],
                  ),
          ),
        ],
      ),
    );
  }

  Widget _profileRow(BuildContext context, dynamic profile) {
    final selected = controller.selectedProfileId == profile.id;
    final borderColor = selected
        ? const Color(0xFF111111)
        : const Color(0xFFD8D1C5);
    final background = selected ? const Color(0xFFF2EEE6) : const Color(0xFFF8F6F1);
    return InkWell(
      borderRadius: BorderRadius.circular(18),
      onTap: () => controller.selectProfile(profile),
      child: Container(
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: background,
          borderRadius: BorderRadius.circular(18),
          border: Border.all(color: borderColor, width: selected ? 1.4 : 1),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: 38,
              height: 38,
              decoration: BoxDecoration(
                color: const Color(0xFF111111),
                borderRadius: BorderRadius.circular(12),
              ),
              child: const Icon(Icons.folder_copy_outlined, color: Colors.white, size: 20),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
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
                        const Icon(Icons.check_circle, size: 18, color: Color(0xFF0B8A6E)),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(
                    controller.formatProfileSubtitle(profile),
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                  const SizedBox(height: 10),
                  InfoBadge(
                    label: 'Support',
                    value: controller.profileSupportBadge(profile),
                    tone: selected ? const Color(0xFF0B8A6E) : const Color(0xFF2F4858),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _field(TextEditingController controller, String label, {Key? key}) {
    return TextField(
      key: key,
      controller: controller,
      decoration: InputDecoration(labelText: label),
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
            'Delete "${selected.name}" from the local profile list? This does not change the remote wrongsv server.',
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
      await controller.runTask('delete profile', controller.deleteSelectedProfile);
    }
  }
}
