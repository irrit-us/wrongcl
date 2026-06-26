import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/widgets.dart';

import 'autostart_manager.dart';
import 'control_state.dart';
import 'desktop_shell_controller.dart';
import 'health_view.dart';
import 'profile_store.dart';
import 'system_proxy_manager.dart';
import 'wrongcl_client.dart';

export 'control_state.dart' show ConnectionInfo, RequestInfo, LogEntry;

enum HomeRoute {
  dashboard,
  profiles,
  proxies,
  connections,
  requests,
  logs,
  modePicker,
  settingsBasic,
  settingsNetwork,
  settingsDns,
  settingsAdvanced,
}

enum LogLevelFilter {
  all('All logs'),
  error('Error only'),
  warn('Warn and error'),
  info('Info and above'),
  debug('Debug and above');

  const LogLevelFilter(this.label);

  final String label;

  bool allows(String level) {
    if (this == LogLevelFilter.all) {
      return true;
    }
    return _severity(level) >= _minimumSeverity;
  }

  int get _minimumSeverity => switch (this) {
    LogLevelFilter.all => -1,
    LogLevelFilter.debug => 0,
    LogLevelFilter.info => 1,
    LogLevelFilter.warn => 2,
    LogLevelFilter.error => 3,
  };

  static int _severity(String level) => switch (level.toUpperCase()) {
    'ERROR' => 3,
    'WARN' => 2,
    'INFO' => 1,
    'DEBUG' => 0,
    _ => 0,
  };
}

class ClientHomeController extends ChangeNotifier {
  static const _maxSignalHistoryPoints = 60;
  static const _maxSignalEvents = 16;

  ClientHomeController({
    required this.client,
    required this.profileStore,
    required this.autostartManager,
    required this.systemProxyManager,
    required this.desktopShellController,
  }) {
    _draftConfig = _defaultDraftConfig();
  }

  final WrongclClient client;
  final ProfileStore profileStore;
  final AutostartManager autostartManager;
  final SystemProxyManager systemProxyManager;
  final DesktopShellController desktopShellController;
  late ClientConfigInput _draftConfig;

  final profileName = TextEditingController(text: 'default');
  final clientConfigPath = TextEditingController();
  final rawConfigEditor = TextEditingController();
  final wrongsvConfigPath = TextEditingController();
  final wrongsvServerHost = TextEditingController(text: '127.0.0.1');
  final wrongsvListenHost = TextEditingController(text: '127.0.0.1');
  final wrongsvListenPort = TextEditingController(text: '1080');

  final serverHost = TextEditingController(text: '127.0.0.1');
  final serverPort = TextEditingController(text: '443');
  final uuid = TextEditingController(
    text: '12345678-1234-1234-1234-123456789abc',
  );
  final naiveUsername = TextEditingController();
  final naivePassword = TextEditingController();
  final naivePaddingHeaderName = TextEditingController(text: 'Padding');
  final hysteria2ServerName = TextEditingController(text: 'foo.cloudfront.net');
  final hysteria2Password = TextEditingController();
  final hysteria2ObfsPassword = TextEditingController();
  final hysteria2ObfsMinPacketSize = TextEditingController();
  final hysteria2ObfsMaxPacketSize = TextEditingController();
  final tuicServerName = TextEditingController(text: 'foo.cloudfront.net');
  final tuicUuid = TextEditingController(
    text: '12345678-1234-1234-1234-123456789abc',
  );
  final tuicPassword = TextEditingController();
  final trojanPassword = TextEditingController();
  final mixedUsername = TextEditingController();
  final mixedPassword = TextEditingController();
  final shadowsocksPassword = TextEditingController();
  final wireguardPrivateKey = TextEditingController();
  final wireguardPeerPublicKey = TextEditingController();
  final wireguardPreSharedKey = TextEditingController();
  final wireguardClientIp = TextEditingController(text: '10.66.66.2/32');
  final wireguardAllowedIps = TextEditingController(text: '10.66.66.1/32');
  final wireguardMtu = TextEditingController(text: '1400');
  final kcpSeed = TextEditingController();
  final kcpMtu = TextEditingController(text: '1350');
  final kcpTti = TextEditingController(text: '50');
  final meekPath = TextEditingController(text: '/');
  final meekHost = TextEditingController();
  final gdocsviewerPathPrefix = TextEditingController(text: '/gdocsviewer');
  final gdocsviewerSharedKey = TextEditingController();
  final quicServerName = TextEditingController(text: 'cloudfront.net');
  final webtransportAuthority = TextEditingController(text: 'cloudfront.net');
  final webtransportPath = TextEditingController(text: '/wt');
  final wsPath = TextEditingController(text: '/ws');
  final wsHost = TextEditingController();
  final huPath = TextEditingController(text: '/up');
  final huHost = TextEditingController();
  final xhttpPath = TextEditingController(text: '/xhttp');
  final xhttpHost = TextEditingController();
  final grpcServiceName = TextEditingController(text: 'GunService');
  final tlsServerName = TextEditingController();
  final tlsAlpn = TextEditingController(text: 'h2, http/1.1');
  final realityServerName = TextEditingController(text: 'www.microsoft.com');
  final realityPublicKey = TextEditingController();
  final realityShortId = TextEditingController();
  final realityRawPubkey = TextEditingController();
  final anytlsServerName = TextEditingController();
  final anytlsPassword = TextEditingController();
  final shadowTlsServerName = TextEditingController(text: 'cloudfront.net');
  final shadowTlsPassword = TextEditingController();

  final localHost = TextEditingController(text: '127.0.0.1');
  final localPort = TextEditingController(text: '1080');
  final targetHost = TextEditingController(text: 'example.com');
  final targetPort = TextEditingController(text: '80');
  final payload = TextEditingController(
    text: 'HEAD / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n',
  );

  ProxyKind proxyKind = ProxyKind.vless;
  TransportKind transportKind = TransportKind.raw;
  OuterSecurityKind outerSecurityKind = OuterSecurityKind.none;
  bool hysteria2UdpEnabled = true;
  String hysteria2ObfsType = '';
  bool quicUdpEnabled = true;
  bool webtransportUdpEnabled = true;
  bool tlsInsecure = false;
  bool vlessVisionFlow = false;
  bool anytlsInsecureSkipVerify = true;
  bool localSocksEnabled = true;
  bool localHttpEnabled = true;

  bool busy = false;
  bool running = false;
  bool desktopShellSyncScheduled = false;
  String stackSummary = '';
  String nativeInfo = 'Native Rust client not checked';
  String profilesStatus = '';
  String status = 'Stopped';
  Map<String, Object?> stats = const {};
  List<SavedProfile> savedProfiles = const [];
  List<DashboardActivityEntry> activityLog = const [];
  List<DashboardSeriesPoint> activeConnectionsHistory = const [];
  List<DashboardSeriesPoint> totalConnectionsHistory = const [];
  List<DashboardSeriesPoint> failedConnectionsHistory = const [];
  List<DashboardSeriesPoint> uploadedBytesHistory = const [];
  List<DashboardSeriesPoint> downloadedBytesHistory = const [];
  List<DashboardSignalEvent> recentProbeOutcomes = const [];
  List<DashboardSignalEvent> recentRuntimeStateChanges = const [];
  String? selectedProfileId;
  AutostartStatus? autostartStatus;
  SystemProxyStatus? systemProxyStatus;
  WrongsvCapabilityReport? wrongsvReport;
  WrongsvAdaptResult? wrongsvAdaptResult;
  NativeResponse? lastResponse;
  HealthProbeSnapshot? lastProbe;
  HealthErrorSnapshot? lastError;
  HomeRoute activeRoute = HomeRoute.dashboard;

  AgentMode selectedAgentMode = AgentMode.rule;

  RouterSnapshot get currentRouterSnapshot =>
      running ? routerSnapshot : RouterSnapshot.fromConfig(buildConfig());

  ProxyGroupsSnapshot get currentProxyGroups =>
      running ? proxyGroups : ProxyGroupsSnapshot.fromConfig(buildConfig());

  List<ModeSlot> get modeSlots {
    final modes = currentRouterSnapshot.modes;
    if (modes.isEmpty) {
      return const [...kBuiltinModeSlots];
    }
    return [
      for (final m in modes)
        ModeSlot(
          id: m.name,
          name: _displayModeName(m.name),
          builtin: m.kind != 'user',
        ),
    ];
  }

  String get activeModeId => currentRouterSnapshot.activeMode ?? 'global';

  static String _displayModeName(String id) {
    if (id.isEmpty) return id;
    return id[0].toUpperCase() + id.substring(1);
  }

  static ClientConfigInput _defaultDraftConfig() {
    return const ClientConfigInput(
      endpoints: [
        NamedEndpointInput(
          name: 'default',
          host: '127.0.0.1',
          port: 443,
          endpoint: EndpointConfig(
            proxy: {
              'type': 'vless',
              'uuid': '12345678-1234-1234-1234-123456789abc',
              'flow': '',
            },
            transport: {'type': 'raw'},
            outerSecurity: {'type': 'none'},
          ),
        ),
      ],
      active: ActiveSelectionInput.endpoint('default'),
      localHost: '127.0.0.1',
      localPort: 1080,
    );
  }

  List<ConnectionInfo> activeConnections = const [];
  List<RequestInfo> recentRequests = const [];
  List<LogEntry> recentLogs = const [];
  ProxyGroupsSnapshot proxyGroups = ProxyGroupsSnapshot.empty;
  String proxyGroupsStatus = '';
  RouterSnapshot routerSnapshot = RouterSnapshot.empty;
  String routerStatus = '';
  String dnsStatus = '';
  LogLevelFilter logLevelFilter = LogLevelFilter.all;
  bool tunPreparationAvailable = false;
  ControlAvailability tunStatus = const ControlAvailability(
    supported: false,
    enabled: false,
    disabledReason: 'Loading TUN status...',
    platform: 'unknown',
  );

  Timer? _statusPollTimer;
  Timer? _connectionsPollTimer;
  Timer? _logsPollTimer;
  Timer? _requestsPollTimer;
  Timer? _proxyGroupsPollTimer;
  Timer? _routerPollTimer;
  int _logCursor = 0;
  int _requestCursor = 0;
  bool _pollingConnections = false;
  bool _pollingLogs = false;
  bool _pollingRequests = false;
  bool _pollingProxyGroups = false;
  bool _pollingRouter = false;
  static const int _maxLogEntries = 500;
  static const int _maxRequestEntries = 500;

  Future<void> init() async {
    profileName.addListener(_scheduleDesktopShellSync);
    final version = client.version();
    nativeInfo = version.ok ? _formatVersion(version.data) : version.message;
    _attachFieldListeners();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      syncRawConfigEditorFromDraft();
      refreshStack();
      unawaited(refreshStatus());
    });
    unawaited(_attachDesktopShell());
    unawaited(loadProfiles());
    unawaited(loadAutostartStatus());
    unawaited(loadSystemProxyStatus());
    unawaited(loadTunStatus());
  }

  @override
  void dispose() {
    _statusPollTimer?.cancel();
    _statusPollTimer = null;
    _connectionsPollTimer?.cancel();
    _connectionsPollTimer = null;
    _logsPollTimer?.cancel();
    _logsPollTimer = null;
    _requestsPollTimer?.cancel();
    _requestsPollTimer = null;
    _proxyGroupsPollTimer?.cancel();
    _proxyGroupsPollTimer = null;
    _routerPollTimer?.cancel();
    _routerPollTimer = null;
    profileName.removeListener(_scheduleDesktopShellSync);
    unawaited(desktopShellController.dispose());
    for (final controller in [
      profileName,
      clientConfigPath,
      rawConfigEditor,
      wrongsvConfigPath,
      wrongsvServerHost,
      wrongsvListenHost,
      wrongsvListenPort,
      serverHost,
      serverPort,
      uuid,
      naiveUsername,
      naivePassword,
      naivePaddingHeaderName,
      hysteria2ServerName,
      hysteria2Password,
      hysteria2ObfsPassword,
      hysteria2ObfsMinPacketSize,
      hysteria2ObfsMaxPacketSize,
      tuicServerName,
      tuicUuid,
      tuicPassword,
      trojanPassword,
      mixedUsername,
      mixedPassword,
      shadowsocksPassword,
      wireguardPrivateKey,
      wireguardPeerPublicKey,
      wireguardPreSharedKey,
      wireguardClientIp,
      wireguardAllowedIps,
      wireguardMtu,
      kcpSeed,
      kcpMtu,
      kcpTti,
      meekPath,
      meekHost,
      gdocsviewerPathPrefix,
      gdocsviewerSharedKey,
      quicServerName,
      webtransportAuthority,
      webtransportPath,
      wsPath,
      wsHost,
      huPath,
      huHost,
      xhttpPath,
      xhttpHost,
      grpcServiceName,
      tlsServerName,
      tlsAlpn,
      realityServerName,
      realityPublicKey,
      realityShortId,
      realityRawPubkey,
      anytlsServerName,
      anytlsPassword,
      shadowTlsServerName,
      shadowTlsPassword,
      localHost,
      localPort,
      targetHost,
      targetPort,
      payload,
    ]) {
      controller.dispose();
    }
    super.dispose();
  }

  DashboardSnapshot get dashboardSnapshot {
    final systemProxy = systemProxyStatus == null
        ? const ControlAvailability(
            supported: false,
            enabled: false,
            disabledReason: 'System proxy status is still loading',
            platform: 'unknown',
          )
        : ControlAvailability(
            supported: systemProxyStatus!.supported,
            enabled: systemProxyStatus!.enabled,
            disabledReason: systemProxyStatus!.supported
                ? ''
                : systemProxyStatus!.message,
            platform:
                systemProxyManager.platform?.name ?? Platform.operatingSystem,
          );
    return DashboardSnapshot(
      running: running,
      busy: busy,
      statusText: status,
      stackSummary: stackSummary,
      nativeInfo: nativeInfo,
      systemProxy: systemProxy,
      tun: tunAvailability,
      agentModeSupported: false,
      selectedAgentMode: selectedAgentMode,
      agentModeDisabledReason:
          'Agent mode control is not exposed by wrongcl yet',
      scriptSelection: const ScriptSelectionState(
        supported: false,
        selectedId: null,
        selectedLabel: null,
        options: [],
        disabledReason:
            'Script runtime selection is not exposed by wrongcl yet',
      ),
      stats: stats,
      activityEntries: activityLog,
      healthSnapshot: lastProbe,
      lastError: lastError,
      importSummary: _buildImportSummary(),
      signalSnapshot: _buildSignalSnapshot(),
    );
  }

  DashboardSignalSnapshot _buildSignalSnapshot() {
    return DashboardSignalSnapshot(
      activeConnectionsSeries: DashboardTrendSeries(
        id: 'active_connections',
        label: 'Active connections',
        points: activeConnectionsHistory,
        tone: DashboardSignalTone.healthy,
      ),
      totalConnectionsSeries: DashboardTrendSeries(
        id: 'total_connections',
        label: 'Total connections',
        points: totalConnectionsHistory,
        tone: DashboardSignalTone.accent,
      ),
      failedConnectionsSeries: DashboardTrendSeries(
        id: 'failed_connections',
        label: 'Failed connections',
        points: failedConnectionsHistory,
        tone: DashboardSignalTone.warning,
      ),
      uploadedBytesSeries: DashboardTrendSeries(
        id: 'bytes_uploaded',
        label: 'Uploaded bytes',
        points: uploadedBytesHistory,
        tone: DashboardSignalTone.accent,
      ),
      downloadedBytesSeries: DashboardTrendSeries(
        id: 'bytes_downloaded',
        label: 'Downloaded bytes',
        points: downloadedBytesHistory,
        tone: DashboardSignalTone.healthy,
      ),
      recentProbeOutcomes: recentProbeOutcomes,
      recentRuntimeStateChanges: recentRuntimeStateChanges,
    );
  }

  SavedProfile? get selectedProfile {
    final selectedId = selectedProfileId;
    if (selectedId == null) {
      return null;
    }
    for (final profile in savedProfiles) {
      if (profile.id == selectedId) {
        return profile;
      }
    }
    return null;
  }

  bool get showingSubpage => activeRoute != HomeRoute.dashboard;

  String get activeRouteLabel {
    switch (activeRoute) {
      case HomeRoute.dashboard:
        return 'Dashboard';
      case HomeRoute.profiles:
        return 'Profiles';
      case HomeRoute.proxies:
        return 'Proxies';
      case HomeRoute.connections:
        return 'Connections';
      case HomeRoute.requests:
        return 'Requests';
      case HomeRoute.logs:
        return 'Logs';
      case HomeRoute.modePicker:
        return 'Add mode';
      case HomeRoute.settingsBasic:
        return 'Basic';
      case HomeRoute.settingsNetwork:
        return 'Network';
      case HomeRoute.settingsDns:
        return 'DNS';
      case HomeRoute.settingsAdvanced:
        return 'Advanced';
    }
  }

  void openRoute(HomeRoute route) {
    activeRoute = route;
    notifyListeners();
  }

  void closeSubView() {
    activeRoute = HomeRoute.dashboard;
    notifyListeners();
  }

  ControlAvailability get tunAvailability => tunStatus;

  String get modeStripDisabledReason => '';

  DnsSettingsInput get currentDnsSettings =>
      DnsSettingsInput.fromMap(buildConfig().dns);

  void setActiveMode(String modeId) {
    if (modeId == activeModeId) {
      return;
    }
    if (!running) {
      final response = _setDraftActiveMode(modeId);
      routerStatus = response.message;
      notifyListeners();
      return;
    }
    unawaited(_setActiveModeAsync(modeId));
  }

  Future<void> _setActiveModeAsync(String modeId) async {
    final response = await Future<NativeResponse>(
      () => client.routerSetActiveMode(modeId),
    );
    if (!response.ok) {
      routerStatus = response.message;
      notifyListeners();
      return;
    }
    _setDraftActiveMode(modeId);
    await _pollRouterNow();
  }

  void openAddMode() {
    if (modeSlots.length >= kMaxModeSlots) {
      return;
    }
    openRoute(HomeRoute.modePicker);
  }

  Future<NativeResponse> upsertUserMode(RouterMode mode) async {
    final local = _upsertDraftUserMode(mode, commit: !running);
    if (!local.ok) {
      routerStatus = local.message;
      notifyListeners();
      return local;
    }
    if (!running) {
      routerStatus = local.message;
      notifyListeners();
      return local;
    }
    final response = await Future<NativeResponse>(
      () => client.routerUpsertUserMode(mode.toMap()),
    );
    if (response.ok) {
      _upsertDraftUserMode(mode);
      await _pollRouterNow();
    } else {
      routerStatus = response.message;
      notifyListeners();
    }
    return response;
  }

  Future<NativeResponse> removeUserMode(String name) async {
    final local = _removeDraftUserMode(name, commit: !running);
    if (!local.ok) {
      routerStatus = local.message;
      notifyListeners();
      return local;
    }
    if (!running) {
      routerStatus = local.message;
      notifyListeners();
      return local;
    }
    final response = await Future<NativeResponse>(
      () => client.routerRemoveUserMode(name),
    );
    if (response.ok) {
      _removeDraftUserMode(name);
      await _pollRouterNow();
    } else {
      routerStatus = response.message;
      notifyListeners();
    }
    return response;
  }

  Future<NativeResponse> upsertScript(RouterScript script) async {
    final local = _upsertDraftScript(script, commit: !running);
    if (!local.ok) {
      routerStatus = local.message;
      notifyListeners();
      return local;
    }
    if (!running) {
      routerStatus = local.message;
      notifyListeners();
      return local;
    }
    final response = await Future<NativeResponse>(
      () => client.routerSetScript(script.toMap()),
    );
    if (response.ok) {
      _upsertDraftScript(script);
      await _pollRouterNow();
    } else {
      routerStatus = response.message;
      notifyListeners();
    }
    return response;
  }

  Future<NativeResponse> removeScript(String name) async {
    final local = _removeDraftScript(name, commit: !running);
    if (!local.ok) {
      routerStatus = local.message;
      notifyListeners();
      return local;
    }
    if (!running) {
      routerStatus = local.message;
      notifyListeners();
      return local;
    }
    final response = await Future<NativeResponse>(
      () => client.routerRemoveScript(name),
    );
    if (response.ok) {
      _removeDraftScript(name);
      await _pollRouterNow();
    } else {
      routerStatus = response.message;
      notifyListeners();
    }
    return response;
  }

  void closeConnection(int id) {
    final response = client.connectionClose(id);
    if (response.ok) {
      activeConnections = activeConnections
          .where((c) => c.id != id)
          .toList(growable: false);
      notifyListeners();
    }
    unawaited(_pollConnectionsNow());
  }

  void closeAllConnections() {
    if (activeConnections.isEmpty) {
      return;
    }
    final response = client.connectionsCloseMatching(const {});
    if (response.ok) {
      activeConnections = const [];
      notifyListeners();
    }
    unawaited(_pollConnectionsNow());
  }

  Future<void> refreshConnections() => _pollConnectionsNow();
  Future<void> refreshLogs() => _pollLogsNow();
  Future<void> refreshRequests() => _pollRequestsNow();
  Future<void> refreshProxyGroups() => _pollProxyGroupsNow();

  List<LogEntry> get visibleLogs => recentLogs
      .where((entry) => logLevelFilter.allows(entry.level))
      .toList(growable: false);

  void setLogLevelFilter(LogLevelFilter value) {
    if (logLevelFilter == value) {
      return;
    }
    logLevelFilter = value;
    notifyListeners();
  }

  void setLocalSocksEnabled(bool value) {
    if (!value && !localHttpEnabled) {
      profilesStatus = 'At least one local proxy protocol must remain enabled.';
      notifyListeners();
      return;
    }
    localSocksEnabled = value;
    notifyListeners();
  }

  void setLocalHttpEnabled(bool value) {
    if (!value && !localSocksEnabled) {
      profilesStatus = 'At least one local proxy protocol must remain enabled.';
      notifyListeners();
      return;
    }
    localHttpEnabled = value;
    notifyListeners();
  }

  Future<void> selectProxyGroupMember(String group, String member) async {
    final local = _selectDraftProxyGroupMember(group, member, commit: !running);
    if (!local.ok) {
      proxyGroupsStatus = local.message;
      notifyListeners();
      return;
    }
    if (!running) {
      proxyGroupsStatus = local.message;
      notifyListeners();
      return;
    }
    final response = await Future<NativeResponse>(
      () => client.proxyGroupSelect(group, member),
    );
    if (!response.ok) {
      proxyGroupsStatus = response.message;
      notifyListeners();
      return;
    }
    _selectDraftProxyGroupMember(group, member);
    proxyGroupsStatus = 'Selected $member in $group';
    await _pollProxyGroupsNow();
  }

  Future<NativeResponse> setDnsSettings(DnsSettingsInput settings) async {
    final local = _setDraftDnsSettings(settings, commit: !running);
    if (!local.ok) {
      dnsStatus = local.message;
      notifyListeners();
      return local;
    }
    if (!running) {
      dnsStatus = local.message;
      notifyListeners();
      return local;
    }
    final response = await Future<NativeResponse>(
      () => client.dnsSettingsSet(settings.toMap()),
    );
    if (response.ok) {
      final applied = response.data.isEmpty
          ? settings
          : DnsSettingsInput.fromMap(response.data);
      _setDraftDnsSettings(applied);
      dnsStatus = response.message;
      notifyListeners();
    } else {
      dnsStatus = response.message;
      notifyListeners();
    }
    return response;
  }

  double get upRatePerSecond => _latestRate(uploadedBytesHistory);
  double get downRatePerSecond => _latestRate(downloadedBytesHistory);

  int get bytesUploaded => (stats['bytes_uploaded'] as num?)?.toInt() ?? 0;
  int get bytesDownloaded => (stats['bytes_downloaded'] as num?)?.toInt() ?? 0;

  double _latestRate(List<DashboardSeriesPoint> points) {
    if (points.length < 2) {
      return 0;
    }
    final a = points[points.length - 2];
    final b = points[points.length - 1];
    final dt = b.timestamp.difference(a.timestamp).inMilliseconds / 1000;
    if (dt <= 0) {
      return 0;
    }
    final delta = b.value - a.value;
    return delta < 0 ? 0 : delta / dt;
  }

  void _ensureStatusPolling() {
    _statusPollTimer ??= Timer.periodic(const Duration(seconds: 1), (_) {
      if (!running || busy) {
        return;
      }
      unawaited(refreshStatus());
    });
    if (_connectionsPollTimer == null) {
      _connectionsPollTimer = Timer.periodic(const Duration(seconds: 1), (_) {
        if (!running) {
          return;
        }
        unawaited(_pollConnectionsNow());
      });
      unawaited(_pollConnectionsNow());
    }
    if (_logsPollTimer == null) {
      _logsPollTimer = Timer.periodic(const Duration(milliseconds: 750), (_) {
        unawaited(_pollLogsNow());
      });
      unawaited(_pollLogsNow());
    }
    if (_requestsPollTimer == null) {
      _requestsPollTimer = Timer.periodic(const Duration(seconds: 1), (_) {
        unawaited(_pollRequestsNow());
      });
      unawaited(_pollRequestsNow());
    }
    if (_proxyGroupsPollTimer == null) {
      _proxyGroupsPollTimer = Timer.periodic(const Duration(seconds: 2), (_) {
        if (!running) {
          return;
        }
        unawaited(_pollProxyGroupsNow());
      });
      unawaited(_pollProxyGroupsNow());
    }
    if (_routerPollTimer == null) {
      _routerPollTimer = Timer.periodic(const Duration(seconds: 2), (_) {
        if (!running) {
          return;
        }
        unawaited(_pollRouterNow());
      });
      unawaited(_pollRouterNow());
    }
  }

  void _stopStatusPolling() {
    _statusPollTimer?.cancel();
    _statusPollTimer = null;
    _connectionsPollTimer?.cancel();
    _connectionsPollTimer = null;
    _logsPollTimer?.cancel();
    _logsPollTimer = null;
    _requestsPollTimer?.cancel();
    _requestsPollTimer = null;
    _proxyGroupsPollTimer?.cancel();
    _proxyGroupsPollTimer = null;
    _routerPollTimer?.cancel();
    _routerPollTimer = null;
    proxyGroups = ProxyGroupsSnapshot.empty;
    routerSnapshot = RouterSnapshot.empty;
  }

  Future<void> _pollConnectionsNow() async {
    if (_pollingConnections) {
      return;
    }
    _pollingConnections = true;
    try {
      final response = await Future<NativeResponse>(
        () => client.connectionsList(),
      );
      if (!response.ok) {
        return;
      }
      final parsed = _parseConnections(response.data);
      final changed = !_connectionsEqual(activeConnections, parsed);
      if (changed) {
        activeConnections = parsed;
        notifyListeners();
      }
    } finally {
      _pollingConnections = false;
    }
  }

  Future<void> _pollLogsNow() async {
    if (_pollingLogs) {
      return;
    }
    _pollingLogs = true;
    try {
      final response = await Future<NativeResponse>(
        () => client.logsSince(_logCursor),
      );
      if (!response.ok) {
        return;
      }
      final nextCursor =
          (response.data['cursor'] as num?)?.toInt() ?? _logCursor;
      final entries = response.data['entries'];
      if (entries is! List || entries.isEmpty) {
        _logCursor = nextCursor;
        return;
      }
      final appended = <LogEntry>[];
      for (final raw in entries) {
        if (raw is! Map) continue;
        final map = Map<String, Object?>.from(raw);
        final tsMs = (map['ts_unix_ms'] as num?)?.toInt() ?? 0;
        appended.add(
          LogEntry(
            timestamp: DateTime.fromMillisecondsSinceEpoch(
              tsMs,
              isUtc: true,
            ).toLocal(),
            level: (map['level'] as String? ?? '').toString(),
            target: (map['target'] as String? ?? '').toString(),
            message: (map['message'] as String? ?? '').toString(),
          ),
        );
      }
      _logCursor = nextCursor;
      if (appended.isEmpty) {
        return;
      }
      final combined = [...recentLogs, ...appended];
      final overflow = combined.length - _maxLogEntries;
      recentLogs = overflow > 0
          ? combined.sublist(overflow)
          : List<LogEntry>.unmodifiable(combined);
      notifyListeners();
    } finally {
      _pollingLogs = false;
    }
  }

  Future<void> _pollRequestsNow() async {
    if (_pollingRequests) {
      return;
    }
    _pollingRequests = true;
    try {
      final response = await Future<NativeResponse>(
        () => client.requestsSince(_requestCursor),
      );
      if (!response.ok) {
        return;
      }
      final nextCursor =
          (response.data['cursor'] as num?)?.toInt() ?? _requestCursor;
      final entries = response.data['entries'];
      if (entries is! List || entries.isEmpty) {
        _requestCursor = nextCursor;
        return;
      }
      final appended = <RequestInfo>[];
      for (final raw in entries) {
        if (raw is! Map) continue;
        final map = Map<String, Object?>.from(raw);
        final id = (map['conn_id'] as num?)?.toInt() ?? 0;
        final tsMs = (map['ts_unix_ms'] as num?)?.toInt() ?? 0;
        appended.add(
          RequestInfo(
            id: id,
            target: (map['target'] as String?) ?? '',
            sourceApp: (map['source_app'] as String?) ?? '',
            timestamp: DateTime.fromMillisecondsSinceEpoch(
              tsMs,
              isUtc: true,
            ).toLocal(),
            method: (map['method'] as String?) ?? '',
            url: map['url'] as String?,
            host: map['host'] as String?,
            sourcePid: (map['source_pid'] as num?)?.toInt(),
          ),
        );
      }
      _requestCursor = nextCursor;
      if (appended.isEmpty) {
        return;
      }
      final combined = [...recentRequests, ...appended];
      final overflow = combined.length - _maxRequestEntries;
      recentRequests = overflow > 0
          ? combined.sublist(overflow)
          : List<RequestInfo>.unmodifiable(combined);
      notifyListeners();
    } finally {
      _pollingRequests = false;
    }
  }

  Future<void> _pollProxyGroupsNow() async {
    if (_pollingProxyGroups) {
      return;
    }
    _pollingProxyGroups = true;
    try {
      final response = await Future<NativeResponse>(
        () => client.proxyGroupsJson(),
      );
      if (!response.ok) {
        proxyGroupsStatus = response.message;
        notifyListeners();
        return;
      }
      proxyGroups = ProxyGroupsSnapshot.fromMap(response.data);
      _syncDraftProxyGroups(proxyGroups);
      notifyListeners();
    } finally {
      _pollingProxyGroups = false;
    }
  }

  Future<void> _pollRouterNow() async {
    if (_pollingRouter) {
      return;
    }
    _pollingRouter = true;
    try {
      final response = await Future<NativeResponse>(
        () => client.routerSnapshotJson(),
      );
      if (!response.ok) {
        routerStatus = response.message;
        notifyListeners();
        return;
      }
      routerSnapshot = RouterSnapshot.fromMap(response.data);
      _syncDraftRouter(routerSnapshot);
      routerStatus = '';
      notifyListeners();
    } finally {
      _pollingRouter = false;
    }
  }

  Future<void> refreshRouter() => _pollRouterNow();

  ControlAvailability _parseTunStatus(Map<String, Object?> data) {
    tunPreparationAvailable = data['preparable'] == true;
    return ControlAvailability(
      supported: data['supported'] == true,
      enabled: data['enabled'] == true,
      disabledReason: data['disabled_reason'] as String? ?? '',
      platform: data['platform'] as String? ?? 'unknown',
    );
  }

  String get tunGuidance => switch (tunAvailability.platform) {
    'linux' =>
      'Linux requires /dev/net/tun plus CAP_NET_ADMIN. wrongcl will not elevate privileges automatically.',
    'windows' =>
      'Windows TUN now depends on a bundled wintun.dll, and `WRONGCL_WINTUN_DLL` can override lookup during external validation. The runtime currently targets the IPv4 routed path first and still needs Administrator rights.',
    'macos' =>
      'macOS TUN is still a planned host path. The current repo only prewires the seam and validation entrypoints; the native utun-backed implementation still needs real macOS-host work.',
    _ => 'TUN setup is not implemented for this platform yet.',
  };

  List<ConnectionInfo> _parseConnections(Map<String, Object?> data) {
    final list = data['connections'];
    if (list is! List) {
      return const [];
    }
    final out = <ConnectionInfo>[];
    for (final raw in list) {
      if (raw is! Map) continue;
      final map = Map<String, Object?>.from(raw);
      final id = (map['id'] as num?)?.toInt();
      if (id == null) continue;
      final startedMs = (map['started_at_unix_ms'] as num?)?.toInt() ?? 0;
      out.add(
        ConnectionInfo(
          id: id,
          target: (map['target'] as String?) ?? '(handshaking)',
          sourceApp: (map['source_app'] as String?) ?? '',
          bytesUp: (map['bytes_up'] as num?)?.toInt() ?? 0,
          bytesDown: (map['bytes_down'] as num?)?.toInt() ?? 0,
          startedAt: DateTime.fromMillisecondsSinceEpoch(
            startedMs,
            isUtc: true,
          ).toLocal(),
        ),
      );
    }
    return List<ConnectionInfo>.unmodifiable(out);
  }

  bool _connectionsEqual(List<ConnectionInfo> a, List<ConnectionInfo> b) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      final x = a[i];
      final y = b[i];
      if (x.id != y.id ||
          x.target != y.target ||
          x.sourceApp != y.sourceApp ||
          x.bytesUp != y.bytesUp ||
          x.bytesDown != y.bytesDown) {
        return false;
      }
    }
    return true;
  }

  String formatVersion(Map<String, Object?> data) => _formatVersion(data);

  String _formatVersion(Map<String, Object?> data) {
    final version = data['version'] ?? 'unknown';
    final proxies = (data['proxies'] as List?)?.join(', ') ?? '';
    final transports = (data['transports'] as List?)?.join(', ') ?? '';
    final outer = (data['outer_security'] as List?)?.join(', ') ?? '';
    return 'Native $version | proxies: $proxies | transports: $transports | outer: $outer';
  }

  void _attachFieldListeners() {
    for (final controller in [
      serverHost,
      serverPort,
      uuid,
      naiveUsername,
      naivePassword,
      naivePaddingHeaderName,
      hysteria2ServerName,
      hysteria2Password,
      hysteria2ObfsPassword,
      hysteria2ObfsMinPacketSize,
      hysteria2ObfsMaxPacketSize,
      tuicServerName,
      tuicUuid,
      tuicPassword,
      trojanPassword,
      mixedUsername,
      mixedPassword,
      shadowsocksPassword,
      wireguardPrivateKey,
      wireguardPeerPublicKey,
      wireguardPreSharedKey,
      wireguardClientIp,
      wireguardAllowedIps,
      wireguardMtu,
      kcpSeed,
      kcpMtu,
      kcpTti,
      meekPath,
      meekHost,
      gdocsviewerPathPrefix,
      gdocsviewerSharedKey,
      quicServerName,
      webtransportAuthority,
      webtransportPath,
      wsPath,
      wsHost,
      huPath,
      huHost,
      xhttpPath,
      xhttpHost,
      grpcServiceName,
      tlsServerName,
      tlsAlpn,
      realityServerName,
      realityPublicKey,
      realityShortId,
      realityRawPubkey,
      anytlsServerName,
      anytlsPassword,
      shadowTlsServerName,
      shadowTlsPassword,
      localHost,
      localPort,
    ]) {
      controller.addListener(refreshStack);
    }
  }

  ClientConfigInput buildConfig() {
    final draft = _draftConfig;
    final primaryName = draft.endpoints.isEmpty
        ? 'default'
        : draft.endpoints.first.name;
    final endpoint = NamedEndpointInput(
      name: primaryName,
      host: serverHost.text,
      port: int.tryParse(serverPort.text) ?? 0,
      endpoint: EndpointConfig(
        proxy: _proxyJson(),
        transport: _transportJson(),
        outerSecurity: _outerSecurityJson(),
      ),
    );
    final endpoints = <NamedEndpointInput>[
      endpoint,
      ...draft.endpoints.skip(1),
    ];
    return draft.copyWith(
      endpoints: endpoints,
      localHost: localHost.text,
      localPort: int.tryParse(localPort.text) ?? 0,
      allowSocks: localSocksEnabled,
      allowHttp: localHttpEnabled,
    );
  }

  ProbeRequest buildProbe() {
    return ProbeRequest(
      config: buildConfig(),
      targetHost: targetHost.text,
      targetPort: int.tryParse(targetPort.text) ?? 0,
      payload: payload.text,
    );
  }

  WrongsvAdaptRequest buildWrongsvAdaptRequest() {
    return WrongsvAdaptRequest(
      path: wrongsvConfigPath.text,
      serverHost: wrongsvServerHost.text,
      listenHost: wrongsvListenHost.text,
      listenPort: int.tryParse(wrongsvListenPort.text) ?? 1080,
    );
  }

  NativeResponse _okResponse(
    String message, [
    Map<String, Object?> data = const {},
  ]) {
    return NativeResponse(ok: true, message: message, data: data);
  }

  NativeResponse _errorResponse(String message) {
    return NativeResponse(ok: false, message: message, data: const {});
  }

  RouterMode? _findMode(List<RouterMode> modes, String name) {
    for (final mode in modes) {
      if (mode.name == name) {
        return mode;
      }
    }
    return null;
  }

  bool _hasProxyTarget(ClientConfigInput config, String name) {
    for (final endpoint in config.endpoints) {
      if (endpoint.name == name) {
        return true;
      }
    }
    for (final group in config.groups) {
      if (group.name == name) {
        return true;
      }
    }
    return false;
  }

  RouterMode _normalizeUserMode(RouterMode mode) {
    final proxy = mode.proxy?.trim();
    final script = mode.script?.trim();
    return RouterMode(
      name: mode.name.trim(),
      kind: mode.kind,
      proxy: proxy == null || proxy.isEmpty ? null : proxy,
      script: script == null || script.isEmpty ? null : script,
    );
  }

  NativeResponse _setDraftActiveMode(String modeId, {bool commit = true}) {
    final config = buildConfig();
    if (_findMode(config.modes, modeId) == null) {
      return _errorResponse("mode '$modeId' is not defined");
    }
    if (commit) {
      _draftConfig = config.copyWith(activeMode: modeId);
    }
    return _okResponse('active mode set', {'active_mode': modeId});
  }

  NativeResponse _setDraftDnsSettings(
    DnsSettingsInput settings, {
    bool commit = true,
  }) {
    final normalized = settings.normalized();
    final error = normalized.validateMessage();
    if (error != null) {
      return _errorResponse(error);
    }
    if (commit) {
      final config = buildConfig();
      _draftConfig = config.copyWith(dns: normalized.toMap());
    }
    return _okResponse('DNS settings saved', normalized.toMap());
  }

  NativeResponse _upsertDraftUserMode(RouterMode mode, {bool commit = true}) {
    final normalized = _normalizeUserMode(mode);
    if (normalized.kind != 'user') {
      return _errorResponse('upsert_user_mode only accepts user-kind modes');
    }
    if (normalized.name.isEmpty) {
      return _errorResponse('mode name must not be empty');
    }
    if (normalized.name == 'global' ||
        normalized.name == 'rule' ||
        normalized.name == 'direct') {
      return _errorResponse(
        "'${normalized.name}' is a built-in mode name and cannot be redefined",
      );
    }
    final config = buildConfig();
    final existing = _findMode(config.modes, normalized.name);
    if (existing != null && existing.kind != 'user') {
      return _errorResponse(
        "cannot overwrite built-in mode '${normalized.name}'",
      );
    }
    if (existing == null && config.modes.length >= kMaxModeSlots) {
      return _errorResponse('cannot add more than $kMaxModeSlots modes');
    }
    final proxy = normalized.proxy;
    if (proxy == null) {
      return _errorResponse(
        "user mode '${normalized.name}' must specify a proxy",
      );
    }
    if (!_hasProxyTarget(config, proxy)) {
      return _errorResponse(
        "mode '${normalized.name}' references unknown proxy '$proxy'",
      );
    }
    final script = normalized.script;
    if (script != null &&
        !config.scripts.any((value) => value.name == script)) {
      return _errorResponse(
        "mode '${normalized.name}' references unknown script '$script'",
      );
    }
    if (commit) {
      final modes = [...config.modes];
      final index = modes.indexWhere((value) => value.name == normalized.name);
      if (index >= 0) {
        modes[index] = normalized;
      } else {
        modes.add(normalized);
      }
      _draftConfig = config.copyWith(modes: modes);
    }
    return _okResponse('mode saved', {'name': normalized.name});
  }

  NativeResponse _removeDraftUserMode(String name, {bool commit = true}) {
    final normalized = name.trim();
    final config = buildConfig();
    final mode = _findMode(config.modes, normalized);
    if (mode == null) {
      return _errorResponse("mode '$normalized' is not defined");
    }
    if (mode.kind != 'user') {
      return _errorResponse("cannot remove built-in mode '$normalized'");
    }
    if (commit) {
      _draftConfig = config.copyWith(
        modes: [
          for (final entry in config.modes)
            if (entry.name != normalized) entry,
        ],
        activeMode: config.activeMode == normalized
            ? 'global'
            : config.activeMode,
      );
    }
    return _okResponse('mode removed', {'name': normalized});
  }

  NativeResponse _upsertDraftScript(RouterScript script, {bool commit = true}) {
    final normalized = RouterScript(
      name: script.name.trim(),
      rules: script.rules,
    );
    if (normalized.name.isEmpty) {
      return _errorResponse('script name must not be empty');
    }
    final config = buildConfig();
    for (final rule in normalized.rules) {
      if (rule.action != 'proxy') {
        continue;
      }
      final proxy = rule.proxyName?.trim();
      if (proxy == null || proxy.isEmpty || !_hasProxyTarget(config, proxy)) {
        return _errorResponse(
          "script '${normalized.name}' references unknown proxy '${rule.proxyName ?? ''}'",
        );
      }
    }
    if (commit) {
      final scripts = [...config.scripts];
      final index = scripts.indexWhere(
        (value) => value.name == normalized.name,
      );
      if (index >= 0) {
        scripts[index] = normalized;
      } else {
        scripts.add(normalized);
      }
      _draftConfig = config.copyWith(scripts: scripts);
    }
    return _okResponse('script saved', {'name': normalized.name});
  }

  NativeResponse _removeDraftScript(String name, {bool commit = true}) {
    final normalized = name.trim();
    final config = buildConfig();
    for (final mode in config.modes) {
      if (mode.script == normalized) {
        return _errorResponse(
          "script '$normalized' is still used by mode '${mode.name}'",
        );
      }
    }
    if (commit) {
      _draftConfig = config.copyWith(
        scripts: [
          for (final script in config.scripts)
            if (script.name != normalized) script,
        ],
      );
    }
    return _okResponse('script removed', {'name': normalized});
  }

  NativeResponse _selectDraftProxyGroupMember(
    String group,
    String member, {
    bool commit = true,
  }) {
    final groupName = group.trim();
    final memberName = member.trim();
    final config = buildConfig();
    ProxyGroupInput? target;
    for (final entry in config.groups) {
      if (entry.name == groupName) {
        target = entry;
        break;
      }
    }
    if (target == null) {
      return _errorResponse("group '$groupName' is not defined");
    }
    if (target.kind != ProxyGroupKind.select) {
      return _errorResponse(
        "group '$groupName' does not allow manual selection",
      );
    }
    if (!target.members.contains(memberName)) {
      return _errorResponse(
        "group '$groupName' does not contain member '$memberName'",
      );
    }
    if (commit) {
      _draftConfig = config.copyWith(
        groups: [
          for (final entry in config.groups)
            if (entry.name == groupName)
              ProxyGroupInput(
                name: entry.name,
                kind: entry.kind,
                members: entry.members,
                selected: memberName,
              )
            else
              entry,
        ],
      );
    }
    return _okResponse('group member selected', {
      'group': groupName,
      'member': memberName,
    });
  }

  void _syncDraftRouter(RouterSnapshot snapshot) {
    final config = buildConfig();
    _draftConfig = config.copyWith(
      scripts: snapshot.scripts,
      modes: snapshot.modes.isEmpty ? kBuiltinRouterModes : snapshot.modes,
      activeMode: snapshot.activeMode ?? config.activeMode,
    );
  }

  void _syncDraftProxyGroups(ProxyGroupsSnapshot snapshot) {
    final config = buildConfig();
    final active = snapshot.active;
    _draftConfig = config.copyWith(
      groups: [
        for (final group in snapshot.groups)
          ProxyGroupInput(
            name: group.name,
            kind: group.kind,
            members: group.members,
            selected: group.selected,
          ),
      ],
      active: active == null
          ? config.active
          : active.kind == 'group'
          ? ActiveSelectionInput.group(active.name)
          : ActiveSelectionInput.endpoint(active.name),
    );
  }

  Map<String, Object?> _proxyJson() {
    switch (proxyKind) {
      case ProxyKind.vless:
        return const VlessConfig(uuid: '').toJson()
          ..['uuid'] = uuid.text
          ..['flow'] = vlessVisionFlow ? 'xtls-rprx-vision' : '';
      case ProxyKind.naive:
        return NaiveConfig(
          username: naiveUsername.text,
          password: naivePassword.text,
          paddingHeaderName: naivePaddingHeaderName.text.isEmpty
              ? 'Padding'
              : naivePaddingHeaderName.text,
        ).toJson();
      case ProxyKind.hysteria2:
        return Hysteria2Config(
          serverName: hysteria2ServerName.text.isEmpty
              ? 'foo.cloudfront.net'
              : hysteria2ServerName.text,
          password: hysteria2Password.text,
          udpEnabled: hysteria2UdpEnabled,
          obfsType: hysteria2ObfsType.isEmpty ? null : hysteria2ObfsType,
          obfsPassword: hysteria2ObfsPassword.text.isEmpty
              ? null
              : hysteria2ObfsPassword.text,
          obfsMinPacketSize: hysteria2ObfsMinPacketSize.text.isEmpty
              ? null
              : int.tryParse(hysteria2ObfsMinPacketSize.text),
          obfsMaxPacketSize: hysteria2ObfsMaxPacketSize.text.isEmpty
              ? null
              : int.tryParse(hysteria2ObfsMaxPacketSize.text),
        ).toJson();
      case ProxyKind.tuic:
        return TuicConfig(
          serverName: tuicServerName.text.isEmpty
              ? 'foo.cloudfront.net'
              : tuicServerName.text,
          uuid: tuicUuid.text,
          password: tuicPassword.text,
        ).toJson();
      case ProxyKind.trojan:
        return TrojanConfig(password: trojanPassword.text).toJson();
      case ProxyKind.mixed:
        return MixedConfig(
          username: mixedUsername.text.isEmpty ? null : mixedUsername.text,
          password: mixedPassword.text.isEmpty ? null : mixedPassword.text,
        ).toJson();
      case ProxyKind.shadowsocks:
        return ShadowsocksConfig(
          method: shadowsocksMethod,
          password: shadowsocksPassword.text,
        ).toJson();
      case ProxyKind.wireguard:
        final allowedIps = wireguardAllowedIps.text
            .split(',')
            .map((value) => value.trim())
            .where((value) => value.isNotEmpty)
            .toList();
        return WireGuardConfig(
          privateKey: wireguardPrivateKey.text,
          peerPublicKey: wireguardPeerPublicKey.text,
          preSharedKey: wireguardPreSharedKey.text.isEmpty
              ? null
              : wireguardPreSharedKey.text,
          clientIp: wireguardClientIp.text,
          allowedIps: allowedIps,
          mtu: int.tryParse(wireguardMtu.text) ?? 1400,
        ).toJson();
    }
  }

  Map<String, Object?> _transportJson() {
    switch (transportKind) {
      case TransportKind.raw:
        return const {'type': 'raw'};
      case TransportKind.kcp:
        return KcpConfig(
          seed: kcpSeed.text,
          mtu: int.tryParse(kcpMtu.text) ?? 1350,
          tti: int.tryParse(kcpTti.text) ?? 50,
        ).toJson();
      case TransportKind.meek:
        return MeekConfig(
          path: meekPath.text.isEmpty ? '/' : meekPath.text,
          host: meekHost.text.isEmpty ? null : meekHost.text,
        ).toJson();
      case TransportKind.gdocsviewer:
        return GdocsViewerConfig(
          pathPrefix: gdocsviewerPathPrefix.text.isEmpty
              ? '/gdocsviewer'
              : gdocsviewerPathPrefix.text,
          sharedKey: gdocsviewerSharedKey.text.isEmpty
              ? null
              : gdocsviewerSharedKey.text,
        ).toJson();
      case TransportKind.quic:
        return QuicConfig(
          serverName: quicServerName.text.isEmpty
              ? 'cloudfront.net'
              : quicServerName.text,
          udpEnabled: quicUdpEnabled,
        ).toJson();
      case TransportKind.webtransport:
        return WebTransportConfig(
          authority: webtransportAuthority.text.isEmpty
              ? serverHost.text
              : webtransportAuthority.text,
          path: webtransportPath.text.isEmpty ? '/wt' : webtransportPath.text,
          udpEnabled: webtransportUdpEnabled,
        ).toJson();
      case TransportKind.websocket:
        return WsConfig(
          path: wsPath.text.isEmpty ? '/ws' : wsPath.text,
          host: wsHost.text.isEmpty ? null : wsHost.text,
        ).toJson();
      case TransportKind.httpupgrade:
        return HuConfig(
          path: huPath.text.isEmpty ? '/up' : huPath.text,
          host: huHost.text.isEmpty ? null : huHost.text,
        ).toJson();
      case TransportKind.xhttp:
        return XhttpConfig(
          path: xhttpPath.text.isEmpty ? '/xhttp' : xhttpPath.text,
          host: xhttpHost.text.isEmpty ? null : xhttpHost.text,
        ).toJson();
      case TransportKind.grpc:
        return GrpcConfig(
          serviceName: grpcServiceName.text.isEmpty
              ? 'GunService'
              : grpcServiceName.text,
        ).toJson();
    }
  }

  Map<String, Object?> _outerSecurityJson() {
    switch (outerSecurityKind) {
      case OuterSecurityKind.none:
        return const {'type': 'none'};
      case OuterSecurityKind.tls:
        final alpn = tlsAlpn.text
            .split(',')
            .map((value) => value.trim())
            .where((value) => value.isNotEmpty)
            .toList();
        return TlsConfig(
          serverName: tlsServerName.text.isEmpty
              ? serverHost.text
              : tlsServerName.text,
          insecureSkipVerify: tlsInsecure,
          alpn: alpn,
        ).toJson();
      case OuterSecurityKind.reality:
        return RealityConfig(
          serverName: realityServerName.text.isEmpty
              ? serverHost.text
              : realityServerName.text,
          publicKey: realityPublicKey.text,
          shortId: realityShortId.text,
          rawPubkey: realityRawPubkey.text,
        ).toJson();
      case OuterSecurityKind.anytls:
        return AnyTlsConfig(
          serverName: anytlsServerName.text.isEmpty
              ? serverHost.text
              : anytlsServerName.text,
          password: anytlsPassword.text,
          insecureSkipVerify: anytlsInsecureSkipVerify,
        ).toJson();
      case OuterSecurityKind.shadowtls:
        return ShadowTlsConfig(
          serverName: shadowTlsServerName.text.isEmpty
              ? 'cloudfront.net'
              : shadowTlsServerName.text,
          password: shadowTlsPassword.text,
        ).toJson();
    }
  }

  Future<void> loadProfiles() async {
    try {
      final profiles = await profileStore.loadProfiles();
      savedProfiles = profiles;
      if (selectedProfileId != null &&
          !savedProfiles.any((profile) => profile.id == selectedProfileId)) {
        selectedProfileId = null;
      }
      final backup = profileStore.lastCorruptBackupPath;
      if (backup != null) {
        profilesStatus = 'Corrupt profiles.json backed up to $backup';
      } else {
        profilesStatus = profiles.isEmpty ? 'No saved profiles yet' : '';
      }
    } catch (e) {
      profilesStatus = 'Failed to load profiles: $e';
    }
    notifyListeners();
  }

  Future<void> loadAutostartStatus() async {
    try {
      autostartStatus = await autostartManager.loadStatus();
    } catch (e) {
      autostartStatus = AutostartStatus(
        supported: false,
        enabled: false,
        path: '',
        message: 'Failed to inspect autostart: $e',
      );
    }
    notifyListeners();
  }

  Future<void> loadSystemProxyStatus() async {
    try {
      systemProxyStatus = await systemProxyManager.loadStatus();
    } catch (e) {
      systemProxyStatus = SystemProxyStatus(
        supported: false,
        enabled: false,
        mode: 'error',
        message: 'Failed to inspect system proxy: $e',
      );
    }
    notifyListeners();
  }

  Future<void> loadTunStatus() async {
    try {
      final response = client.tunStatusJson();
      if (!response.ok) {
        tunStatus = ControlAvailability(
          supported: false,
          enabled: false,
          disabledReason: response.message,
          platform: 'unknown',
        );
      } else {
        tunStatus = _parseTunStatus(response.data);
      }
    } catch (e) {
      tunStatus = ControlAvailability(
        supported: false,
        enabled: false,
        disabledReason: 'Failed to inspect TUN status: $e',
        platform: 'unknown',
      );
    }
    notifyListeners();
  }

  Future<void> loadClientConfigFile() async {
    final response = client.loadClientConfigFile(clientConfigPath.text);
    if (!response.ok) {
      throw Exception(response.message);
    }
    final config = response.data['config'];
    if (config is! Map) {
      throw const FormatException('native config payload was not a map');
    }
    applyConfigMap(Map<String, Object?>.from(config));
    stackSummary = response.data['stack'] as String? ?? stackSummary;
    wrongsvReport = null;
    wrongsvAdaptResult = null;
    selectedProfileId = null;
    profilesStatus = 'Loaded client config from ${clientConfigPath.text}';
    syncRawConfigEditorFromDraft(notify: false);
    refreshStack();
    notifyListeners();
  }

  Future<void> exportCurrentConfigJson() async {
    final path = clientConfigPath.text.trim();
    if (path.isEmpty) {
      throw const FormatException('config file path is required');
    }
    final file = File(path);
    file.parent.createSync(recursive: true);
    const encoder = JsonEncoder.withIndent('  ');
    final payload = encoder.convert(buildConfig().toJson());
    file.writeAsStringSync('$payload\n');
    profilesStatus = 'Exported current config to $path';
    notifyListeners();
  }

  Future<void> exportCurrentConfigToml() async {
    final path = clientConfigPath.text.trim();
    if (path.isEmpty) {
      throw const FormatException('config file path is required');
    }
    final response = client.exportConfigToml(buildConfig());
    if (!response.ok) {
      throw Exception(response.message);
    }
    final file = File(path);
    file.parent.createSync(recursive: true);
    file.writeAsStringSync(response.data['toml'] as String? ?? '');
    profilesStatus = 'Exported current TOML to $path';
    notifyListeners();
  }

  void syncRawConfigEditorFromDraft({bool notify = true}) {
    const encoder = JsonEncoder.withIndent('  ');
    rawConfigEditor.text = encoder.convert(buildConfig().toJson());
    if (notify) {
      notifyListeners();
    }
  }

  Future<void> applyRawConfigEditorJson() async {
    final raw = rawConfigEditor.text.trim();
    if (raw.isEmpty) {
      throw const FormatException('raw config JSON is required');
    }
    final decoded = jsonDecode(raw);
    if (decoded is! Map) {
      throw const FormatException('raw config must be a JSON object');
    }
    applyConfigMap(Map<String, Object?>.from(decoded));
    selectedProfileId = null;
    wrongsvReport = null;
    wrongsvAdaptResult = null;
    profilesStatus = 'Applied raw JSON config to the current draft';
    refreshStack();
    notifyListeners();
  }

  String formatProfileSubtitle(SavedProfile profile) {
    final parts = <String>[];
    if (profile.stackSummary.isNotEmpty) {
      parts.add(profile.stackSummary);
    }
    if (profile.activeProfile != null && profile.activeProfile!.isNotEmpty) {
      parts.add(profile.activeProfile!);
    }
    if (profile.supportState != null && profile.supportState!.isNotEmpty) {
      parts.add(profile.supportState!);
    }
    parts.add(profile.source);
    parts.add(formatTimestamp(profile.updatedAt));
    return parts.join(' - ');
  }

  String profileSupportBadge(SavedProfile profile) {
    return profile.supportState?.toUpperCase() ?? profile.source.toUpperCase();
  }

  String wrongsvStatusMessage(WrongsvCapabilityReport report) {
    if (report.missingFields.isNotEmpty) {
      return 'missing fields: ${report.missingFields.map((field) => field.field).join(', ')}';
    }
    return '${report.activeProfile} is ${report.activeSupport}';
  }

  void newBlankProfile() {
    _resetToBlankProfile();
    profilesStatus = 'Started new profile';
    refreshStack();
    notifyListeners();
  }

  Future<void> saveCurrentProfile() async {
    final existingIndex = savedProfiles.indexWhere(
      (profile) => profile.id == selectedProfileId,
    );
    final saved = SavedProfile(
      id: existingIndex >= 0
          ? savedProfiles[existingIndex].id
          : DateTime.now().microsecondsSinceEpoch.toString(),
      name: profileName.text.trim().isEmpty
          ? 'Profile ${savedProfiles.length + 1}'
          : profileName.text.trim(),
      config: buildConfig().toJson(),
      stackSummary: stackSummary,
      updatedAt: DateTime.now(),
      source: wrongsvConfigPath.text.trim().isEmpty ? 'manual' : 'wrongsv',
      sourcePath: wrongsvConfigPath.text.trim().isEmpty
          ? null
          : wrongsvConfigPath.text.trim(),
      activeProfile: wrongsvReport?.activeProfile,
      supportState: wrongsvReport?.activeSupport,
      supportReason: wrongsvReport?.activeReason,
      importReport: wrongsvReport?.toMap(),
    );
    final profiles = [...savedProfiles];
    if (existingIndex >= 0) {
      profiles[existingIndex] = saved;
    } else {
      profiles.add(saved);
    }
    await profileStore.saveProfiles(profiles);
    savedProfiles = [...profiles]
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    selectedProfileId = saved.id;
    profileName.text = saved.name;
    wrongsvConfigPath.text = saved.sourcePath ?? '';
    profilesStatus = 'Saved profile ${saved.name}';
    notifyListeners();
  }

  Future<void> deleteSelectedProfile() async {
    final selected = selectedProfile;
    if (selected == null) {
      return;
    }
    final profiles = savedProfiles
        .where((profile) => profile.id != selected.id)
        .toList();
    await profileStore.saveProfiles(profiles);
    savedProfiles = profiles;
    selectedProfileId = null;
    profilesStatus = 'Deleted profile ${selected.name}';
    notifyListeners();
  }

  Future<void> duplicateSelectedProfile() async {
    final selected = selectedProfile;
    if (selected == null) {
      return;
    }
    final duplicate = selected.copyWith(
      id: DateTime.now().microsecondsSinceEpoch.toString(),
      name: '${selected.name} copy',
      updatedAt: DateTime.now(),
    );
    final profiles = [...savedProfiles, duplicate]
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    await profileStore.saveProfiles(profiles);
    savedProfiles = profiles;
    selectedProfileId = duplicate.id;
    profileName.text = duplicate.name;
    profilesStatus = 'Duplicated profile ${selected.name}';
    notifyListeners();
  }

  void loadSelectedProfile() {
    final selected = selectedProfile;
    if (selected == null) {
      return;
    }
    profileName.text = selected.name;
    wrongsvConfigPath.text = selected.sourcePath ?? '';
    applyConfigMap(selected.config);
    stackSummary = selected.stackSummary;
    refreshStack();
    wrongsvAdaptResult = null;
    wrongsvReport = selected.importReport == null
        ? null
        : WrongsvCapabilityReport.fromMap(selected.importReport!);
    profilesStatus = 'Loaded profile ${selected.name}';
    notifyListeners();
  }

  void selectProfile(SavedProfile profile) {
    selectedProfileId = profile.id;
    profileName.text = profile.name;
    notifyListeners();
  }

  Future<void> enableAutostart() async {
    await autostartManager.enable();
    await loadAutostartStatus();
  }

  Future<void> disableAutostart() async {
    await autostartManager.disable();
    await loadAutostartStatus();
  }

  Future<void> enableSystemProxy() async {
    await systemProxyManager.enableSocks(
      localHost.text,
      int.tryParse(localPort.text) ?? 1080,
    );
    await loadSystemProxyStatus();
  }

  Future<void> disableSystemProxy() async {
    await systemProxyManager.disable();
    await loadSystemProxyStatus();
  }

  Future<void> enableTun() async {
    if (!localSocksEnabled) {
      throw Exception('Enable the local SOCKS5 listener before preparing TUN.');
    }
    final proxyHost = switch (localHost.text.trim()) {
      '0.0.0.0' || '' => '127.0.0.1',
      '::' => '::1',
      final value => value,
    };
    final response = await Future<NativeResponse>(
      () => client.tunEnable({
        'proxy_host': proxyHost,
        'proxy_port': int.tryParse(localPort.text) ?? 1080,
      }),
    );
    if (!response.ok) {
      throw Exception(response.message);
    }
    tunStatus = _parseTunStatus(response.data);
    notifyListeners();
  }

  Future<void> disableTun() async {
    final response = await Future<NativeResponse>(() => client.tunDisable());
    if (!response.ok) {
      throw Exception(response.message);
    }
    tunStatus = _parseTunStatus(response.data);
    notifyListeners();
  }

  Future<void> inspectWrongsv() {
    final mismatch = _detectWrongsvImportFormatMismatch();
    if (mismatch != null) {
      return runUtility(
        'inspect wrongsv',
        () => NativeResponse(ok: false, message: mismatch, data: const {}),
      );
    }
    return runUtility(
      'inspect wrongsv',
      () => client.inspectWrongsvConfig(wrongsvConfigPath.text),
      onSuccess: (response) {
        wrongsvReport = WrongsvCapabilityReport.fromMap(response.data);
        wrongsvAdaptResult = null;
        profilesStatus = wrongsvStatusMessage(wrongsvReport!);
      },
    );
  }

  Future<void> adaptWrongsv() {
    final mismatch = _detectWrongsvImportFormatMismatch();
    if (mismatch != null) {
      return runUtility(
        'adapt wrongsv',
        () => NativeResponse(ok: false, message: mismatch, data: const {}),
      );
    }
    return runUtility(
      'adapt wrongsv',
      () => client.adaptWrongsvConfig(buildWrongsvAdaptRequest()),
      onSuccess: (response) {
        final result = WrongsvAdaptResult.fromMap(response.data);
        wrongsvReport = result.report;
        wrongsvAdaptResult = result;
        _applyAdaptedConfig(response.data);
        profilesStatus = result.effectiveConfig == null
            ? 'Adapted report only: ${wrongsvStatusMessage(result.report)}'
            : result.config == null
            ? 'Adapted draft config: ${wrongsvStatusMessage(result.report)}'
            : 'Adapted wrongsv config into the current form';
        refreshStack();
      },
    );
  }

  Future<void> completeWrongsvImport() {
    if (wrongsvAdaptResult?.effectiveConfig == null) {
      return runUtility(
        'complete wrongsv import',
        () => const NativeResponse(
          ok: false,
          message:
              'Adapt a wrongsv config into the current draft before completing the import.',
          data: {},
        ),
      );
    }
    final unsupported = unresolvedWrongsvMissingFields(
      includeUnsupported: true,
    ).where((field) => controllerForMissingField(field.field) == null).toList();
    if (unsupported.isNotEmpty) {
      return runUtility(
        'complete wrongsv import',
        () => NativeResponse(
          ok: false,
          message:
              'This import still requires manual handling for: ${unsupported.map((field) => field.field).join(', ')}',
          data: const {},
        ),
      );
    }
    final unresolved = unresolvedWrongsvMissingFields();
    if (unresolved.isNotEmpty) {
      return runUtility(
        'complete wrongsv import',
        () => NativeResponse(
          ok: false,
          message:
              'Fill required fields before completing the import: ${unresolved.map((field) => field.field).join(', ')}',
          data: const {},
        ),
      );
    }
    return runUtility(
      'complete wrongsv import',
      () => client.validateConfig(buildConfig()),
      onSuccess: (response) {
        final stack = response.data['stack'] as String?;
        if (stack != null && stack.isNotEmpty) {
          stackSummary = stack;
        }
        profilesStatus =
            'Completed wrongsv import fields and validated the current draft';
      },
    );
  }

  String? _detectWrongsvImportFormatMismatch() {
    final path = wrongsvConfigPath.text.trim();
    if (path.isEmpty) {
      return null;
    }
    final file = File(path);
    if (!file.existsSync()) {
      return null;
    }
    String content;
    try {
      content = file.readAsStringSync();
    } catch (_) {
      return null;
    }
    if (!_looksLikeClashMihomoYaml(content)) {
      return null;
    }
    return 'This looks like a Clash/Mihomo YAML config, not a wrongsv TOML config. '
        'The wrongsv import flow only accepts wrongsv server TOML in this milestone.';
  }

  bool _looksLikeClashMihomoYaml(String content) {
    final lower = content.toLowerCase();
    final hasYamlTopLevel = RegExp(
      r'^(mixed-port|allow-lan|proxy-groups|proxies|rules):',
      multiLine: true,
    ).hasMatch(lower);
    final hasProxyList = RegExp(
      r'^\s*-\s*(name|type|server):',
      multiLine: true,
    ).hasMatch(lower);
    return hasYamlTopLevel && (lower.contains('proxies:') || hasProxyList);
  }

  void refreshStack() {
    try {
      final response = client.stackSummary(buildConfig());
      if (response.ok) {
        stackSummary = response.data['stack'] as String? ?? '';
      } else {
        stackSummary = 'invalid stack: ${response.message}';
      }
    } catch (e) {
      stackSummary = 'invalid stack: $e';
    }
    notifyListeners();
  }

  Future<void> startProxy() {
    return run('start', () => client.startProxy(buildConfig()));
  }

  Future<void> stopProxy() {
    return run('stop', () => client.stopProxy());
  }

  Future<void> refreshStatus() {
    return run('status', () => client.status());
  }

  Future<void> validateCurrentConfig() {
    return runUtility(
      'validate config',
      () => client.validateConfig(buildConfig()),
      onSuccess: (response) {
        final stack = response.data['stack'] as String?;
        if (stack != null) {
          stackSummary = stack;
        }
      },
    );
  }

  Future<void> runProbe() {
    return run('probe', () => client.probe(buildProbe()));
  }

  Future<void> prepareForQuit() async {
    if (!running) {
      return;
    }
    try {
      await stopProxy();
    } catch (_) {}
  }

  Future<void> run(String action, NativeResponse Function() call) async {
    busy = true;
    lastResponse = null;
    notifyListeners();
    _scheduleDesktopShellSync();

    try {
      final response = await Future<NativeResponse>(call);
      final proxyData = response.data['proxy'];
      final summary = response.data['stack'] as String?;
      final nextStats = proxyData is Map<String, Object?>
          ? proxyData
          : response.data;
      final runtimeStats = _extractRuntimeStats(nextStats);
      final nextRunning = nextStats['running'];
      final nextLocalHost = nextStats['local_host'];
      final nextLocalPort = nextStats['local_port'];
      final now = DateTime.now();

      lastResponse = response;
      if (response.ok) {
        if (runtimeStats != null) {
          stats = runtimeStats;
          _appendRuntimeSignalSamples(
            runtimeStats,
            timestamp: now,
            action: action,
          );
        }
        if (summary != null && summary.isNotEmpty) {
          stackSummary = summary;
        }
        if (action == 'probe') {
          final probe = response.data['probe'];
          if (probe is Map) {
            final payload = Map<String, Object?>.from(probe);
            lastProbe = HealthProbeSnapshot(
              bytesRead: (payload['bytes_read'] as num?)?.toInt() ?? 0,
              preview: payload['preview'] as String? ?? '',
              timestamp: now,
            );
          }
          _appendProbeOutcome(
            success: true,
            label: 'Probe ok',
            timestamp: now,
            tone: DashboardSignalTone.healthy,
          );
        }
      } else {
        lastError = HealthErrorSnapshot(
          action: action,
          message: response.message,
          timestamp: now,
        );
        if (action == 'probe') {
          _appendProbeOutcome(
            success: false,
            label: 'Probe failed',
            timestamp: now,
            tone: DashboardSignalTone.danger,
          );
        }
      }
      if (nextRunning is bool) {
        running = nextRunning;
        status = nextRunning
            ? 'Running at $nextLocalHost:$nextLocalPort'
            : 'Stopped';
        if (nextRunning) {
          _ensureStatusPolling();
        } else {
          _stopStatusPolling();
        }
      } else if (!response.ok) {
        status = '$action failed';
      }
      _recordActivity(action, response.message, success: response.ok);
    } catch (e) {
      final now = DateTime.now();
      lastResponse = NativeResponse(
        ok: false,
        message: '$action failed: $e',
        data: const {},
      );
      lastError = HealthErrorSnapshot(
        action: action,
        message: '$e',
        timestamp: now,
      );
      status = '$action failed';
      if (action == 'probe') {
        _appendProbeOutcome(
          success: false,
          label: 'Probe failed',
          timestamp: now,
          tone: DashboardSignalTone.danger,
        );
      }
      _recordActivity(action, '$e', success: false);
    } finally {
      busy = false;
      notifyListeners();
      _scheduleDesktopShellSync();
    }
  }

  Future<void> runUtility(
    String action,
    NativeResponse Function() call, {
    void Function(NativeResponse response)? onSuccess,
  }) async {
    busy = true;
    lastResponse = null;
    notifyListeners();
    _scheduleDesktopShellSync();

    try {
      final response = await Future<NativeResponse>(call);
      if (response.ok) {
        onSuccess?.call(response);
      }

      lastResponse = response;
      status = response.ok ? '$action complete' : '$action failed';
      if (!response.ok) {
        lastError = HealthErrorSnapshot(
          action: action,
          message: response.message,
          timestamp: DateTime.now(),
        );
      }
      _recordActivity(action, response.message, success: response.ok);
    } catch (e) {
      lastResponse = NativeResponse(
        ok: false,
        message: '$action failed: $e',
        data: const {},
      );
      lastError = HealthErrorSnapshot(
        action: action,
        message: '$e',
        timestamp: DateTime.now(),
      );
      status = '$action failed';
      _recordActivity(action, '$e', success: false);
    } finally {
      busy = false;
      notifyListeners();
      _scheduleDesktopShellSync();
    }
  }

  Future<void> runTask(String action, Future<void> Function() task) async {
    busy = true;
    lastResponse = null;
    notifyListeners();
    _scheduleDesktopShellSync();

    try {
      await task();
      busy = false;
      status = '$action complete';
      _recordActivity(action, 'completed', success: true);
    } catch (e) {
      busy = false;
      status = '$action failed';
      lastResponse = NativeResponse(
        ok: false,
        message: '$action failed: $e',
        data: const {},
      );
      lastError = HealthErrorSnapshot(
        action: action,
        message: '$e',
        timestamp: DateTime.now(),
      );
      _recordActivity(action, '$e', success: false);
    }
    notifyListeners();
    _scheduleDesktopShellSync();
  }

  void _recordActivity(String title, String detail, {required bool success}) {
    final entry = DashboardActivityEntry(
      title: title,
      detail: detail,
      success: success,
      timestamp: DateTime.now(),
    );
    activityLog = [entry, ...activityLog].take(20).toList();
  }

  Map<String, Object?>? _extractRuntimeStats(Map<String, Object?> raw) {
    const runtimeKeys = {
      'running',
      'active_connections',
      'total_connections',
      'failed_connections',
      'bytes_uploaded',
      'bytes_downloaded',
      'local_host',
      'local_port',
    };
    if (!raw.keys.any(runtimeKeys.contains)) {
      return null;
    }
    return {
      for (final key in runtimeKeys)
        if (raw.containsKey(key)) key: raw[key],
    };
  }

  void _appendRuntimeSignalSamples(
    Map<String, Object?> nextStats, {
    required DateTime timestamp,
    required String action,
  }) {
    activeConnectionsHistory = _appendSeriesPoint(
      activeConnectionsHistory,
      timestamp,
      (nextStats['active_connections'] as num?)?.toDouble() ?? 0,
    );
    totalConnectionsHistory = _appendSeriesPoint(
      totalConnectionsHistory,
      timestamp,
      (nextStats['total_connections'] as num?)?.toDouble() ?? 0,
    );
    failedConnectionsHistory = _appendSeriesPoint(
      failedConnectionsHistory,
      timestamp,
      (nextStats['failed_connections'] as num?)?.toDouble() ?? 0,
    );
    uploadedBytesHistory = _appendSeriesPoint(
      uploadedBytesHistory,
      timestamp,
      (nextStats['bytes_uploaded'] as num?)?.toDouble() ?? 0,
    );
    downloadedBytesHistory = _appendSeriesPoint(
      downloadedBytesHistory,
      timestamp,
      (nextStats['bytes_downloaded'] as num?)?.toDouble() ?? 0,
    );

    final runningValue = nextStats['running'];
    if (runningValue is bool) {
      final label = switch (action) {
        'start' => runningValue ? 'Runtime started' : 'Start returned stopped',
        'stop' => runningValue ? 'Stop returned running' : 'Runtime stopped',
        'status' => runningValue ? 'Status running' : 'Status stopped',
        _ => runningValue ? 'Runtime running' : 'Runtime stopped',
      };
      recentRuntimeStateChanges = _appendSignalEvent(
        recentRuntimeStateChanges,
        DashboardSignalEvent(
          id: 'runtime-$action-${timestamp.microsecondsSinceEpoch}',
          label: label,
          timestamp: timestamp,
          success: true,
          tone: runningValue
              ? DashboardSignalTone.healthy
              : DashboardSignalTone.neutral,
        ),
      );
    }
  }

  void _appendProbeOutcome({
    required bool success,
    required String label,
    required DateTime timestamp,
    required DashboardSignalTone tone,
  }) {
    recentProbeOutcomes = _appendSignalEvent(
      recentProbeOutcomes,
      DashboardSignalEvent(
        id: 'probe-${timestamp.microsecondsSinceEpoch}',
        label: label,
        timestamp: timestamp,
        success: success,
        tone: tone,
      ),
    );
  }

  List<DashboardSeriesPoint> _appendSeriesPoint(
    List<DashboardSeriesPoint> series,
    DateTime timestamp,
    double value,
  ) {
    return [
      ...series,
      DashboardSeriesPoint(timestamp: timestamp, value: value),
    ].takeLast(_maxSignalHistoryPoints);
  }

  List<DashboardSignalEvent> _appendSignalEvent(
    List<DashboardSignalEvent> events,
    DashboardSignalEvent event,
  ) {
    return [...events, event].takeLast(_maxSignalEvents);
  }

  void _resetToBlankProfile() {
    _draftConfig = _defaultDraftConfig();
    profileName.text = 'default';
    clientConfigPath.text = '';
    wrongsvConfigPath.text = '';
    wrongsvServerHost.text = '127.0.0.1';
    wrongsvListenHost.text = '127.0.0.1';
    wrongsvListenPort.text = '1080';
    serverHost.text = '127.0.0.1';
    serverPort.text = '443';
    uuid.text = '12345678-1234-1234-1234-123456789abc';
    naiveUsername.clear();
    naivePassword.clear();
    naivePaddingHeaderName.text = 'Padding';
    hysteria2ServerName.text = 'foo.cloudfront.net';
    hysteria2Password.clear();
    hysteria2UdpEnabled = true;
    hysteria2ObfsType = '';
    hysteria2ObfsPassword.clear();
    hysteria2ObfsMinPacketSize.clear();
    hysteria2ObfsMaxPacketSize.clear();
    tuicServerName.text = 'foo.cloudfront.net';
    tuicUuid.text = '12345678-1234-1234-1234-123456789abc';
    tuicPassword.clear();
    trojanPassword.clear();
    mixedUsername.clear();
    mixedPassword.clear();
    shadowsocksPassword.clear();
    shadowsocksMethod = 'chacha20-ietf-poly1305';
    wireguardPrivateKey.clear();
    wireguardPeerPublicKey.clear();
    wireguardPreSharedKey.clear();
    wireguardClientIp.text = '10.66.66.2/32';
    wireguardAllowedIps.text = '10.66.66.1/32';
    wireguardMtu.text = '1400';
    kcpSeed.clear();
    kcpMtu.text = '1350';
    kcpTti.text = '50';
    meekPath.text = '/';
    meekHost.clear();
    gdocsviewerPathPrefix.text = '/gdocsviewer';
    gdocsviewerSharedKey.clear();
    quicServerName.text = 'cloudfront.net';
    quicUdpEnabled = true;
    webtransportAuthority.text = 'cloudfront.net';
    webtransportPath.text = '/wt';
    webtransportUdpEnabled = true;
    wsPath.text = '/ws';
    wsHost.clear();
    huPath.text = '/up';
    huHost.clear();
    xhttpPath.text = '/xhttp';
    xhttpHost.clear();
    grpcServiceName.text = 'GunService';
    tlsServerName.clear();
    tlsAlpn.text = 'h2, http/1.1';
    tlsInsecure = false;
    vlessVisionFlow = false;
    realityServerName.text = 'www.microsoft.com';
    realityPublicKey.clear();
    realityShortId.clear();
    realityRawPubkey.clear();
    anytlsServerName.clear();
    anytlsPassword.clear();
    anytlsInsecureSkipVerify = true;
    shadowTlsServerName.text = 'cloudfront.net';
    shadowTlsPassword.clear();
    localHost.text = '127.0.0.1';
    localPort.text = '1080';
    localSocksEnabled = true;
    localHttpEnabled = true;
    targetHost.text = 'example.com';
    targetPort.text = '80';
    payload.text =
        'HEAD / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n';
    proxyKind = ProxyKind.vless;
    transportKind = TransportKind.raw;
    outerSecurityKind = OuterSecurityKind.none;
    selectedProfileId = null;
    wrongsvReport = null;
    wrongsvAdaptResult = null;
    proxyGroupsStatus = '';
    routerStatus = '';
    dnsStatus = '';
    stackSummary = '';
  }

  TextEditingController? controllerForMissingField(String field) {
    switch (field) {
      case 'reality.public-key':
        return realityPublicKey;
      case 'reality.short-id':
        return realityShortId;
      case 'reality.raw-pubkey':
        return realityRawPubkey;
      case 'trojan.password':
        return trojanPassword;
      case 'anytls.password':
        return anytlsPassword;
      case 'wireguard.peers.private-key':
      case 'wireguard.private-key':
        return wireguardPrivateKey;
      case 'naive.username':
        return naiveUsername;
      case 'naive.password':
        return naivePassword;
      default:
        return null;
    }
  }

  String labelForMissingField(String field) {
    switch (field) {
      case 'reality.public-key':
        return 'REALITY public-key (required)';
      case 'reality.short-id':
        return 'REALITY short-id (required)';
      case 'reality.raw-pubkey':
        return 'REALITY raw-pubkey (optional verify helper)';
      case 'trojan.password':
        return 'Trojan password (required)';
      case 'anytls.password':
        return 'AnyTLS password (required)';
      case 'wireguard.peers.private-key':
      case 'wireguard.private-key':
        return 'WireGuard peer private-key (required)';
      case 'naive.username':
        return 'Naive username (required)';
      case 'naive.password':
        return 'Naive password (required)';
      default:
        return field;
    }
  }

  List<WrongsvMissingField> unresolvedWrongsvMissingFields({
    bool includeUnsupported = false,
  }) {
    final report = wrongsvReport;
    if (report == null) {
      return const [];
    }
    return report.missingFields
        .where((field) {
          final controller = controllerForMissingField(field.field);
          if (controller == null) {
            return includeUnsupported;
          }
          return controller.text.trim().isEmpty;
        })
        .toList(growable: false);
  }

  String formatTimestamp(DateTime value) {
    final local = value.toLocal();
    final month = local.month.toString().padLeft(2, '0');
    final day = local.day.toString().padLeft(2, '0');
    final hour = local.hour.toString().padLeft(2, '0');
    final minute = local.minute.toString().padLeft(2, '0');
    return '${local.year}-$month-$day $hour:$minute';
  }

  DesktopShellState _desktopShellState() {
    final currentProfileName = profileName.text.trim();
    return DesktopShellState(
      running: running,
      busy: busy,
      status: status,
      profileName: currentProfileName.isEmpty ? 'default' : currentProfileName,
    );
  }

  Future<void> _attachDesktopShell() async {
    await desktopShellController.attach(
      actions: DesktopShellActions(
        startProxy: startProxy,
        stopProxy: stopProxy,
        refreshStatus: refreshStatus,
        prepareForQuit: prepareForQuit,
      ),
      initialState: _desktopShellState(),
    );
    _scheduleDesktopShellSync();
  }

  void _scheduleDesktopShellSync() {
    if (desktopShellSyncScheduled) {
      return;
    }
    desktopShellSyncScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      desktopShellSyncScheduled = false;
      await desktopShellController.sync(_desktopShellState());
    });
  }

  String _buildImportSummary() {
    if (wrongsvReport != null) {
      return wrongsvStatusMessage(wrongsvReport!);
    }
    final selected = selectedProfile;
    if (selected?.supportReason case final reason?) {
      return reason;
    }
    return 'No wrongsv import inspected yet';
  }

  void _applyAdaptedConfig(Map<String, Object?> data) {
    final result = WrongsvAdaptResult.fromMap(data);
    final config = result.effectiveConfig;
    if (config == null) {
      return;
    }
    applyConfigMap(config);
  }

  void applyConfigMap(Map<String, Object?> map) {
    _draftConfig = ClientConfigInput.fromMap(map);
    final endpoints = _draftConfig.endpoints;
    final server = endpoints.isEmpty
        ? const <String, Object?>{}
        : endpoints.first.toJson();
    final local = <String, Object?>{
      'host': _draftConfig.localHost,
      'port': _draftConfig.localPort,
      'allow_socks': _draftConfig.allowSocks,
      'allow_http': _draftConfig.allowHttp,
    };
    final proxy = Map<String, Object?>.from(
      server['proxy'] as Map? ?? const {},
    );
    final transport = Map<String, Object?>.from(
      server['transport'] as Map? ?? const {},
    );
    final outer = Map<String, Object?>.from(
      server['outer-security'] as Map? ?? const {},
    );
    serverHost.text = server['host'] as String? ?? serverHost.text;
    serverPort.text = '${server['port'] ?? serverPort.text}';
    localHost.text = local['host'] as String? ?? localHost.text;
    localPort.text = '${local['port'] ?? localPort.text}';
    localSocksEnabled = local['allow_socks'] != false;
    localHttpEnabled = local['allow_http'] != false;

    final proxyType = proxy['type'] as String? ?? 'vless';
    proxyKind = ProxyKind.fromId(proxyType);
    switch (proxyKind) {
      case ProxyKind.vless:
        uuid.text = proxy['uuid'] as String? ?? uuid.text;
        vlessVisionFlow =
            (proxy['flow'] as String? ?? '') == 'xtls-rprx-vision';
        break;
      case ProxyKind.naive:
        naiveUsername.text = proxy['username'] as String? ?? naiveUsername.text;
        naivePassword.text = proxy['password'] as String? ?? naivePassword.text;
        naivePaddingHeaderName.text =
            proxy['padding-header-name'] as String? ??
            naivePaddingHeaderName.text;
        break;
      case ProxyKind.hysteria2:
        hysteria2ServerName.text =
            proxy['server-name'] as String? ?? hysteria2ServerName.text;
        hysteria2Password.text =
            proxy['password'] as String? ?? hysteria2Password.text;
        hysteria2UdpEnabled = proxy['udp-enabled'] != false;
        hysteria2ObfsType = proxy['obfs-type'] as String? ?? '';
        hysteria2ObfsPassword.text = proxy['obfs-password'] as String? ?? '';
        hysteria2ObfsMinPacketSize.text =
            proxy['obfs-min-packet-size']?.toString() ?? '';
        hysteria2ObfsMaxPacketSize.text =
            proxy['obfs-max-packet-size']?.toString() ?? '';
        break;
      case ProxyKind.tuic:
        tuicServerName.text =
            proxy['server-name'] as String? ?? tuicServerName.text;
        tuicUuid.text = proxy['uuid'] as String? ?? tuicUuid.text;
        tuicPassword.text = proxy['password'] as String? ?? tuicPassword.text;
        break;
      case ProxyKind.trojan:
        trojanPassword.text =
            proxy['password'] as String? ?? trojanPassword.text;
        break;
      case ProxyKind.mixed:
        mixedUsername.text = proxy['username'] as String? ?? mixedUsername.text;
        mixedPassword.text = proxy['password'] as String? ?? mixedPassword.text;
        break;
      case ProxyKind.shadowsocks:
        shadowsocksMethod = proxy['method'] as String? ?? shadowsocksMethod;
        shadowsocksPassword.text =
            proxy['password'] as String? ?? shadowsocksPassword.text;
        break;
      case ProxyKind.wireguard:
        wireguardPrivateKey.text =
            proxy['private-key'] as String? ?? wireguardPrivateKey.text;
        wireguardPeerPublicKey.text =
            proxy['peer-public-key'] as String? ?? wireguardPeerPublicKey.text;
        wireguardPreSharedKey.text = proxy['pre-shared-key'] as String? ?? '';
        wireguardClientIp.text =
            proxy['client-ip'] as String? ?? wireguardClientIp.text;
        final allowedIps =
            (proxy['allowed-ips'] as List?)?.cast<Object?>() ?? const [];
        wireguardAllowedIps.text = allowedIps.join(', ');
        wireguardMtu.text = '${proxy['mtu'] ?? wireguardMtu.text}';
        break;
    }

    final transportType = transport['type'] as String? ?? 'raw';
    transportKind = TransportKind.fromId(transportType);
    switch (transportKind) {
      case TransportKind.raw:
        break;
      case TransportKind.kcp:
        kcpSeed.text = transport['seed'] as String? ?? '';
        kcpMtu.text = '${transport['mtu'] ?? kcpMtu.text}';
        kcpTti.text = '${transport['tti'] ?? kcpTti.text}';
        break;
      case TransportKind.meek:
        meekPath.text = transport['path'] as String? ?? meekPath.text;
        meekHost.text = transport['host'] as String? ?? '';
        break;
      case TransportKind.gdocsviewer:
        gdocsviewerPathPrefix.text =
            transport['path-prefix'] as String? ?? gdocsviewerPathPrefix.text;
        gdocsviewerSharedKey.text = transport['shared-key'] as String? ?? '';
        break;
      case TransportKind.quic:
        quicServerName.text =
            transport['server-name'] as String? ?? quicServerName.text;
        quicUdpEnabled = transport['udp-enabled'] != false;
        break;
      case TransportKind.webtransport:
        webtransportAuthority.text =
            transport['authority'] as String? ?? webtransportAuthority.text;
        webtransportPath.text =
            transport['path'] as String? ?? webtransportPath.text;
        webtransportUdpEnabled = transport['udp-enabled'] != false;
        break;
      case TransportKind.websocket:
        wsPath.text = transport['path'] as String? ?? wsPath.text;
        wsHost.text = transport['host'] as String? ?? '';
        break;
      case TransportKind.httpupgrade:
        huPath.text = transport['path'] as String? ?? huPath.text;
        huHost.text = transport['host'] as String? ?? '';
        break;
      case TransportKind.xhttp:
        xhttpPath.text = transport['path'] as String? ?? xhttpPath.text;
        xhttpHost.text = transport['host'] as String? ?? '';
        break;
      case TransportKind.grpc:
        grpcServiceName.text =
            transport['service-name'] as String? ?? grpcServiceName.text;
        break;
    }

    final outerType = outer['type'] as String? ?? 'none';
    outerSecurityKind = OuterSecurityKind.fromId(outerType);
    switch (outerSecurityKind) {
      case OuterSecurityKind.none:
        break;
      case OuterSecurityKind.tls:
        tlsServerName.text =
            outer['server-name'] as String? ?? tlsServerName.text;
        tlsInsecure = outer['insecure-skip-verify'] == true;
        final alpn = (outer['alpn'] as List?)?.cast<Object?>() ?? const [];
        tlsAlpn.text = alpn.join(', ');
        break;
      case OuterSecurityKind.reality:
        realityServerName.text =
            outer['server-name'] as String? ?? realityServerName.text;
        realityPublicKey.text =
            outer['public-key'] as String? ?? realityPublicKey.text;
        realityShortId.text =
            outer['short-id'] as String? ?? realityShortId.text;
        realityRawPubkey.text =
            outer['raw-pubkey'] as String? ?? realityRawPubkey.text;
        break;
      case OuterSecurityKind.anytls:
        anytlsServerName.text =
            outer['server-name'] as String? ?? anytlsServerName.text;
        anytlsPassword.text =
            outer['password'] as String? ?? anytlsPassword.text;
        anytlsInsecureSkipVerify = outer['insecure-skip-verify'] != false;
        break;
      case OuterSecurityKind.shadowtls:
        shadowTlsServerName.text =
            outer['server-name'] as String? ?? shadowTlsServerName.text;
        shadowTlsPassword.text =
            outer['password'] as String? ?? shadowTlsPassword.text;
        break;
    }
    proxyGroupsStatus = '';
    routerStatus = '';
    dnsStatus = '';
    notifyListeners();
  }

  String shadowsocksMethod = 'chacha20-ietf-poly1305';

  void setProxyKind(ProxyKind value) {
    proxyKind = value;
    if (value == ProxyKind.mixed ||
        value == ProxyKind.shadowsocks ||
        value == ProxyKind.hysteria2 ||
        value == ProxyKind.tuic ||
        value == ProxyKind.wireguard) {
      transportKind = TransportKind.raw;
      outerSecurityKind = OuterSecurityKind.none;
    } else if (value == ProxyKind.trojan || value == ProxyKind.naive) {
      outerSecurityKind = OuterSecurityKind.tls;
      if (transportKind != TransportKind.raw) {
        transportKind = TransportKind.raw;
      }
    } else if (value != ProxyKind.vless &&
        (outerSecurityKind == OuterSecurityKind.reality ||
            outerSecurityKind == OuterSecurityKind.anytls ||
            outerSecurityKind == OuterSecurityKind.shadowtls)) {
      outerSecurityKind = OuterSecurityKind.none;
    }
    refreshStack();
  }

  bool get transportDisabled =>
      busy ||
      proxyKind == ProxyKind.mixed ||
      proxyKind == ProxyKind.shadowsocks ||
      proxyKind == ProxyKind.hysteria2 ||
      proxyKind == ProxyKind.tuic ||
      proxyKind == ProxyKind.wireguard ||
      proxyKind == ProxyKind.naive ||
      outerSecurityKind == OuterSecurityKind.reality ||
      outerSecurityKind == OuterSecurityKind.anytls ||
      outerSecurityKind == OuterSecurityKind.shadowtls;

  bool get outerSecurityDisabled =>
      busy ||
      proxyKind == ProxyKind.mixed ||
      proxyKind == ProxyKind.shadowsocks ||
      proxyKind == ProxyKind.hysteria2 ||
      proxyKind == ProxyKind.tuic ||
      proxyKind == ProxyKind.wireguard ||
      proxyKind == ProxyKind.naive ||
      transportKind == TransportKind.kcp ||
      transportKind == TransportKind.quic ||
      transportKind == TransportKind.webtransport ||
      proxyKind == ProxyKind.trojan;

  void setTransportKind(TransportKind value) {
    transportKind = value;
    if (value == TransportKind.quic ||
        value == TransportKind.kcp ||
        value == TransportKind.webtransport) {
      outerSecurityKind = OuterSecurityKind.none;
    }
    refreshStack();
  }

  void setOuterSecurityKind(OuterSecurityKind value) {
    outerSecurityKind = value;
    if (value == OuterSecurityKind.reality ||
        value == OuterSecurityKind.anytls ||
        value == OuterSecurityKind.shadowtls) {
      transportKind = TransportKind.raw;
    }
    refreshStack();
  }
}

extension<T> on List<T> {
  List<T> takeLast(int count) {
    if (length <= count) {
      return this;
    }
    return sublist(length - count);
  }
}
