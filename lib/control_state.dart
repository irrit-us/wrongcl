import 'health_view.dart';

class ControlAvailability {
  const ControlAvailability({
    required this.supported,
    required this.enabled,
    required this.disabledReason,
  });

  final bool supported;
  final bool enabled;
  final String disabledReason;
}

enum AgentMode { rule, global, direct, script }

class ScriptOption {
  const ScriptOption({required this.id, required this.name});

  final String id;
  final String name;
}

class ScriptSelectionState {
  const ScriptSelectionState({
    required this.supported,
    required this.selectedId,
    required this.selectedLabel,
    required this.options,
    required this.disabledReason,
  });

  final bool supported;
  final String? selectedId;
  final String? selectedLabel;
  final List<ScriptOption> options;
  final String disabledReason;
}

class DashboardActivityEntry {
  const DashboardActivityEntry({
    required this.title,
    required this.detail,
    required this.success,
    required this.timestamp,
  });

  final String title;
  final String detail;
  final bool success;
  final DateTime timestamp;
}

enum DashboardSignalTone { neutral, accent, healthy, warning, danger }

class DashboardSeriesPoint {
  const DashboardSeriesPoint({
    required this.timestamp,
    required this.value,
  });

  final DateTime timestamp;
  final double value;
}

class DashboardTrendSeries {
  const DashboardTrendSeries({
    required this.id,
    required this.label,
    required this.points,
    this.tone = DashboardSignalTone.neutral,
  });

  final String id;
  final String label;
  final List<DashboardSeriesPoint> points;
  final DashboardSignalTone tone;
}

class DashboardSignalEvent {
  const DashboardSignalEvent({
    required this.id,
    required this.label,
    required this.timestamp,
    required this.success,
    this.tone = DashboardSignalTone.neutral,
  });

  final String id;
  final String label;
  final DateTime timestamp;
  final bool success;
  final DashboardSignalTone tone;
}

class DashboardSignalSnapshot {
  const DashboardSignalSnapshot({
    required this.activeConnectionsSeries,
    required this.totalConnectionsSeries,
    required this.failedConnectionsSeries,
    required this.uploadedBytesSeries,
    required this.downloadedBytesSeries,
    required this.recentProbeOutcomes,
    required this.recentRuntimeStateChanges,
  });

  final DashboardTrendSeries activeConnectionsSeries;
  final DashboardTrendSeries totalConnectionsSeries;
  final DashboardTrendSeries failedConnectionsSeries;
  final DashboardTrendSeries uploadedBytesSeries;
  final DashboardTrendSeries downloadedBytesSeries;
  final List<DashboardSignalEvent> recentProbeOutcomes;
  final List<DashboardSignalEvent> recentRuntimeStateChanges;
}

class DashboardSnapshot {
  const DashboardSnapshot({
    required this.running,
    required this.busy,
    required this.statusText,
    required this.stackSummary,
    required this.nativeInfo,
    required this.systemProxy,
    required this.tun,
    required this.agentModeSupported,
    required this.selectedAgentMode,
    required this.agentModeDisabledReason,
    required this.scriptSelection,
    required this.stats,
    required this.activityEntries,
    required this.healthSnapshot,
    required this.lastError,
    required this.importSummary,
    required this.signalSnapshot,
  });

  final bool running;
  final bool busy;
  final String statusText;
  final String stackSummary;
  final String nativeInfo;
  final ControlAvailability systemProxy;
  final ControlAvailability tun;
  final bool agentModeSupported;
  final AgentMode selectedAgentMode;
  final String agentModeDisabledReason;
  final ScriptSelectionState scriptSelection;
  final Map<String, Object?> stats;
  final List<DashboardActivityEntry> activityEntries;
  final HealthProbeSnapshot? healthSnapshot;
  final HealthErrorSnapshot? lastError;
  final String importSummary;
  final DashboardSignalSnapshot signalSnapshot;
}
