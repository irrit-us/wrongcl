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

class ModeSlot {
  const ModeSlot({
    required this.id,
    required this.name,
    required this.builtin,
  });

  final String id;
  final String name;
  final bool builtin;
}

const List<ModeSlot> kBuiltinModeSlots = [
  ModeSlot(id: 'global', name: 'Global', builtin: true),
  ModeSlot(id: 'rule', name: 'Rule', builtin: true),
  ModeSlot(id: 'direct', name: 'Direct', builtin: true),
];

const int kMaxModeSlots = 6;

class ConnectionInfo {
  const ConnectionInfo({
    required this.id,
    required this.target,
    required this.sourceApp,
    required this.bytesUp,
    required this.bytesDown,
    required this.startedAt,
  });

  final int id;
  final String target;
  final String sourceApp;
  final int bytesUp;
  final int bytesDown;
  final DateTime startedAt;
}

class RequestInfo {
  const RequestInfo({
    required this.id,
    required this.target,
    required this.sourceApp,
    required this.timestamp,
    this.method = '',
    this.url,
    this.host,
    this.sourcePid,
  });

  final int id;
  final String target;
  final String sourceApp;
  final DateTime timestamp;
  final String method;
  final String? url;
  final String? host;
  final int? sourcePid;
}

class LogEntry {
  const LogEntry({
    required this.timestamp,
    required this.level,
    required this.target,
    required this.message,
  });

  final DateTime timestamp;
  final String level;
  final String target;
  final String message;
}
