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

enum HomeRoute { dashboard, profiles, importView, editor, runtime, settings }

class ClientHomeController extends ChangeNotifier {
  static const _maxSignalHistoryPoints = 60;
  static const _maxSignalEvents = 16;

  ClientHomeController({
    required this.client,
    required this.profileStore,
    required this.autostartManager,
    required this.systemProxyManager,
    required this.desktopShellController,
  });

  final WrongclClient client;
  final ProfileStore profileStore;
  final AutostartManager autostartManager;
  final SystemProxyManager systemProxyManager;
  final DesktopShellController desktopShellController;

  final profileName = TextEditingController(text: 'default');
  final clientConfigPath = TextEditingController();
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
  bool quicUdpEnabled = true;
  bool webtransportUdpEnabled = true;
  bool tlsInsecure = false;
  bool vlessVisionFlow = false;
  bool anytlsInsecureSkipVerify = true;

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

  Future<void> init() async {
    profileName.addListener(_scheduleDesktopShellSync);
    final version = client.version();
    nativeInfo = version.ok ? _formatVersion(version.data) : version.message;
    _attachFieldListeners();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      refreshStack();
      unawaited(refreshStatus());
    });
    unawaited(_attachDesktopShell());
    unawaited(loadProfiles());
    unawaited(loadAutostartStatus());
    unawaited(loadSystemProxyStatus());
  }

  @override
  void dispose() {
    profileName.removeListener(_scheduleDesktopShellSync);
    unawaited(desktopShellController.dispose());
    for (final controller in [
      profileName,
      clientConfigPath,
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
          )
        : ControlAvailability(
            supported: systemProxyStatus!.supported,
            enabled: systemProxyStatus!.enabled,
            disabledReason: systemProxyStatus!.supported
                ? ''
                : systemProxyStatus!.message,
          );
    return DashboardSnapshot(
      running: running,
      busy: busy,
      statusText: status,
      stackSummary: stackSummary,
      nativeInfo: nativeInfo,
      systemProxy: systemProxy,
      tun: const ControlAvailability(
        supported: false,
        enabled: false,
        disabledReason: 'TUN runtime control is not exposed by wrongcl yet',
      ),
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

  bool get showingSecondaryPanel =>
      activeRoute == HomeRoute.profiles ||
      activeRoute == HomeRoute.importView ||
      activeRoute == HomeRoute.settings;

  bool get showingHeavyMode =>
      activeRoute == HomeRoute.editor || activeRoute == HomeRoute.runtime;

  String get activeRouteLabel {
    switch (activeRoute) {
      case HomeRoute.dashboard:
        return 'Dashboard';
      case HomeRoute.profiles:
        return 'Profiles';
      case HomeRoute.importView:
        return 'Import';
      case HomeRoute.editor:
        return 'Editor';
      case HomeRoute.runtime:
        return 'Diagnostics';
      case HomeRoute.settings:
        return 'Settings';
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
    return ClientConfigInput(
      serverHost: serverHost.text,
      serverPort: int.tryParse(serverPort.text) ?? 0,
      localHost: localHost.text,
      localPort: int.tryParse(localPort.text) ?? 0,
      endpoint: EndpointConfig(
        proxy: _proxyJson(),
        transport: _transportJson(),
        outerSecurity: _outerSecurityJson(),
      ),
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
      profilesStatus = profiles.isEmpty ? 'No saved profiles yet' : '';
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

    busy = false;
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
    } else if (!response.ok) {
      status = '$action failed';
    }
    _recordActivity(action, response.message, success: response.ok);
    notifyListeners();
    _scheduleDesktopShellSync();
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

    final response = await Future<NativeResponse>(call);
    if (response.ok) {
      onSuccess?.call(response);
    }

    busy = false;
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
    notifyListeners();
    _scheduleDesktopShellSync();
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
      case 'wireguard.private-key':
        return 'WireGuard private-key (required)';
      case 'naive.username':
        return 'Naive username (required)';
      case 'naive.password':
        return 'Naive password (required)';
      default:
        return field;
    }
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
    final server = Map<String, Object?>.from(map['server'] as Map? ?? const {});
    final local = Map<String, Object?>.from(map['local'] as Map? ?? const {});
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
