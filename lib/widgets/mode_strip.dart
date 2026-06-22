import 'package:flutter/material.dart';

import '../control_state.dart';

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
    final canAdd = slots.length < kMaxModeSlots;

    return Container(
      decoration: const BoxDecoration(
        color: Color(0xFF1F2933),
        borderRadius: BorderRadius.all(Radius.circular(14)),
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
    final isActive = slot.id == activeId;
    final disabled = disabledReason.isNotEmpty;
    return Tooltip(
      message: disabled ? disabledReason : '',
      child: InkWell(
        onTap: disabled ? null : () => onSelect(slot.id),
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 8),
          decoration: BoxDecoration(
            color: isActive ? const Color(0xFF2F4858) : Colors.transparent,
            borderRadius: BorderRadius.circular(10),
          ),
          child: Center(
            child: Text(
              slot.name,
              style: TextStyle(
                color: disabled ? Colors.white38 : Colors.white,
                fontWeight: isActive ? FontWeight.w600 : FontWeight.w400,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildAddCell(BuildContext context) {
    return InkWell(
      onTap: onAdd,
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 8),
        child: const Center(
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.add, color: Colors.white70, size: 16),
              SizedBox(width: 4),
              Text('Add', style: TextStyle(color: Colors.white70)),
            ],
          ),
        ),
      ),
    );
  }
}
