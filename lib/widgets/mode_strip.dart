import 'package:flutter/material.dart';

import '../control_state.dart';
import '../l10n/app_localizations.dart';
import '../theme/wrongcl_colors.dart';
import 'entry_chip.dart';

class ModeStrip extends StatelessWidget {
  const ModeStrip({
    super.key,
    required this.slots,
    required this.activeId,
    required this.onSelect,
    required this.onAdd,
    required this.disabledReason,
    this.iconSide = ChipIconSide.left,
  });

  final List<ModeSlot> slots;
  final String activeId;
  final ValueChanged<String> onSelect;
  final VoidCallback onAdd;
  final String disabledReason;
  final ChipIconSide iconSide;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final canAdd = slots.length < kMaxModeSlots;
    final iconOnRight = iconSide == ChipIconSide.right;

    final cells = <Widget>[
      for (var i = 0; i < kMaxModeSlots + 1; i++)
        Expanded(child: _buildCell(context, i, canAdd)),
    ];

    return MediaQuery(
      data: MediaQuery.of(context).copyWith(
        textScaler: const TextScaler.linear(1.4),
      ),
      child: Container(
        decoration: BoxDecoration(
          color: palette.topBar.background,
          borderRadius: const BorderRadius.all(Radius.circular(14)),
          border: Border.all(color: palette.border.subtle),
        ),
        child: Row(
          children: iconOnRight ? cells.reversed.toList() : cells,
        ),
      ),
    );
  }

  Widget _buildCell(BuildContext context, int index, bool canAdd) {
    if (index == 0) {
      return _buildIconCell(context);
    }
    final slotIndex = index - 1;
    if (slotIndex < slots.length) {
      return _buildModeCell(context, slots[slotIndex]);
    }
    if (canAdd && slotIndex == slots.length) {
      return _buildAddCell(context);
    }
    return const SizedBox.shrink();
  }

  Widget _buildIconCell(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 10),
      child: Center(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(8),
          child: Image.asset(
            'assets/brand/wrongcl_app_mark.png',
            width: 28,
            height: 28,
            fit: BoxFit.cover,
          ),
        ),
      ),
    );
  }

  Widget _buildModeCell(BuildContext context, ModeSlot slot) {
    final palette = context.wrongclColors;
    final l10n = AppLocalizations.of(context);
    final isActive = slot.id == activeId;
    final disabled = disabledReason.isNotEmpty;
    final foreground = disabled
        ? palette.topBar.foregroundMuted
        : palette.topBar.foreground;
    final displayName = _localizedSlotName(l10n, slot);
    return Tooltip(
      message: disabled ? disabledReason : '',
      child: InkWell(
        onTap: disabled ? null : () => onSelect(slot.id),
        borderRadius: BorderRadius.circular(10),
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 8),
          decoration: BoxDecoration(
            color: isActive ? palette.topBar.activeCell : Colors.transparent,
            borderRadius: BorderRadius.circular(10),
            border: isActive
                ? Border.all(color: palette.topBar.activeBorder)
                : null,
          ),
          child: Center(
            child: Text(
              displayName,
              textAlign: TextAlign.center,
              style: TextStyle(
                color: foreground,
                fontWeight: isActive ? FontWeight.w600 : FontWeight.w500,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ),
      ),
    );
  }

  String _localizedSlotName(AppLocalizations l10n, ModeSlot slot) {
    if (!slot.builtin) return slot.name;
    switch (slot.id) {
      case 'global':
        return l10n.modeGlobal;
      case 'rule':
        return l10n.modeRule;
      case 'direct':
        return l10n.modeDirect;
      default:
        return slot.name;
    }
  }

  Widget _buildAddCell(BuildContext context) {
    final palette = context.wrongclColors;
    final l10n = AppLocalizations.of(context);
    return InkWell(
      onTap: onAdd,
      borderRadius: BorderRadius.circular(10),
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 8),
        child: Center(
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.add, color: palette.topBar.foregroundMuted, size: 16),
              const SizedBox(width: 4),
              Flexible(
                child: Text(
                  l10n.modeAdd,
                  style: TextStyle(color: palette.topBar.foregroundMuted),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
