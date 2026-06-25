import 'package:flutter/material.dart';

import 'client_home_controller.dart';
import 'widgets/control_column.dart';
import 'widgets/entry_chip.dart';
import 'widgets/mode_strip.dart';
import 'widgets/traffic_chart.dart';
import 'widgets/traffic_stats.dart';

class MainView extends StatelessWidget {
  const MainView({
    super.key,
    required this.controller,
    this.chipIconSide = ChipIconSide.left,
  });

  final ClientHomeController controller;
  final ChipIconSide chipIconSide;

  @override
  Widget build(BuildContext context) {
    final snapshot = controller.dashboardSnapshot;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ModeStrip(
              slots: controller.modeSlots,
              activeId: controller.activeModeId,
              onSelect: controller.setActiveMode,
              onAdd: controller.openAddMode,
              disabledReason: controller.modeStripDisabledReason,
              iconSide: chipIconSide,
            ),
            const SizedBox(height: 10),
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
                      iconSide: chipIconSide,
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
                      uploadSeries: snapshot.signalSnapshot.uploadedBytesSeries,
                      downloadSeries:
                          snapshot.signalSnapshot.downloadedBytesSeries,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 10),
            Expanded(
              flex: 3,
              child: _EntriesGrid(
                controller: controller,
                iconSide: chipIconSide,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _EntriesGrid extends StatelessWidget {
  const _EntriesGrid({required this.controller, required this.iconSide});

  final ClientHomeController controller;
  final ChipIconSide iconSide;

  @override
  Widget build(BuildContext context) {
    final selectedProfile = controller.selectedProfile;
    final activeProxy = controller.proxyGroups.active?.name;
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
                  subtitle: activeProxy,
                  iconSide: iconSide,
                  onTap: () => controller.openRoute(HomeRoute.proxies),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: EntryChip(
                  label: 'Profiles',
                  icon: Icons.folder_copy_outlined,
                  subtitle: selectedProfile?.name,
                  iconSide: iconSide,
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
                  iconSide: iconSide,
                  onTap: () => controller.openRoute(HomeRoute.connections),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'Requests',
                  icon: Icons.http_outlined,
                  subtitle: '${controller.recentRequests.length} captured',
                  iconSide: iconSide,
                  onTap: () => controller.openRoute(HomeRoute.requests),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'Logs',
                  icon: Icons.article_outlined,
                  subtitle: '${controller.recentLogs.length} entries',
                  iconSide: iconSide,
                  onTap: () => controller.openRoute(HomeRoute.logs),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: EntryChip(
                  label: 'Basic',
                  icon: Icons.tune,
                  iconSide: iconSide,
                  onTap: () => controller.openRoute(HomeRoute.settingsBasic),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'Network',
                  icon: Icons.cable,
                  iconSide: iconSide,
                  onTap: () => controller.openRoute(HomeRoute.settingsNetwork),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'DNS',
                  icon: Icons.dns_outlined,
                  iconSide: iconSide,
                  onTap: () => controller.openRoute(HomeRoute.settingsDns),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: EntryChip(
                  label: 'Advanced',
                  icon: Icons.science_outlined,
                  iconSide: iconSide,
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
