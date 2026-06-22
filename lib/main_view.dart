import 'package:flutter/material.dart';

import 'client_home_controller.dart';
import 'widgets/control_column.dart';
import 'widgets/entry_chip.dart';
import 'widgets/mode_strip.dart';
import 'widgets/traffic_chart.dart';
import 'widgets/traffic_stats.dart';

class MainView extends StatelessWidget {
  const MainView({super.key, required this.controller});

  final ClientHomeController controller;

  @override
  Widget build(BuildContext context) {
    final snapshot = controller.dashboardSnapshot;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ModeStrip(
              slots: controller.modeSlots,
              activeId: controller.activeModeId,
              onSelect: controller.setActiveMode,
              onAdd: controller.openAddMode,
              disabledReason: controller.modeStripDisabledReason,
            ),
            const SizedBox(height: 12),
            Expanded(
              flex: 4,
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    child: TrafficChart(
                      uploadSeries: snapshot.signalSnapshot.uploadedBytesSeries,
                      downloadSeries:
                          snapshot.signalSnapshot.downloadedBytesSeries,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: ControlColumn(
                      systemProxy: snapshot.systemProxy,
                      tun: controller.tunAvailability,
                      running: controller.running,
                      busy: controller.busy,
                      onSystemProxyChanged: (value) {
                        final task = value
                            ? controller.enableSystemProxy
                            : controller.disableSystemProxy;
                        controller.runTask(
                          value
                              ? 'enable system proxy'
                              : 'disable system proxy',
                          task,
                        );
                      },
                      onRuntimeChanged: (value) {
                        if (value) {
                          controller.startProxy();
                        } else {
                          controller.stopProxy();
                        }
                      },
                      onTunChanged: (value) {
                        final task = value
                            ? controller.enableTun
                            : controller.disableTun;
                        controller.runTask(
                          value ? 'enable tun' : 'disable tun',
                          task,
                        );
                      },
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: TrafficStats(
                      bytesUploaded: controller.bytesUploaded,
                      bytesDownloaded: controller.bytesDownloaded,
                      upRatePerSecond: controller.upRatePerSecond,
                      downRatePerSecond: controller.downRatePerSecond,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 12),
            Expanded(flex: 3, child: _EntriesGrid(controller: controller)),
          ],
        ),
      ),
    );
  }
}

class _EntriesGrid extends StatelessWidget {
  const _EntriesGrid({required this.controller});

  final ClientHomeController controller;

  @override
  Widget build(BuildContext context) {
    final selectedProfile = controller.selectedProfile;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: Row(
            children: [
              Expanded(
                child: EntryChip(
                  label: 'Proxies',
                  icon: Icons.lan_outlined,
                  subtitle: 'Phase 4 — groups & members',
                  onTap: () => controller.openRoute(HomeRoute.proxies),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: EntryChip(
                  label: 'Profiles',
                  icon: Icons.folder_copy_outlined,
                  subtitle: selectedProfile?.name,
                  onTap: () => controller.openRoute(HomeRoute.profiles),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 8),
        Expanded(
          child: Row(
            children: [
              Expanded(
                child: EntryChip(
                  label: 'Connections',
                  icon: Icons.swap_horiz,
                  subtitle: '${controller.activeConnections.length} active',
                  onTap: () => controller.openRoute(HomeRoute.connections),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'Requests',
                  icon: Icons.http_outlined,
                  subtitle: '${controller.recentRequests.length} captured',
                  onTap: () => controller.openRoute(HomeRoute.requests),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'Logs',
                  icon: Icons.article_outlined,
                  subtitle: '${controller.recentLogs.length} entries',
                  onTap: () => controller.openRoute(HomeRoute.logs),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 8),
        Expanded(
          child: Row(
            children: [
              Expanded(
                child: EntryChip(
                  label: 'Basic',
                  icon: Icons.tune,
                  onTap: () => controller.openRoute(HomeRoute.settingsBasic),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'Network',
                  icon: Icons.cable,
                  onTap: () => controller.openRoute(HomeRoute.settingsNetwork),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'DNS',
                  icon: Icons.dns_outlined,
                  onTap: () => controller.openRoute(HomeRoute.settingsDns),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'Advanced',
                  icon: Icons.science_outlined,
                  onTap: () => controller.openRoute(HomeRoute.settingsAdvanced),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
