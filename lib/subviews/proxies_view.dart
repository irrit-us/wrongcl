import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../l10n/app_localizations.dart';
import '../theme/wrongcl_colors.dart';
import '../widgets/subpage_scaffold.dart';
import '../wrongcl_client.dart';

class ProxiesView extends StatelessWidget {
  const ProxiesView({
    super.key,
    required this.controller,
    required this.onClose,
  });

  final ClientHomeController controller;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final snapshot = controller.proxyGroups;
    final activeKind = snapshot.active?.kind;
    final activeName = snapshot.active?.name ?? '';

    final Widget body;
    if (!controller.running) {
      body = _Notice(
        message: l10n.proxiesEmptyStopped,
      );
    } else if (snapshot.endpoints.isEmpty) {
      body = _Notice(
        message: l10n.proxiesEmptyNoEndpoints,
      );
    } else {
      body = ListView(
        padding: const EdgeInsets.all(16),
        children: [
          if (controller.proxyGroupsStatus.isNotEmpty) ...[
            _StatusBanner(message: controller.proxyGroupsStatus),
            const SizedBox(height: 12),
          ],
          _ActiveBanner(
            activeKind: activeKind,
            activeName: activeName,
          ),
          const SizedBox(height: 16),
          for (final group in snapshot.groups) ...[
            _GroupCard(
              group: group,
              endpoints: snapshot.endpoints,
              isActiveGroup:
                  activeKind == 'group' && activeName == group.name,
              onSelect: (member) => controller.selectProxyGroupMember(
                group.name,
                member,
              ),
            ),
            const SizedBox(height: 12),
          ],
          const SizedBox(height: 8),
          Text(
            l10n.proxiesEndpoints,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 6),
          for (final endpoint in snapshot.endpoints)
            _EndpointTile(
              endpoint: endpoint,
              isActive: activeKind == 'endpoint' &&
                  activeName == endpoint.name,
            ),
        ],
      );
    }

    return SubpageScaffold(
      title: l10n.navProxies,
      onClose: onClose,
      child: body,
    );
  }
}

class _ActiveBanner extends StatelessWidget {
  const _ActiveBanner({required this.activeKind, required this.activeName});

  final String? activeKind;
  final String activeName;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final l10n = AppLocalizations.of(context);
    final label = activeKind == null || activeName.isEmpty
        ? l10n.proxiesNoActiveSelection
        : l10n.proxiesActiveLabel(activeKind!, activeName);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: palette.surface.surfaceTinted,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: palette.border.regular),
      ),
      child: Row(
        children: [
          Icon(
            Icons.flag_outlined,
            size: 18,
            color: palette.border.accent,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(label, style: Theme.of(context).textTheme.bodyMedium),
          ),
        ],
      ),
    );
  }
}

class _GroupCard extends StatelessWidget {
  const _GroupCard({
    required this.group,
    required this.endpoints,
    required this.isActiveGroup,
    required this.onSelect,
  });

  final ProxyGroupInfo group;
  final List<ProxyEndpointInfo> endpoints;
  final bool isActiveGroup;
  final ValueChanged<String> onSelect;

  ProxyEndpointInfo? _findEndpoint(String name) {
    for (final endpoint in endpoints) {
      if (endpoint.name == name) return endpoint;
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final canSelect = group.kind == ProxyGroupKind.select;
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: palette.surface.surfaceRaised,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: isActiveGroup
              ? palette.border.accent
              : palette.border.regular,
          width: isActiveGroup ? 1.4 : 1,
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                group.name,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 8,
                  vertical: 2,
                ),
                decoration: BoxDecoration(
                  color: palette.surface.surfaceMuted,
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: palette.border.regular),
                ),
                child: Text(
                  group.kind.label,
                  style: Theme.of(context).textTheme.labelSmall,
                ),
              ),
              const Spacer(),
              if (!canSelect)
                Text(AppLocalizations.of(context).proxiesAuto, style: Theme.of(context).textTheme.labelSmall),
            ],
          ),
          const SizedBox(height: 8),
          for (final member in group.members)
            _MemberRow(
              member: member,
              endpoint: _findEndpoint(member),
              selected: group.selected == member,
              enabled: canSelect,
              onTap: canSelect ? () => onSelect(member) : null,
            ),
        ],
      ),
    );
  }
}

class _MemberRow extends StatelessWidget {
  const _MemberRow({
    required this.member,
    required this.endpoint,
    required this.selected,
    required this.enabled,
    required this.onTap,
  });

  final String member;
  final ProxyEndpointInfo? endpoint;
  final bool selected;
  final bool enabled;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final subtitle = endpoint == null
        ? AppLocalizations.of(context).proxiesUnknownEndpoint
        : '${endpoint!.host}:${endpoint!.port} - ${endpoint!.stack}';
    final iconColor = enabled
        ? (selected ? palette.status.success : palette.border.accent)
        : palette.accent.soft;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          borderRadius: BorderRadius.circular(8),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
            child: Row(
              children: [
                Icon(
                  selected
                      ? Icons.radio_button_checked
                      : Icons.radio_button_off,
                  size: 18,
                  color: iconColor,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        member,
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                      Text(
                        subtitle,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _EndpointTile extends StatelessWidget {
  const _EndpointTile({required this.endpoint, required this.isActive});

  final ProxyEndpointInfo endpoint;
  final bool isActive;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Container(
      margin: const EdgeInsets.only(bottom: 6),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: palette.surface.surfaceRaised,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: isActive
              ? palette.border.accent
              : palette.border.regular,
          width: isActive ? 1.4 : 1,
        ),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  endpoint.name,
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
                Text(
                  '${endpoint.host}:${endpoint.port} - ${endpoint.stack}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
          if (isActive)
            Icon(
              Icons.check_circle,
              color: palette.status.success,
              size: 18,
            ),
        ],
      ),
    );
  }
}

class _StatusBanner extends StatelessWidget {
  const _StatusBanner({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: palette.surface.surfaceAccent,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: palette.border.regular),
      ),
      child: Text(message, style: Theme.of(context).textTheme.bodySmall),
    );
  }
}

class _Notice extends StatelessWidget {
  const _Notice({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Container(
          padding: const EdgeInsets.all(20),
          decoration: BoxDecoration(
            color: palette.surface.surfaceMuted,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: palette.border.regular),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.info_outline,
                size: 28,
                color: palette.text.secondary,
              ),
              const SizedBox(height: 10),
              Text(
                message,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
