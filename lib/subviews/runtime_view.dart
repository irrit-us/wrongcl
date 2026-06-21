import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../home_widgets.dart';

class _RuntimeSubsection extends StatelessWidget {
  const _RuntimeSubsection({
    required this.title,
    required this.description,
    required this.child,
  });

  final String title;
  final String description;
  final Widget child;

  @override
  Widget build(BuildContext context) {
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
          Text(title, style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 6),
          Text(description, style: Theme.of(context).textTheme.bodySmall),
          const SizedBox(height: 12),
          child,
        ],
      ),
    );
  }
}

class RuntimeView extends StatelessWidget {
  const RuntimeView({super.key, required this.controller});

  final ClientHomeController controller;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 980),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              PanelIntroCard(
                title: 'Diagnostics Workbench',
                description:
                    'Use this mode to inspect the current runtime shape, send a targeted probe, and review the latest response or failure context without crowding the main control surface.',
                badges: [
                  InfoBadge(
                    label: 'Runtime',
                    value: controller.running ? 'Running' : 'Stopped',
                  ),
                  InfoBadge(
                    label: 'Probe target',
                    value: '${controller.targetHost.text}:${controller.targetPort.text}',
                    tone: const Color(0xFF2F4858),
                  ),
                  InfoBadge(
                    label: 'Last error',
                    value: controller.lastError == null ? 'None' : controller.lastError!.action,
                    tone: controller.lastError == null
                        ? const Color(0xFF0B8A6E)
                        : const Color(0xFF9A3412),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              SectionCard(
                eyebrow: 'Overview',
                title: 'Diagnostics Context',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Use the main control surface for shallow runtime actions. This focused mode is for deeper probe and response inspection.',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 12,
                      runSpacing: 12,
                      children: [
                        OutlinedButton.icon(
                          onPressed: controller.busy ? null : controller.refreshStatus,
                          icon: const Icon(Icons.refresh),
                          label: const Text('Refresh runtime state'),
                        ),
                        FilledButton.icon(
                          onPressed: controller.busy ? null : controller.runProbe,
                          icon: const Icon(Icons.network_check),
                          label: const Text('Run probe'),
                        ),
                      ],
                    ),
                    const SizedBox(height: 14),
                    _RuntimeSubsection(
                      title: 'Recent health context',
                      description:
                          'This is the most recent health information already tracked by the controller. It stays truthful to the last probe and last error snapshots.',
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            controller.lastProbe == null
                                ? 'Last successful probe: none recorded yet'
                                : 'Last successful probe: ${controller.lastProbe!.bytesRead} bytes | ${controller.lastProbe!.preview}',
                            style: Theme.of(context).textTheme.bodySmall,
                          ),
                          const SizedBox(height: 8),
                          Text(
                            controller.lastError == null
                                ? 'Last error: none recorded'
                                : 'Last error: ${controller.lastError!.action} | ${controller.lastError!.message}',
                            style: Theme.of(context).textTheme.bodySmall,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
              SectionCard(
                eyebrow: 'Probe',
                title: 'Probe',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    _RuntimeSubsection(
                      title: 'Probe target',
                      description:
                          'Define where the current draft should connect when you run a direct probe through the configured stack.',
                      child: Wrap(
                        spacing: 12,
                        runSpacing: 12,
                        children: [
                          _field(controller.targetHost, 'Target host', width: 360),
                          _field(controller.targetPort, 'Target port', width: 150),
                        ],
                      ),
                    ),
                    const SizedBox(height: 12),
                    _RuntimeSubsection(
                      title: 'Payload',
                      description:
                          'Send the raw request body exactly as entered here. This is useful for lightweight HTTP-style probes and protocol sanity checks.',
                      child: TextField(
                        controller: controller.payload,
                        minLines: 4,
                        maxLines: 8,
                        decoration: const InputDecoration(
                          labelText: 'Payload',
                          alignLabelWithHint: true,
                        ),
                        style: const TextStyle(fontFamily: 'monospace'),
                      ),
                    ),
                    const SizedBox(height: 12),
                    FilledButton.icon(
                      onPressed: controller.busy ? null : controller.runProbe,
                      icon: const Icon(Icons.network_check),
                      label: const Text('Run probe'),
                    ),
                  ],
                ),
              ),
              if (controller.lastResponse != null) ...[
                const SizedBox(height: 16),
                SectionCard(
                  eyebrow: 'Output',
                  title: controller.lastResponse!.ok ? 'Result' : 'Error',
                  child: _RuntimeSubsection(
                    title: controller.lastResponse!.ok
                        ? 'Response review'
                        : 'Failure review',
                    description: controller.lastResponse!.ok
                        ? 'Inspect the most recent probe or runtime response exactly as returned by the current client path.'
                        : 'Inspect the latest failure message exactly as returned by the current client path.',
                    child: ResultView(response: controller.lastResponse!),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
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
