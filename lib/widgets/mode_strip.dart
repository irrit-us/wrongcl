import 'package:flutter/material.dart';

import '../control_state.dart';
import '../theme/wrongcl_colors.dart';

class ModeStrip extends StatelessWidget {
  const ModeStrip({
    super.key,
    required this.slots,
    required this.activeId,
    required this.onSelect,
    required this.onAdd,
    required this.disabledReason,
  });

  final List<ModeSlot> slots;
  final String activeId;
  final ValueChanged<String> onSelect;
  final VoidCallback onAdd;
  final String disabledReason;

  @override
  Widget build(BuildContext context) {
    final palette = context.wrongclColors;
    final canAdd = slots.length < kMaxModeSlots;

    return Container(
      decoration: BoxDecoration(
        color: palette.topBar.background,
        borderRadius: const BorderRadius.all(Radius.circular(14)),
        border: Border.all(color: palette.border.subtle),
      ),
      child: Row(
        children: [
          for (var i = 0; i < kMaxModeSlots + 1; i++)
            Expanded(child: _buildCell(context, i, canAdd)),
        ],
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
    final isActive = slot.id == activeId;
    final disabled = disabledReason.isNotEmpty;
    final foreground = disabled
        ? palette.topBar.foregroundMuted
        : palette.topBar.foreground;
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
              slot.name,
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

  Widget _buildAddCell(BuildContext context) {
    final palette = context.wrongclColors;
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
              Text(
                'Add',
                style: TextStyle(color: palette.topBar.foregroundMuted),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
