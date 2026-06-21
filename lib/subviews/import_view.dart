import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../home_widgets.dart';

class ImportView extends StatelessWidget {
  const ImportView({super.key, required this.controller});

  final ClientHomeController controller;

  @override
  Widget build(BuildContext context) {
    final report = controller.wrongsvReport;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          PanelIntroCard(
            title: 'wrongsv Import',
            description:
                'Inspect a server config, understand support boundaries, and adapt whatever can truthfully map into the current wrongcl client form.',
            badges: [
              InfoBadge(
                label: 'Report',
                value: report == null ? 'Not inspected' : report.activeSupport,
              ),
              InfoBadge(
                label: 'Active profile',
                value: report?.activeProfile ?? 'Unknown',
                tone: const Color(0xFF0B8A6E),
              ),
            ],
          ),
          const SizedBox(height: 16),
          SectionCard(
            eyebrow: 'Step 1',
            title: 'Import Flow',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                TextField(
                  key: const ValueKey('wrongsv-config-path'),
                  controller: controller.wrongsvConfigPath,
                  decoration: const InputDecoration(labelText: 'wrongsv config path'),
                ),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 12,
                  runSpacing: 12,
                  children: [
                    _field(controller.wrongsvServerHost, 'Adapt server host', width: 240),
                    _field(controller.wrongsvListenHost, 'Adapt listen host', width: 220),
                    _field(controller.wrongsvListenPort, 'Adapt listen port', width: 150),
                  ],
                ),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 12,
                  runSpacing: 12,
                  children: [
                    OutlinedButton.icon(
                      onPressed: controller.busy ? null : controller.inspectWrongsv,
                      icon: const Icon(Icons.rule),
                      label: const Text('Inspect wrongsv'),
                    ),
                    FilledButton.icon(
                      onPressed: controller.busy ? null : controller.adaptWrongsv,
                      icon: const Icon(Icons.sync_alt),
                      label: const Text('Adapt into form'),
                    ),
                  ],
                ),
              ],
            ),
          ),
          if (report != null) ...[
            const SizedBox(height: 16),
            SectionCard(
              eyebrow: 'Step 2',
              title: 'Capability Report',
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  WrongsvReportView(
                    report: report,
                    stackSummary: controller.wrongsvAdaptResult?.stackSummary,
                  ),
                  if (report.missingFields.isNotEmpty) ...[
                    const SizedBox(height: 12),
                    const NoticeCard(
                      title: 'Action required',
                      message:
                          'Some client-side values cannot be derived from the wrongsv server config alone. Supply them here before treating the draft as complete.',
                      tone: Color(0xFF7A5C1E),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      'Fill required client-side fields',
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                    const SizedBox(height: 8),
                    ..._missingFieldEditors(context),
                  ],
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }

  List<Widget> _missingFieldEditors(BuildContext context) {
    final report = controller.wrongsvReport;
    if (report == null || report.missingFields.isEmpty) {
      return const [];
    }
    final widgets = <Widget>[];
    final seen = <String>{};
    for (final field in report.missingFields) {
      if (!seen.add(field.field)) {
        continue;
      }
      final textController = controller.controllerForMissingField(field.field);
      if (textController == null) {
        widgets.add(
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(
              '${field.field}: ${field.reason}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ),
        );
        continue;
      }
      widgets.add(
        Container(
          padding: const EdgeInsets.all(14),
          margin: const EdgeInsets.only(bottom: 10),
          decoration: BoxDecoration(
            color: const Color(0xFFF8F6F1),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: const Color(0xFFD8D1C5)),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              TextField(
                key: ValueKey('missing-${field.field}'),
                controller: textController,
                decoration: InputDecoration(
                  labelText: controller.labelForMissingField(field.field),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                field.reason,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ),
      );
    }
    return widgets;
  }

  Widget _field(TextEditingController controller, String label, {double width = 220}) {
    return SizedBox(
      width: width,
      child: TextField(
        controller: controller,
        decoration: InputDecoration(labelText: label),
      ),
    );
  }
}
