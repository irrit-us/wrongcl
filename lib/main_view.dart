import 'package:flutter/material.dart';

import 'client_home_controller.dart';
import 'l10n/app_localizations.dart';
import 'theme/wrongcl_colors.dart';
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
    final iconOnRight = iconSide == ChipIconSide.right;
    final l10n = AppLocalizations.of(context);
    final blocks = <Widget>[
      Expanded(
        flex: 2,
        child: _InspectBlock(controller: controller, iconSide: iconSide),
      ),
      const SizedBox(width: 8),
      Expanded(
        flex: 3,
        child: _SettingsBlock(controller: controller, iconSide: iconSide),
      ),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: Row(
            children: [
              Expanded(
                child: EntryChip(
                  label: l10n.navProxies,
                  icon: Icons.lan_outlined,
                  subtitle: activeProxy,
                  iconSide: iconSide,
                  onTap: () => controller.openRoute(HomeRoute.proxies),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: EntryChip(
                  label: l10n.navProfiles,
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
            children: iconOnRight ? blocks.reversed.toList() : blocks,
          ),
        ),
      ],
    );
  }
}

class _InspectBlock extends StatelessWidget {
  const _InspectBlock({required this.controller, required this.iconSide});

  final ClientHomeController controller;
  final ChipIconSide iconSide;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final iconOnRight = iconSide == ChipIconSide.right;
    final l10n = AppLocalizations.of(context);
    final topRow = <Widget>[
      Expanded(child: _BlockTitle(label: l10n.navInspect, iconOnRight: iconOnRight)),
      const SizedBox(width: 8),
      Expanded(
        child: EntryChip(
          label: l10n.navConnections,
          icon: Icons.swap_horiz,
          trailing: '${controller.activeConnections.length}',
          iconSide: iconSide,
          onTap: () => controller.openRoute(HomeRoute.connections),
        ),
      ),
    ];
    final bottomRow = <Widget>[
      Expanded(
        child: EntryChip(
          label: l10n.navRequests,
          icon: Icons.http_outlined,
          trailing: '${controller.recentRequests.length}',
          iconSide: iconSide,
          onTap: () => controller.openRoute(HomeRoute.requests),
        ),
      ),
      const SizedBox(width: 8),
      Expanded(
        child: EntryChip(
          label: l10n.navLogs,
          icon: Icons.article_outlined,
          trailing: '${controller.recentLogs.length}',
          iconSide: iconSide,
          onTap: () => controller.openRoute(HomeRoute.logs),
        ),
      ),
    ];
    return Container(
      padding: const EdgeInsets.all(6),
      decoration: BoxDecoration(
        color: palette.surface.surfaceMuted,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Column(
        children: [
          Expanded(
            child: Row(
              children: iconOnRight ? topRow.reversed.toList() : topRow,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: Row(
              children: iconOnRight ? bottomRow.reversed.toList() : bottomRow,
            ),
          ),
        ],
      ),
    );
  }
}

class _SettingsBlock extends StatelessWidget {
  const _SettingsBlock({required this.controller, required this.iconSide});

  final ClientHomeController controller;
  final ChipIconSide iconSide;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final iconOnRight = iconSide == ChipIconSide.right;
    final l10n = AppLocalizations.of(context);
    final topRow = <Widget>[
      Expanded(
        flex: 2,
        child: _BlockTitle(label: l10n.navSettings, iconOnRight: iconOnRight),
      ),
      const SizedBox(width: 8),
      Expanded(
        flex: 2,
        child: EntryChip(
          label: l10n.navBasic,
          icon: Icons.tune,
          iconSide: iconSide,
          onTap: () => controller.openRoute(HomeRoute.settingsBasic),
        ),
      ),
      const SizedBox(width: 8),
      Expanded(
        flex: 2,
        child: EntryChip(
          label: l10n.navNetwork,
          icon: Icons.cable,
          iconSide: iconSide,
          onTap: () => controller.openRoute(HomeRoute.settingsNetwork),
        ),
      ),
    ];
    final bottomRow = <Widget>[
      const Spacer(),
      Expanded(
        flex: 2,
        child: EntryChip(
          label: l10n.navDns,
          icon: Icons.dns_outlined,
          iconSide: iconSide,
          onTap: () => controller.openRoute(HomeRoute.settingsDns),
        ),
      ),
      const SizedBox(width: 8),
      Expanded(
        flex: 2,
        child: EntryChip(
          label: l10n.navAdvanced,
          icon: Icons.science_outlined,
          iconSide: iconSide,
          onTap: () => controller.openRoute(HomeRoute.settingsAdvanced),
        ),
      ),
      const Spacer(),
    ];
    return Container(
      padding: const EdgeInsets.all(6),
      decoration: BoxDecoration(
        color: palette.surface.surfaceHighlight,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Column(
        children: [
          Expanded(
            child: Row(
              children: iconOnRight ? topRow.reversed.toList() : topRow,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: Row(
              children: iconOnRight ? bottomRow.reversed.toList() : bottomRow,
            ),
          ),
        ],
      ),
    );
  }
}

class _BlockTitle extends StatelessWidget {
  const _BlockTitle({required this.label, required this.iconOnRight});

  final String label;
  final bool iconOnRight;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      child: Align(
        alignment: iconOnRight ? Alignment.centerRight : Alignment.centerLeft,
        child: FittedBox(
          fit: BoxFit.scaleDown,
          alignment: iconOnRight ? Alignment.centerRight : Alignment.centerLeft,
          child: Text(
            label,
            maxLines: 1,
            style: theme.textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.w700,
              color: palette.text.primary,
            ),
          ),
        ),
      ),
    );
  }
}
