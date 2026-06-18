import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';

import 'autostart_manager.dart';
import 'desktop_shell_controller.dart';
import 'health_view.dart';
import 'profile_store.dart';
import 'system_proxy_manager.dart';
import 'wrongcl_client.dart';

class WrongclApp extends StatelessWidget {
  WrongclApp({
    super.key,
    WrongclClient? client,
    ProfileStore? profileStore,
    AutostartManager? autostartManager,
    SystemProxyManager? systemProxyManager,
    DesktopShellController? desktopShellController,
  }) : client = client ?? NativeWrongclClient(),
       profileStore = profileStore ?? ProfileStore(),
       autostartManager = autostartManager ?? AutostartManager(),
       systemProxyManager = systemProxyManager ?? SystemProxyManager(),
       desktopShellController =
           desktopShellController ?? const NoopDesktopShellController();

  final WrongclClient client;
  final ProfileStore profileStore;
  final AutostartManager autostartManager;
  final SystemProxyManager systemProxyManager;
  final DesktopShellController desktopShellController;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Wrongcl',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF006D77)),
        inputDecorationTheme: const InputDecorationTheme(
          border: OutlineInputBorder(),
          isDense: true,
        ),
        cardTheme: const CardThemeData(
          margin: EdgeInsets.zero,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.all(Radius.circular(8)),
          ),
        ),
      ),
      home: ClientHome(
        client: client,
        profileStore: profileStore,
        autostartManager: autostartManager,
        systemProxyManager: systemProxyManager,
        desktopShellController: desktopShellController,
      ),
    );
  }
}

class ClientHome extends StatefulWidget {
  const ClientHome({
    super.key,
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

  @override
  State<ClientHome> createState() => _ClientHomeState();
}

class _ClientHomeState extends State<ClientHome> {
  final _profileName = TextEditingController(text: 'default');
  final _clientConfigPath = TextEditingController();
  final _wrongsvConfigPath = TextEditingController();
  final _wrongsvServerHost = TextEditingController(text: '127.0.0.1');
  final _wrongsvListenHost = TextEditingController(text: '127.0.0.1');
  final _wrongsvListenPort = TextEditingController(text: '1080');

  final _serverHost = TextEditingController(text: '127.0.0.1');
  final _serverPort = TextEditingController(text: '443');
  final _uuid = TextEditingController(
    text: '12345678-1234-1234-1234-123456789abc',
  );
  final _hysteria2ServerName = TextEditingController(
    text: 'foo.cloudfront.net',
  );
  final _hysteria2Password = TextEditingController();
  bool _hysteria2UdpEnabled = true;
  final _tuicServerName = TextEditingController(text: 'foo.cloudfront.net');
  final _tuicUuid = TextEditingController(
    text: '12345678-1234-1234-1234-123456789abc',
  );
  final _tuicPassword = TextEditingController();
  final _trojanPassword = TextEditingController();
  final _mixedUsername = TextEditingController();
  final _mixedPassword = TextEditingController();
  final _shadowsocksPassword = TextEditingController();
  String _shadowsocksMethod = 'chacha20-ietf-poly1305';
  final _kcpSeed = TextEditingController();
  final _kcpMtu = TextEditingController(text: '1350');
  final _kcpTti = TextEditingController(text: '50');
  final _meekPath = TextEditingController(text: '/');
  final _meekHost = TextEditingController();
  final _gdocsviewerPathPrefix = TextEditingController(text: '/gdocsviewer');
  final _gdocsviewerSharedKey = TextEditingController();
  final _quicServerName = TextEditingController(text: 'cloudfront.net');
  bool _quicUdpEnabled = true;
  final _webtransportAuthority = TextEditingController(text: 'cloudfront.net');
  final _webtransportPath = TextEditingController(text: '/wt');
  bool _webtransportUdpEnabled = true;
  final _wsPath = TextEditingController(text: '/ws');
  final _wsHost = TextEditingController();
  final _huPath = TextEditingController(text: '/up');
  final _huHost = TextEditingController();
  final _xhttpPath = TextEditingController(text: '/xhttp');
  final _xhttpHost = TextEditingController();
  final _grpcServiceName = TextEditingController(text: 'GunService');
  final _tlsServerName = TextEditingController();
  final _tlsAlpn = TextEditingController(text: 'h2, http/1.1');
  bool _tlsInsecure = false;
  bool _vlessVisionFlow = false;
  final _realityServerName = TextEditingController(text: 'www.microsoft.com');
  final _realityPublicKey = TextEditingController();
  final _realityShortId = TextEditingController();
  final _realityRawPubkey = TextEditingController();
  final _anytlsServerName = TextEditingController();
  final _anytlsPassword = TextEditingController();
  bool _anytlsInsecureSkipVerify = true;
  final _shadowTlsServerName = TextEditingController(text: 'cloudfront.net');
  final _shadowTlsPassword = TextEditingController();

  final _localHost = TextEditingController(text: '127.0.0.1');
  final _localPort = TextEditingController(text: '1080');
  final _targetHost = TextEditingController(text: 'example.com');
  final _targetPort = TextEditingController(text: '80');
  final _payload = TextEditingController(
    text: 'HEAD / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n',
  );

  ProxyKind _proxyKind = ProxyKind.vless;
  TransportKind _transportKind = TransportKind.raw;
  OuterSecurityKind _outerSecurityKind = OuterSecurityKind.none;

  bool _busy = false;
  bool _running = false;
  String _stackSummary = '';
  String _nativeInfo = 'Native Rust client not checked';
  String _profilesStatus = '';
  String _status = 'Stopped';
  Map<String, Object?> _stats = const {};
  List<SavedProfile> _savedProfiles = const [];
  List<_ActivityEntry> _activityLog = const [];
  String? _selectedProfileId;
  AutostartStatus? _autostartStatus;
  SystemProxyStatus? _systemProxyStatus;
  WrongsvCapabilityReport? _wrongsvReport;
  WrongsvAdaptResult? _wrongsvAdaptResult;
  NativeResponse? _lastResponse;
  HealthProbeSnapshot? _lastProbe;
  HealthErrorSnapshot? _lastError;
  bool _desktopShellSyncScheduled = false;

  @override
  void initState() {
    super.initState();
    _profileName.addListener(_scheduleDesktopShellSync);
    final version = widget.client.version();
    _nativeInfo = version.ok ? _formatVersion(version.data) : version.message;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _refreshStack();
      _refreshStatus();
    });
    unawaited(_attachDesktopShell());
    _loadProfiles();
    _loadAutostartStatus();
    _loadSystemProxyStatus();
  }

  @override
  void dispose() {
    _profileName.removeListener(_scheduleDesktopShellSync);
    unawaited(widget.desktopShellController.dispose());
    _profileName.dispose();
    _clientConfigPath.dispose();
    _wrongsvConfigPath.dispose();
    _wrongsvServerHost.dispose();
    _wrongsvListenHost.dispose();
    _wrongsvListenPort.dispose();
    _serverHost.dispose();
    _serverPort.dispose();
    _uuid.dispose();
    _hysteria2ServerName.dispose();
    _hysteria2Password.dispose();
    _tuicServerName.dispose();
    _tuicUuid.dispose();
    _tuicPassword.dispose();
    _trojanPassword.dispose();
    _mixedUsername.dispose();
    _mixedPassword.dispose();
    _shadowsocksPassword.dispose();
    _kcpSeed.dispose();
    _kcpMtu.dispose();
    _kcpTti.dispose();
    _meekPath.dispose();
    _meekHost.dispose();
    _gdocsviewerPathPrefix.dispose();
    _gdocsviewerSharedKey.dispose();
    _quicServerName.dispose();
    _webtransportAuthority.dispose();
    _webtransportPath.dispose();
    _wsPath.dispose();
    _wsHost.dispose();
    _huPath.dispose();
    _huHost.dispose();
    _xhttpPath.dispose();
    _xhttpHost.dispose();
    _grpcServiceName.dispose();
    _tlsServerName.dispose();
    _tlsAlpn.dispose();
    _realityServerName.dispose();
    _realityPublicKey.dispose();
    _realityShortId.dispose();
    _realityRawPubkey.dispose();
    _anytlsServerName.dispose();
    _anytlsPassword.dispose();
    _shadowTlsServerName.dispose();
    _shadowTlsPassword.dispose();
    _localHost.dispose();
    _localPort.dispose();
    _targetHost.dispose();
    _targetPort.dispose();
    _payload.dispose();
    super.dispose();
  }

  String _formatVersion(Map<String, Object?> data) {
    final version = data['version'] ?? 'unknown';
    final proxies = (data['proxies'] as List?)?.join(', ') ?? '';
    final transports = (data['transports'] as List?)?.join(', ') ?? '';
    final outer = (data['outer_security'] as List?)?.join(', ') ?? '';
    return 'Native $version | proxies: $proxies | transports: $transports | outer: $outer';
  }

  ClientConfigInput _buildConfig() {
    return ClientConfigInput(
      serverHost: _serverHost.text,
      serverPort: int.tryParse(_serverPort.text) ?? 0,
      localHost: _localHost.text,
      localPort: int.tryParse(_localPort.text) ?? 0,
      endpoint: EndpointConfig(
        proxy: _proxyJson(),
        transport: _transportJson(),
        outerSecurity: _outerSecurityJson(),
      ),
    );
  }

  Map<String, Object?> _proxyJson() {
    switch (_proxyKind) {
      case ProxyKind.vless:
        return const VlessConfig(uuid: '').toJson()
          ..['uuid'] = _uuid.text
          ..['flow'] = _vlessVisionFlow ? 'xtls-rprx-vision' : '';
      case ProxyKind.hysteria2:
        return Hysteria2Config(
          serverName: _hysteria2ServerName.text.isEmpty
              ? 'foo.cloudfront.net'
              : _hysteria2ServerName.text,
          password: _hysteria2Password.text,
          udpEnabled: _hysteria2UdpEnabled,
        ).toJson();
      case ProxyKind.tuic:
        return TuicConfig(
          serverName: _tuicServerName.text.isEmpty
              ? 'foo.cloudfront.net'
              : _tuicServerName.text,
          uuid: _tuicUuid.text,
          password: _tuicPassword.text,
        ).toJson();
      case ProxyKind.trojan:
        return TrojanConfig(password: _trojanPassword.text).toJson();
      case ProxyKind.mixed:
        return MixedConfig(
          username: _mixedUsername.text.isEmpty ? null : _mixedUsername.text,
          password: _mixedPassword.text.isEmpty ? null : _mixedPassword.text,
        ).toJson();
      case ProxyKind.shadowsocks:
        return ShadowsocksConfig(
          method: _shadowsocksMethod,
          password: _shadowsocksPassword.text,
        ).toJson();
    }
  }

  Map<String, Object?> _transportJson() {
    switch (_transportKind) {
      case TransportKind.raw:
        return const {'type': 'raw'};
      case TransportKind.kcp:
        return KcpConfig(
          seed: _kcpSeed.text,
          mtu: int.tryParse(_kcpMtu.text) ?? 1350,
          tti: int.tryParse(_kcpTti.text) ?? 50,
        ).toJson();
      case TransportKind.meek:
        return MeekConfig(
          path: _meekPath.text.isEmpty ? '/' : _meekPath.text,
          host: _meekHost.text.isEmpty ? null : _meekHost.text,
        ).toJson();
      case TransportKind.gdocsviewer:
        return GdocsViewerConfig(
          pathPrefix: _gdocsviewerPathPrefix.text.isEmpty
              ? '/gdocsviewer'
              : _gdocsviewerPathPrefix.text,
          sharedKey: _gdocsviewerSharedKey.text.isEmpty
              ? null
              : _gdocsviewerSharedKey.text,
        ).toJson();
      case TransportKind.quic:
        return QuicConfig(
          serverName: _quicServerName.text.isEmpty
              ? 'cloudfront.net'
              : _quicServerName.text,
          udpEnabled: _quicUdpEnabled,
        ).toJson();
      case TransportKind.webtransport:
        return WebTransportConfig(
          authority: _webtransportAuthority.text.isEmpty
              ? _serverHost.text
              : _webtransportAuthority.text,
          path: _webtransportPath.text.isEmpty ? '/wt' : _webtransportPath.text,
          udpEnabled: _webtransportUdpEnabled,
        ).toJson();
      case TransportKind.websocket:
        return WsConfig(
          path: _wsPath.text.isEmpty ? '/ws' : _wsPath.text,
          host: _wsHost.text.isEmpty ? null : _wsHost.text,
        ).toJson();
      case TransportKind.httpupgrade:
        return HuConfig(
          path: _huPath.text.isEmpty ? '/up' : _huPath.text,
          host: _huHost.text.isEmpty ? null : _huHost.text,
        ).toJson();
      case TransportKind.xhttp:
        return XhttpConfig(
          path: _xhttpPath.text.isEmpty ? '/xhttp' : _xhttpPath.text,
          host: _xhttpHost.text.isEmpty ? null : _xhttpHost.text,
        ).toJson();
      case TransportKind.grpc:
        return GrpcConfig(
          serviceName: _grpcServiceName.text.isEmpty
              ? 'GunService'
              : _grpcServiceName.text,
        ).toJson();
    }
  }

  Map<String, Object?> _outerSecurityJson() {
    switch (_outerSecurityKind) {
      case OuterSecurityKind.none:
        return const {'type': 'none'};
      case OuterSecurityKind.tls:
        final alpn = _tlsAlpn.text
            .split(',')
            .map((value) => value.trim())
            .where((value) => value.isNotEmpty)
            .toList();
        return TlsConfig(
          serverName: _tlsServerName.text.isEmpty
              ? _serverHost.text
              : _tlsServerName.text,
          insecureSkipVerify: _tlsInsecure,
          alpn: alpn,
        ).toJson();
      case OuterSecurityKind.reality:
        return RealityConfig(
          serverName: _realityServerName.text.isEmpty
              ? _serverHost.text
              : _realityServerName.text,
          publicKey: _realityPublicKey.text,
          shortId: _realityShortId.text,
          rawPubkey: _realityRawPubkey.text,
        ).toJson();
      case OuterSecurityKind.anytls:
        return AnyTlsConfig(
          serverName: _anytlsServerName.text.isEmpty
              ? _serverHost.text
              : _anytlsServerName.text,
          password: _anytlsPassword.text,
          insecureSkipVerify: _anytlsInsecureSkipVerify,
        ).toJson();
      case OuterSecurityKind.shadowtls:
        return ShadowTlsConfig(
          serverName: _shadowTlsServerName.text.isEmpty
              ? 'cloudfront.net'
              : _shadowTlsServerName.text,
          password: _shadowTlsPassword.text,
        ).toJson();
    }
  }

  ProbeRequest _buildProbe() {
    return ProbeRequest(
      config: _buildConfig(),
      targetHost: _targetHost.text,
      targetPort: int.tryParse(_targetPort.text) ?? 0,
      payload: _payload.text,
    );
  }

  WrongsvAdaptRequest _buildWrongsvAdaptRequest() {
    return WrongsvAdaptRequest(
      path: _wrongsvConfigPath.text,
      serverHost: _wrongsvServerHost.text,
      listenHost: _wrongsvListenHost.text,
      listenPort: int.tryParse(_wrongsvListenPort.text) ?? 1080,
    );
  }

  Future<void> _loadProfiles() async {
    try {
      final profiles = await widget.profileStore.loadProfiles();
      if (!mounted) return;
      setState(() {
        _savedProfiles = profiles;
        if (_selectedProfileId != null &&
            !_savedProfiles.any(
              (profile) => profile.id == _selectedProfileId,
            )) {
          _selectedProfileId = null;
        }
        _profilesStatus = profiles.isEmpty ? 'No saved profiles yet' : '';
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _profilesStatus = 'Failed to load profiles: $e';
      });
    }
  }

  Future<void> _loadAutostartStatus() async {
    try {
      final status = await widget.autostartManager.loadStatus();
      if (!mounted) return;
      setState(() {
        _autostartStatus = status;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _autostartStatus = AutostartStatus(
          supported: false,
          enabled: false,
          path: '',
          message: 'Failed to inspect autostart: $e',
        );
      });
    }
  }

  Future<void> _loadSystemProxyStatus() async {
    try {
      final status = await widget.systemProxyManager.loadStatus();
      if (!mounted) return;
      setState(() {
        _systemProxyStatus = status;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _systemProxyStatus = SystemProxyStatus(
          supported: false,
          enabled: false,
          mode: 'error',
          message: 'Failed to inspect system proxy: $e',
        );
      });
    }
  }

  Future<void> _loadClientConfigFile() async {
    final response = widget.client.loadClientConfigFile(_clientConfigPath.text);
    if (!response.ok) {
      throw Exception(response.message);
    }
    final config = response.data['config'];
    if (config is! Map) {
      throw const FormatException('native config payload was not a map');
    }
    _applyConfigMap(Map<String, Object?>.from(config));
    _stackSummary = response.data['stack'] as String? ?? _stackSummary;
    if (!mounted) return;
    setState(() {
      _wrongsvReport = null;
      _wrongsvAdaptResult = null;
      _selectedProfileId = null;
      _profilesStatus = 'Loaded client config from ${_clientConfigPath.text}';
    });
    _refreshStack();
  }

  Future<void> _exportCurrentConfigJson() async {
    final path = _clientConfigPath.text.trim();
    if (path.isEmpty) {
      throw const FormatException('config file path is required');
    }
    final file = File(path);
    file.parent.createSync(recursive: true);
    const encoder = JsonEncoder.withIndent('  ');
    final payload = encoder.convert(_buildConfig().toJson());
    file.writeAsStringSync('$payload\n');
    if (!mounted) return;
    setState(() {
      _profilesStatus = 'Exported current config to $path';
    });
  }

  Future<void> _exportCurrentConfigToml() async {
    final path = _clientConfigPath.text.trim();
    if (path.isEmpty) {
      throw const FormatException('config file path is required');
    }
    final response = widget.client.exportConfigToml(_buildConfig());
    if (!response.ok) {
      throw Exception(response.message);
    }
    final toml = response.data['toml'] as String? ?? '';
    final file = File(path);
    file.parent.createSync(recursive: true);
    file.writeAsStringSync(toml);
    if (!mounted) return;
    setState(() {
      _profilesStatus = 'Exported current TOML to $path';
    });
  }

  String _formatProfileSubtitle(SavedProfile profile) {
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
    parts.add(_formatTimestamp(profile.updatedAt));
    return parts.join(' · ');
  }

  String _profileSupportBadge(SavedProfile profile) {
    return profile.supportState?.toUpperCase() ?? profile.source.toUpperCase();
  }

  String _wrongsvStatusMessage(WrongsvCapabilityReport report) {
    if (report.missingFields.isNotEmpty) {
      return 'missing fields: ${report.missingFields.map((field) => field.field).join(', ')}';
    }
    return '${report.activeProfile} is ${report.activeSupport}';
  }

  void _resetToBlankProfile() {
    _profileName.text = 'default';
    _clientConfigPath.text = '';
    _wrongsvConfigPath.text = '';
    _wrongsvServerHost.text = '127.0.0.1';
    _wrongsvListenHost.text = '127.0.0.1';
    _wrongsvListenPort.text = '1080';

    _serverHost.text = '127.0.0.1';
    _serverPort.text = '443';
    _uuid.text = '12345678-1234-1234-1234-123456789abc';
    _hysteria2ServerName.text = 'foo.cloudfront.net';
    _hysteria2Password.clear();
    _hysteria2UdpEnabled = true;
    _tuicServerName.text = 'foo.cloudfront.net';
    _tuicUuid.text = '12345678-1234-1234-1234-123456789abc';
    _tuicPassword.clear();
    _trojanPassword.clear();
    _mixedUsername.clear();
    _mixedPassword.clear();
    _shadowsocksPassword.clear();
    _shadowsocksMethod = 'chacha20-ietf-poly1305';
    _kcpSeed.clear();
    _kcpMtu.text = '1350';
    _kcpTti.text = '50';
    _meekPath.text = '/';
    _meekHost.clear();
    _gdocsviewerPathPrefix.text = '/gdocsviewer';
    _gdocsviewerSharedKey.clear();
    _quicServerName.text = 'cloudfront.net';
    _quicUdpEnabled = true;
    _webtransportAuthority.text = 'cloudfront.net';
    _webtransportPath.text = '/wt';
    _webtransportUdpEnabled = true;
    _wsPath.text = '/ws';
    _wsHost.clear();
    _huPath.text = '/up';
    _huHost.clear();
    _xhttpPath.text = '/xhttp';
    _xhttpHost.clear();
    _grpcServiceName.text = 'GunService';
    _tlsServerName.clear();
    _tlsAlpn.text = 'h2, http/1.1';
    _tlsInsecure = false;
    _vlessVisionFlow = false;
    _realityServerName.text = 'www.microsoft.com';
    _realityPublicKey.clear();
    _realityShortId.clear();
    _realityRawPubkey.clear();
    _anytlsServerName.clear();
    _anytlsPassword.clear();
    _anytlsInsecureSkipVerify = true;
    _shadowTlsServerName.text = 'cloudfront.net';
    _shadowTlsPassword.clear();

    _localHost.text = '127.0.0.1';
    _localPort.text = '1080';
    _targetHost.text = 'example.com';
    _targetPort.text = '80';
    _payload.text =
        'HEAD / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n';

    _proxyKind = ProxyKind.vless;
    _transportKind = TransportKind.raw;
    _outerSecurityKind = OuterSecurityKind.none;
    _selectedProfileId = null;
    _wrongsvReport = null;
    _wrongsvAdaptResult = null;
    _stackSummary = '';
  }

  TextEditingController? _controllerForMissingField(String field) {
    switch (field) {
      case 'reality.public-key':
        return _realityPublicKey;
      case 'reality.short-id':
        return _realityShortId;
      case 'reality.raw-pubkey':
        return _realityRawPubkey;
      case 'trojan.password':
        return _trojanPassword;
      case 'anytls.password':
        return _anytlsPassword;
      default:
        return null;
    }
  }

  String _labelForMissingField(String field) {
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
      default:
        return field;
    }
  }

  List<Widget> _wrongsvMissingFieldEditors() {
    final report = _wrongsvReport;
    if (report == null || report.missingFields.isEmpty) {
      return const [];
    }

    final editors = <Widget>[];
    final seen = <String>{};
    for (final field in report.missingFields) {
      if (!seen.add(field.field)) {
        continue;
      }
      final controller = _controllerForMissingField(field.field);
      if (controller == null) {
        editors.add(
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(
              '${field.field}: ${field.reason}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ),
        );
        continue;
      }
      editors.add(
        _field(
          controller,
          _labelForMissingField(field.field),
          520,
          key: ValueKey('missing-${field.field}'),
        ),
      );
      editors.add(
        Padding(
          padding: const EdgeInsets.only(top: 6, bottom: 8),
          child: Text(
            field.reason,
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ),
      );
    }
    return editors;
  }

  String _formatTimestamp(DateTime value) {
    final local = value.toLocal();
    final month = local.month.toString().padLeft(2, '0');
    final day = local.day.toString().padLeft(2, '0');
    final hour = local.hour.toString().padLeft(2, '0');
    final minute = local.minute.toString().padLeft(2, '0');
    return '${local.year}-$month-$day $hour:$minute';
  }

  DesktopShellState _desktopShellState() {
    final profileName = _profileName.text.trim();
    return DesktopShellState(
      running: _running,
      busy: _busy,
      status: _status,
      profileName: profileName.isEmpty ? 'default' : profileName,
    );
  }

  Future<void> _attachDesktopShell() async {
    await widget.desktopShellController.attach(
      actions: DesktopShellActions(
        startProxy: _startProxy,
        stopProxy: _stopProxy,
        refreshStatus: _refreshStatus,
        prepareForQuit: _prepareForQuit,
      ),
      initialState: _desktopShellState(),
    );
    _scheduleDesktopShellSync();
  }

  void _scheduleDesktopShellSync() {
    if (_desktopShellSyncScheduled || !mounted) {
      return;
    }
    _desktopShellSyncScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      _desktopShellSyncScheduled = false;
      if (!mounted) {
        return;
      }
      await widget.desktopShellController.sync(_desktopShellState());
    });
  }

  void _recordActivity(String title, String detail, {required bool success}) {
    final entry = _ActivityEntry(
      title: title,
      detail: detail,
      success: success,
      timestamp: DateTime.now(),
    );
    setState(() {
      _activityLog = [entry, ..._activityLog].take(20).toList();
    });
  }

  void _refreshStack() {
    try {
      final response = widget.client.stackSummary(_buildConfig());
      if (!mounted) return;
      setState(() {
        if (response.ok) {
          _stackSummary = response.data['stack'] as String? ?? '';
        } else {
          _stackSummary = 'invalid stack: ${response.message}';
        }
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _stackSummary = 'invalid stack: $e';
      });
    }
  }

  Future<void> _startProxy() {
    return _run('start', () => widget.client.startProxy(_buildConfig()));
  }

  Future<void> _stopProxy() {
    return _run('stop', () => widget.client.stopProxy());
  }

  Future<void> _refreshStatus() {
    return _run('status', () => widget.client.status());
  }

  Future<void> _prepareForQuit() async {
    if (!_running) {
      return;
    }
    try {
      await _stopProxy();
    } catch (_) {}
  }

  Future<void> _run(String action, NativeResponse Function() call) async {
    setState(() {
      _busy = true;
      _lastResponse = null;
    });
    _scheduleDesktopShellSync();

    final response = await Future<NativeResponse>(call);
    if (!mounted) {
      return;
    }

    final proxyData = response.data['proxy'];
    final summary = response.data['stack'] as String?;
    final stats = proxyData is Map<String, Object?> ? proxyData : response.data;
    final running = stats['running'];
    final localHost = stats['local_host'];
    final localPort = stats['local_port'];

    setState(() {
      _busy = false;
      _lastResponse = response;
      if (response.ok) {
        _stats = stats;
        if (summary != null && summary.isNotEmpty) {
          _stackSummary = summary;
        }
        if (action == 'probe') {
          final probe = response.data['probe'];
          if (probe is Map) {
            final payload = Map<String, Object?>.from(probe);
            _lastProbe = HealthProbeSnapshot(
              bytesRead: (payload['bytes_read'] as num?)?.toInt() ?? 0,
              preview: payload['preview'] as String? ?? '',
              timestamp: DateTime.now(),
            );
          }
        }
      } else {
        _lastError = HealthErrorSnapshot(
          action: action,
          message: response.message,
          timestamp: DateTime.now(),
        );
      }
      if (running is bool) {
        _running = running;
        _status = running ? 'Running at $localHost:$localPort' : 'Stopped';
      } else if (!response.ok) {
        _status = '$action failed';
      }
    });
    _recordActivity(action, response.message, success: response.ok);
    _scheduleDesktopShellSync();
  }

  Future<void> _runUtility(
    String action,
    NativeResponse Function() call, {
    void Function(NativeResponse response)? onSuccess,
  }) async {
    setState(() {
      _busy = true;
      _lastResponse = null;
    });
    _scheduleDesktopShellSync();

    final response = await Future<NativeResponse>(call);
    if (!mounted) {
      return;
    }

    if (response.ok) {
      onSuccess?.call(response);
    }

    setState(() {
      _busy = false;
      _lastResponse = response;
      _status = response.ok ? '$action complete' : '$action failed';
      if (!response.ok) {
        _lastError = HealthErrorSnapshot(
          action: action,
          message: response.message,
          timestamp: DateTime.now(),
        );
      }
    });
    _recordActivity(action, response.message, success: response.ok);
    _scheduleDesktopShellSync();
  }

  Future<void> _runTask(String action, Future<void> Function() task) async {
    setState(() {
      _busy = true;
      _lastResponse = null;
    });
    _scheduleDesktopShellSync();

    try {
      await task();
      if (!mounted) return;
      setState(() {
        _busy = false;
        _status = '$action complete';
      });
      _recordActivity(action, 'completed', success: true);
      _scheduleDesktopShellSync();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _status = '$action failed';
        _lastResponse = NativeResponse(
          ok: false,
          message: '$action failed: $e',
          data: const {},
        );
        _lastError = HealthErrorSnapshot(
          action: action,
          message: '$e',
          timestamp: DateTime.now(),
        );
      });
      _recordActivity(action, '$e', success: false);
      _scheduleDesktopShellSync();
    }
  }

  Future<void> _saveCurrentProfile() async {
    final existingIndex = _savedProfiles.indexWhere(
      (profile) => profile.id == _selectedProfileId,
    );
    final saved = SavedProfile(
      id: existingIndex >= 0
          ? _savedProfiles[existingIndex].id
          : DateTime.now().microsecondsSinceEpoch.toString(),
      name: _profileName.text.trim().isEmpty
          ? 'Profile ${_savedProfiles.length + 1}'
          : _profileName.text.trim(),
      config: _buildConfig().toJson(),
      stackSummary: _stackSummary,
      updatedAt: DateTime.now(),
      source: _wrongsvConfigPath.text.trim().isEmpty ? 'manual' : 'wrongsv',
      sourcePath: _wrongsvConfigPath.text.trim().isEmpty
          ? null
          : _wrongsvConfigPath.text.trim(),
      activeProfile: _wrongsvReport?.activeProfile,
      supportState: _wrongsvReport?.activeSupport,
      supportReason: _wrongsvReport?.activeReason,
      importReport: _wrongsvReport?.toMap(),
    );
    final profiles = [..._savedProfiles];
    if (existingIndex >= 0) {
      profiles[existingIndex] = saved;
    } else {
      profiles.add(saved);
    }
    await widget.profileStore.saveProfiles(profiles);
    if (!mounted) return;
    setState(() {
      _savedProfiles = [...profiles]
        ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
      _selectedProfileId = saved.id;
      _profileName.text = saved.name;
      _wrongsvConfigPath.text = saved.sourcePath ?? '';
      _profilesStatus = 'Saved profile ${saved.name}';
    });
  }

  SavedProfile? _selectedProfile() {
    final selectedId = _selectedProfileId;
    if (selectedId == null) {
      return null;
    }
    for (final profile in _savedProfiles) {
      if (profile.id == selectedId) {
        return profile;
      }
    }
    return null;
  }

  Future<void> _confirmDeleteSelectedProfile() async {
    final selected = _selectedProfile();
    if (selected == null) {
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) {
        return AlertDialog(
          title: const Text('Delete saved profile?'),
          content: Text(
            'Delete "${selected.name}" from the local profile list? This does not change the remote wrongsv server.',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(dialogContext).pop(true),
              child: const Text('Delete profile'),
            ),
          ],
        );
      },
    );
    if (!mounted || confirmed != true) {
      return;
    }
    await _runTask('delete profile', _deleteSelectedProfile);
  }

  Future<void> _deleteSelectedProfile() async {
    final selected = _selectedProfile();
    if (selected == null) {
      return;
    }
    final profiles = _savedProfiles
        .where((profile) => profile.id != selected.id)
        .toList();
    await widget.profileStore.saveProfiles(profiles);
    if (!mounted) return;
    setState(() {
      _savedProfiles = profiles;
      _selectedProfileId = null;
      _profilesStatus = 'Deleted profile ${selected.name}';
    });
  }

  Future<void> _enableAutostart() async {
    await widget.autostartManager.enable();
    await _loadAutostartStatus();
  }

  Future<void> _disableAutostart() async {
    await widget.autostartManager.disable();
    await _loadAutostartStatus();
  }

  Future<void> _enableSystemProxy() async {
    await widget.systemProxyManager.enableSocks(
      _localHost.text,
      int.tryParse(_localPort.text) ?? 1080,
    );
    await _loadSystemProxyStatus();
  }

  Future<void> _disableSystemProxy() async {
    await widget.systemProxyManager.disable();
    await _loadSystemProxyStatus();
  }

  void _newBlankProfile() {
    setState(() {
      _resetToBlankProfile();
      _profilesStatus = 'Started new profile';
    });
    _refreshStack();
  }

  Future<void> _duplicateSelectedProfile() async {
    final selected = _selectedProfile();
    if (selected == null) {
      return;
    }
    final duplicate = selected.copyWith(
      id: DateTime.now().microsecondsSinceEpoch.toString(),
      name: '${selected.name} copy',
      updatedAt: DateTime.now(),
    );
    final profiles = [..._savedProfiles, duplicate]
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    await widget.profileStore.saveProfiles(profiles);
    if (!mounted) return;
    setState(() {
      _savedProfiles = profiles;
      _selectedProfileId = duplicate.id;
      _profileName.text = duplicate.name;
      _profilesStatus = 'Duplicated profile ${selected.name}';
    });
  }

  void _loadSelectedProfile() {
    SavedProfile? profile;
    for (final candidate in _savedProfiles) {
      if (candidate.id == _selectedProfileId) {
        profile = candidate;
        break;
      }
    }
    if (profile == null) {
      return;
    }
    final selected = profile;
    _profileName.text = selected.name;
    _wrongsvConfigPath.text = selected.sourcePath ?? '';
    _applyConfigMap(selected.config);
    _stackSummary = selected.stackSummary;
    _refreshStack();
    setState(() {
      _wrongsvAdaptResult = null;
      _wrongsvReport = selected.importReport == null
          ? null
          : WrongsvCapabilityReport.fromMap(selected.importReport!);
      _profilesStatus = 'Loaded profile ${selected.name}';
    });
  }

  void _selectProfile(SavedProfile profile) {
    setState(() {
      _selectedProfileId = profile.id;
      _profileName.text = profile.name;
    });
  }

  void _applyAdaptedConfig(Map<String, Object?> data) {
    final result = WrongsvAdaptResult.fromMap(data);
    final config = result.effectiveConfig;
    if (config == null) {
      return;
    }
    _applyConfigMap(config);
  }

  void _applyConfigMap(Map<String, Object?> map) {
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

    _serverHost.text = server['host'] as String? ?? _serverHost.text;
    _serverPort.text = '${server['port'] ?? _serverPort.text}';
    _localHost.text = local['host'] as String? ?? _localHost.text;
    _localPort.text = '${local['port'] ?? _localPort.text}';

    final proxyType = proxy['type'] as String? ?? 'vless';
    _proxyKind = ProxyKind.fromId(proxyType);
    switch (_proxyKind) {
      case ProxyKind.vless:
        _uuid.text = proxy['uuid'] as String? ?? _uuid.text;
        _vlessVisionFlow =
            (proxy['flow'] as String? ?? '') == 'xtls-rprx-vision';
        break;
      case ProxyKind.hysteria2:
        _hysteria2ServerName.text =
            proxy['server-name'] as String? ?? _hysteria2ServerName.text;
        _hysteria2Password.text =
            proxy['password'] as String? ?? _hysteria2Password.text;
        _hysteria2UdpEnabled = proxy['udp-enabled'] != false;
        break;
      case ProxyKind.tuic:
        _tuicServerName.text =
            proxy['server-name'] as String? ?? _tuicServerName.text;
        _tuicUuid.text = proxy['uuid'] as String? ?? _tuicUuid.text;
        _tuicPassword.text = proxy['password'] as String? ?? _tuicPassword.text;
        break;
      case ProxyKind.trojan:
        _trojanPassword.text =
            proxy['password'] as String? ?? _trojanPassword.text;
        break;
      case ProxyKind.mixed:
        _mixedUsername.text =
            proxy['username'] as String? ?? _mixedUsername.text;
        _mixedPassword.text =
            proxy['password'] as String? ?? _mixedPassword.text;
        break;
      case ProxyKind.shadowsocks:
        _shadowsocksMethod = proxy['method'] as String? ?? _shadowsocksMethod;
        _shadowsocksPassword.text =
            proxy['password'] as String? ?? _shadowsocksPassword.text;
        break;
    }

    final transportType = transport['type'] as String? ?? 'raw';
    _transportKind = TransportKind.fromId(transportType);
    switch (_transportKind) {
      case TransportKind.raw:
        break;
      case TransportKind.kcp:
        _kcpSeed.text = transport['seed'] as String? ?? '';
        _kcpMtu.text = '${transport['mtu'] ?? _kcpMtu.text}';
        _kcpTti.text = '${transport['tti'] ?? _kcpTti.text}';
        break;
      case TransportKind.meek:
        _meekPath.text = transport['path'] as String? ?? _meekPath.text;
        _meekHost.text = transport['host'] as String? ?? '';
        break;
      case TransportKind.gdocsviewer:
        _gdocsviewerPathPrefix.text =
            transport['path-prefix'] as String? ?? _gdocsviewerPathPrefix.text;
        _gdocsviewerSharedKey.text = transport['shared-key'] as String? ?? '';
        break;
      case TransportKind.quic:
        _quicServerName.text =
            transport['server-name'] as String? ?? _quicServerName.text;
        _quicUdpEnabled = transport['udp-enabled'] != false;
        break;
      case TransportKind.webtransport:
        _webtransportAuthority.text =
            transport['authority'] as String? ?? _webtransportAuthority.text;
        _webtransportPath.text =
            transport['path'] as String? ?? _webtransportPath.text;
        _webtransportUdpEnabled = transport['udp-enabled'] != false;
        break;
      case TransportKind.websocket:
        _wsPath.text = transport['path'] as String? ?? _wsPath.text;
        _wsHost.text = transport['host'] as String? ?? '';
        break;
      case TransportKind.httpupgrade:
        _huPath.text = transport['path'] as String? ?? _huPath.text;
        _huHost.text = transport['host'] as String? ?? '';
        break;
      case TransportKind.xhttp:
        _xhttpPath.text = transport['path'] as String? ?? _xhttpPath.text;
        _xhttpHost.text = transport['host'] as String? ?? '';
        break;
      case TransportKind.grpc:
        _grpcServiceName.text =
            transport['service-name'] as String? ?? _grpcServiceName.text;
        break;
    }

    final outerType = outer['type'] as String? ?? 'none';
    _outerSecurityKind = OuterSecurityKind.fromId(outerType);
    switch (_outerSecurityKind) {
      case OuterSecurityKind.none:
        break;
      case OuterSecurityKind.tls:
        _tlsServerName.text =
            outer['server-name'] as String? ?? _tlsServerName.text;
        _tlsInsecure = outer['insecure-skip-verify'] == true;
        final alpn = (outer['alpn'] as List?)?.cast<Object?>() ?? const [];
        _tlsAlpn.text = alpn.join(', ');
        break;
      case OuterSecurityKind.reality:
        _realityServerName.text =
            outer['server-name'] as String? ?? _realityServerName.text;
        _realityPublicKey.text =
            outer['public-key'] as String? ?? _realityPublicKey.text;
        _realityShortId.text =
            outer['short-id'] as String? ?? _realityShortId.text;
        _realityRawPubkey.text =
            outer['raw-pubkey'] as String? ?? _realityRawPubkey.text;
        break;
      case OuterSecurityKind.anytls:
        _anytlsServerName.text =
            outer['server-name'] as String? ?? _anytlsServerName.text;
        _anytlsPassword.text =
            outer['password'] as String? ?? _anytlsPassword.text;
        _anytlsInsecureSkipVerify = outer['insecure-skip-verify'] != false;
        break;
      case OuterSecurityKind.shadowtls:
        _shadowTlsServerName.text =
            outer['server-name'] as String? ?? _shadowTlsServerName.text;
        _shadowTlsPassword.text =
            outer['password'] as String? ?? _shadowTlsPassword.text;
        break;
    }
    if (mounted) {
      setState(() {});
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Wrongcl')),
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, _) {
            return SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 980),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _StatusBar(
                        running: _running,
                        busy: _busy,
                        status: _status,
                        stackSummary: _stackSummary,
                        nativeInfo: _nativeInfo,
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Health',
                        child: HealthSummaryView(
                          running: _running,
                          stats: _stats,
                          lastProbe: _lastProbe,
                          lastError: _lastError,
                        ),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Activity',
                        child: _ActivityLogView(entries: _activityLog),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Connection Manager',
                        child: _StatsGrid(stats: _stats),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Desktop Integration',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              _autostartStatus == null
                                  ? 'Autostart: loading...'
                                  : 'Autostart: ${_autostartStatus!.message}',
                              style: Theme.of(context).textTheme.bodyMedium,
                            ),
                            if (_autostartStatus != null &&
                                _autostartStatus!.path.isNotEmpty) ...[
                              const SizedBox(height: 6),
                              Text(
                                _autostartStatus!.path,
                                style: Theme.of(context).textTheme.bodySmall,
                              ),
                            ],
                            const SizedBox(height: 12),
                            Wrap(
                              spacing: 12,
                              runSpacing: 12,
                              children: [
                                FilledButton.icon(
                                  onPressed:
                                      _busy ||
                                          !(_autostartStatus?.supported ??
                                              false)
                                      ? null
                                      : () => _runTask(
                                          'enable autostart',
                                          _enableAutostart,
                                        ),
                                  icon: const Icon(Icons.login),
                                  label: const Text('Enable autostart'),
                                ),
                                OutlinedButton.icon(
                                  onPressed:
                                      _busy ||
                                          !(_autostartStatus?.supported ??
                                              false)
                                      ? null
                                      : () => _runTask(
                                          'disable autostart',
                                          _disableAutostart,
                                        ),
                                  icon: const Icon(Icons.logout),
                                  label: const Text('Disable autostart'),
                                ),
                              ],
                            ),
                            const SizedBox(height: 16),
                            Text(
                              _systemProxyStatus == null
                                  ? 'System proxy: loading...'
                                  : 'System proxy: ${_systemProxyStatus!.message}',
                              style: Theme.of(context).textTheme.bodyMedium,
                            ),
                            const SizedBox(height: 12),
                            Wrap(
                              spacing: 12,
                              runSpacing: 12,
                              children: [
                                FilledButton.icon(
                                  onPressed:
                                      _busy ||
                                          !(_systemProxyStatus?.supported ??
                                              false)
                                      ? null
                                      : () => _runTask(
                                          'enable system proxy',
                                          _enableSystemProxy,
                                        ),
                                  icon: const Icon(Icons.settings_ethernet),
                                  label: const Text('Enable system proxy'),
                                ),
                                OutlinedButton.icon(
                                  onPressed:
                                      _busy ||
                                          !(_systemProxyStatus?.supported ??
                                              false)
                                      ? null
                                      : () => _runTask(
                                          'disable system proxy',
                                          _disableSystemProxy,
                                        ),
                                  icon: const Icon(Icons.portable_wifi_off),
                                  label: const Text('Disable system proxy'),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Profiles',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _responsiveWrap([
                              _field(
                                _profileName,
                                'Profile name',
                                280,
                                key: const ValueKey('profile-name'),
                              ),
                            ]),
                            const SizedBox(height: 12),
                            Wrap(
                              spacing: 12,
                              runSpacing: 12,
                              children: [
                                OutlinedButton.icon(
                                  onPressed: _busy ? null : _newBlankProfile,
                                  icon: const Icon(Icons.add_circle_outline),
                                  label: const Text('New blank'),
                                ),
                                FilledButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _runTask(
                                          'save profile',
                                          _saveCurrentProfile,
                                        ),
                                  icon: const Icon(Icons.save),
                                  label: const Text('Save current'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy || _selectedProfileId == null
                                      ? null
                                      : _loadSelectedProfile,
                                  icon: const Icon(Icons.upload_file),
                                  label: const Text('Load selected'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy || _selectedProfileId == null
                                      ? null
                                      : () => _runTask(
                                          'duplicate profile',
                                          _duplicateSelectedProfile,
                                        ),
                                  icon: const Icon(Icons.copy_all),
                                  label: const Text('Duplicate selected'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy || _selectedProfileId == null
                                      ? null
                                      : _confirmDeleteSelectedProfile,
                                  icon: const Icon(Icons.delete_outline),
                                  label: const Text('Delete selected'),
                                ),
                              ],
                            ),
                            if (_profilesStatus.isNotEmpty) ...[
                              const SizedBox(height: 8),
                              Text(
                                _profilesStatus,
                                style: Theme.of(context).textTheme.bodySmall,
                              ),
                            ],
                            const SizedBox(height: 12),
                            if (_savedProfiles.isEmpty)
                              Text(
                                'No saved profiles',
                                style: Theme.of(context).textTheme.bodySmall,
                              )
                            else
                              Column(
                                children: [
                                  for (final profile in _savedProfiles)
                                    ListTile(
                                      dense: true,
                                      contentPadding: EdgeInsets.zero,
                                      title: Text(profile.name),
                                      subtitle: Text(
                                        _formatProfileSubtitle(profile),
                                      ),
                                      trailing: _selectedProfileId == profile.id
                                          ? Row(
                                              mainAxisSize: MainAxisSize.min,
                                              children: [
                                                Text(
                                                  _profileSupportBadge(profile),
                                                  style: Theme.of(
                                                    context,
                                                  ).textTheme.labelSmall,
                                                ),
                                                const SizedBox(width: 8),
                                                const Icon(Icons.check_circle),
                                              ],
                                            )
                                          : Text(
                                              _profileSupportBadge(profile),
                                              style: Theme.of(
                                                context,
                                              ).textTheme.labelSmall,
                                            ),
                                      onTap: () => _selectProfile(profile),
                                    ),
                                ],
                              ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Client Config',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _responsiveWrap([
                              _field(
                                _clientConfigPath,
                                'wrongcl config file path',
                                520,
                                key: const ValueKey('client-config-path'),
                              ),
                            ]),
                            const SizedBox(height: 12),
                            Wrap(
                              spacing: 12,
                              runSpacing: 12,
                              children: [
                                OutlinedButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _runTask(
                                          'load client config',
                                          _loadClientConfigFile,
                                        ),
                                  icon: const Icon(Icons.file_open),
                                  label: const Text('Load client config'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _runTask(
                                          'export current config',
                                          _exportCurrentConfigJson,
                                        ),
                                  icon: const Icon(Icons.download),
                                  label: const Text('Export current JSON'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _runTask(
                                          'export current TOML',
                                          _exportCurrentConfigToml,
                                        ),
                                  icon: const Icon(Icons.description_outlined),
                                  label: const Text('Export current TOML'),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'wrongsv Import',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _responsiveWrap([
                              _field(
                                _wrongsvConfigPath,
                                'wrongsv config path',
                                520,
                                key: const ValueKey('wrongsv-config-path'),
                              ),
                            ]),
                            const SizedBox(height: 12),
                            _responsiveWrap([
                              _field(
                                _wrongsvServerHost,
                                'Adapt server host',
                                240,
                              ),
                              _field(
                                _wrongsvListenHost,
                                'Adapt listen host',
                                220,
                              ),
                              _field(
                                _wrongsvListenPort,
                                'Adapt listen port',
                                150,
                              ),
                            ]),
                            const SizedBox(height: 12),
                            Wrap(
                              spacing: 12,
                              runSpacing: 12,
                              children: [
                                OutlinedButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _runUtility(
                                          'inspect wrongsv',
                                          () => widget.client
                                              .inspectWrongsvConfig(
                                                _wrongsvConfigPath.text,
                                              ),
                                          onSuccess: (response) {
                                            if (!mounted) {
                                              return;
                                            }
                                            setState(() {
                                              _wrongsvReport =
                                                  WrongsvCapabilityReport.fromMap(
                                                    response.data,
                                                  );
                                              _wrongsvAdaptResult = null;
                                              _profilesStatus =
                                                  _wrongsvStatusMessage(
                                                    _wrongsvReport!,
                                                  );
                                            });
                                          },
                                        ),
                                  icon: const Icon(Icons.rule),
                                  label: const Text('Inspect wrongsv'),
                                ),
                                FilledButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _runUtility(
                                          'adapt wrongsv',
                                          () =>
                                              widget.client.adaptWrongsvConfig(
                                                _buildWrongsvAdaptRequest(),
                                              ),
                                          onSuccess: (response) {
                                            final result =
                                                WrongsvAdaptResult.fromMap(
                                                  response.data,
                                                );
                                            _wrongsvReport = result.report;
                                            _wrongsvAdaptResult = result;
                                            _applyAdaptedConfig(response.data);
                                            if (mounted) {
                                              setState(() {
                                                _profilesStatus =
                                                    result.effectiveConfig ==
                                                        null
                                                    ? 'Adapted report only: ${_wrongsvStatusMessage(result.report)}'
                                                    : result.config == null
                                                    ? 'Adapted draft config: ${_wrongsvStatusMessage(result.report)}'
                                                    : 'Adapted wrongsv config into the current form';
                                              });
                                            }
                                            _refreshStack();
                                          },
                                        ),
                                  icon: const Icon(Icons.sync_alt),
                                  label: const Text('Adapt into form'),
                                ),
                              ],
                            ),
                            if (_wrongsvReport != null) ...[
                              const SizedBox(height: 16),
                              _WrongsvReportView(
                                report: _wrongsvReport!,
                                stackSummary: _wrongsvAdaptResult?.stackSummary,
                              ),
                              if (_wrongsvReport!.missingFields.isNotEmpty) ...[
                                const SizedBox(height: 12),
                                Text(
                                  'Fill required client-side fields',
                                  style: Theme.of(context).textTheme.titleSmall,
                                ),
                                const SizedBox(height: 8),
                                ..._wrongsvMissingFieldEditors(),
                              ],
                            ],
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Endpoint',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _responsiveWrap([
                              _proxyDropdown(),
                              _transportDropdown(),
                              _outerSecurityDropdown(),
                            ]),
                            const SizedBox(height: 12),
                            _responsiveWrap([
                              _field(_serverHost, 'Server host', 300),
                              _field(_serverPort, 'Server port', 150),
                            ]),
                            const SizedBox(height: 12),
                            ..._proxyFields(),
                            ..._transportFields(),
                            ..._outerSecurityFields(),
                            Wrap(
                              spacing: 12,
                              runSpacing: 12,
                              children: [
                                OutlinedButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _runUtility(
                                          'validate config',
                                          () => widget.client.validateConfig(
                                            _buildConfig(),
                                          ),
                                          onSuccess: (response) {
                                            final stack =
                                                response.data['stack']
                                                    as String?;
                                            if (stack != null && mounted) {
                                              setState(() {
                                                _stackSummary = stack;
                                              });
                                            }
                                          },
                                        ),
                                  icon: const Icon(Icons.verified_outlined),
                                  label: const Text('Validate current'),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Local Proxy',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _responsiveWrap([
                              _field(_localHost, 'Listen host', 260),
                              _field(_localPort, 'Listen port', 150),
                            ]),
                            const SizedBox(height: 12),
                            Wrap(
                              spacing: 12,
                              runSpacing: 12,
                              children: [
                                FilledButton.icon(
                                  onPressed: _busy ? null : _startProxy,
                                  icon: const Icon(Icons.play_arrow),
                                  label: const Text('Start proxy'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy ? null : _stopProxy,
                                  icon: const Icon(Icons.stop),
                                  label: const Text('Stop'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy ? null : _refreshStatus,
                                  icon: const Icon(Icons.refresh),
                                  label: const Text('Refresh'),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Probe',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _responsiveWrap([
                              _field(_targetHost, 'Target host', 360),
                              _field(_targetPort, 'Target port', 150),
                            ]),
                            const SizedBox(height: 12),
                            TextField(
                              controller: _payload,
                              minLines: 4,
                              maxLines: 8,
                              decoration: const InputDecoration(
                                labelText: 'Payload',
                                alignLabelWithHint: true,
                              ),
                              style: const TextStyle(fontFamily: 'monospace'),
                            ),
                            const SizedBox(height: 12),
                            FilledButton.icon(
                              onPressed: _busy
                                  ? null
                                  : () => _run(
                                      'probe',
                                      () => widget.client.probe(_buildProbe()),
                                    ),
                              icon: const Icon(Icons.network_check),
                              label: const Text('Run probe'),
                            ),
                          ],
                        ),
                      ),
                      if (_lastResponse != null) ...[
                        const SizedBox(height: 16),
                        _Section(
                          title: _lastResponse!.ok ? 'Result' : 'Error',
                          child: SelectableText(
                            _formatResponse(_lastResponse!),
                            style: const TextStyle(fontFamily: 'monospace'),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ),
            );
          },
        ),
      ),
    );
  }

  Widget _responsiveWrap(List<Widget> children) {
    return Wrap(spacing: 12, runSpacing: 12, children: children);
  }

  Widget _field(
    TextEditingController controller,
    String label,
    double width, {
    Key? key,
  }) {
    final available = MediaQuery.sizeOf(context).width - 32;
    return SizedBox(
      width: available < width ? available : width,
      child: TextField(
        key: key,
        controller: controller,
        decoration: InputDecoration(labelText: label),
        onChanged: (_) => _refreshStack(),
      ),
    );
  }

  Widget _proxyDropdown() {
    final available = MediaQuery.sizeOf(context).width - 32;
    return SizedBox(
      width: available < 230 ? available : 230,
      child: DropdownButtonFormField<ProxyKind>(
        initialValue: _proxyKind,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Proxy'),
        items: [
          for (final kind in ProxyKind.values)
            DropdownMenuItem(value: kind, child: Text(kind.label)),
        ],
        onChanged: _busy
            ? null
            : (value) {
                if (value == null) return;
                setState(() {
                  _proxyKind = value;
                  if (value == ProxyKind.mixed ||
                      value == ProxyKind.shadowsocks ||
                      value == ProxyKind.hysteria2 ||
                      value == ProxyKind.tuic) {
                    _transportKind = TransportKind.raw;
                    _outerSecurityKind = OuterSecurityKind.none;
                  } else if (value == ProxyKind.trojan) {
                    _outerSecurityKind = OuterSecurityKind.tls;
                    if (_transportKind == TransportKind.xhttp ||
                        _transportKind == TransportKind.grpc) {
                      _transportKind = TransportKind.raw;
                    }
                  } else if (value != ProxyKind.vless &&
                      (_outerSecurityKind == OuterSecurityKind.reality ||
                          _outerSecurityKind == OuterSecurityKind.anytls ||
                          _outerSecurityKind == OuterSecurityKind.shadowtls)) {
                    _outerSecurityKind = OuterSecurityKind.none;
                  }
                });
                _refreshStack();
              },
      ),
    );
  }

  Widget _transportDropdown() {
    final available = MediaQuery.sizeOf(context).width - 32;
    final disabled =
        _busy ||
        _proxyKind == ProxyKind.mixed ||
        _proxyKind == ProxyKind.shadowsocks ||
        _proxyKind == ProxyKind.hysteria2 ||
        _proxyKind == ProxyKind.tuic ||
        _outerSecurityKind == OuterSecurityKind.reality ||
        _outerSecurityKind == OuterSecurityKind.anytls ||
        _outerSecurityKind == OuterSecurityKind.shadowtls;
    return SizedBox(
      width: available < 230 ? available : 230,
      child: DropdownButtonFormField<TransportKind>(
        initialValue: _transportKind,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Transport'),
        items: [
          for (final kind in TransportKind.values)
            DropdownMenuItem(value: kind, child: Text(kind.label)),
        ],
        onChanged: disabled
            ? null
            : (value) {
                if (value == null) return;
                setState(() {
                  _transportKind = value;
                  if (value == TransportKind.quic ||
                      value == TransportKind.kcp ||
                      value == TransportKind.webtransport) {
                    _outerSecurityKind = OuterSecurityKind.none;
                  }
                });
                _refreshStack();
              },
      ),
    );
  }

  Widget _outerSecurityDropdown() {
    final available = MediaQuery.sizeOf(context).width - 32;
    final disabled =
        _busy ||
        _proxyKind == ProxyKind.mixed ||
        _proxyKind == ProxyKind.shadowsocks ||
        _proxyKind == ProxyKind.hysteria2 ||
        _proxyKind == ProxyKind.tuic ||
        _transportKind == TransportKind.kcp ||
        _transportKind == TransportKind.quic ||
        _transportKind == TransportKind.webtransport ||
        _proxyKind == ProxyKind.trojan;
    return SizedBox(
      width: available < 230 ? available : 230,
      child: DropdownButtonFormField<OuterSecurityKind>(
        initialValue: _outerSecurityKind,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Outer security'),
        items: [
          for (final kind in OuterSecurityKind.values)
            if ((kind != OuterSecurityKind.reality &&
                    kind != OuterSecurityKind.anytls &&
                    kind != OuterSecurityKind.shadowtls) ||
                _proxyKind == ProxyKind.vless)
              DropdownMenuItem(value: kind, child: Text(kind.label)),
        ],
        onChanged: disabled
            ? null
            : (value) {
                if (value == null) return;
                setState(() {
                  _outerSecurityKind = value;
                  if (value == OuterSecurityKind.reality ||
                      value == OuterSecurityKind.anytls ||
                      value == OuterSecurityKind.shadowtls) {
                    _transportKind = TransportKind.raw;
                  }
                });
                _refreshStack();
              },
      ),
    );
  }

  List<Widget> _proxyFields() {
    switch (_proxyKind) {
      case ProxyKind.vless:
        return [
          _responsiveWrap([_field(_uuid, 'User UUID', 420)]),
          const SizedBox(height: 4),
          CheckboxListTile(
            value: _vlessVisionFlow,
            onChanged: (value) =>
                setState(() => _vlessVisionFlow = value ?? false),
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('XTLS Vision flow (xtls-rprx-vision)'),
            subtitle: const Text(
              'Adds Vision padding/splice on the inner stream. Server must enable the same flow.',
            ),
          ),
          const SizedBox(height: 12),
        ];
      case ProxyKind.hysteria2:
        return [
          _responsiveWrap([
            _field(_hysteria2ServerName, 'Hysteria2 SNI / server name', 320),
            _field(_hysteria2Password, 'Hysteria2 password', 320),
          ]),
          const SizedBox(height: 4),
          CheckboxListTile(
            value: _hysteria2UdpEnabled,
            onChanged: (value) =>
                setState(() => _hysteria2UdpEnabled = value ?? true),
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('Enable UDP relay'),
            subtitle: const Text(
              'Disable this only when the wrongsv hysteria2 server sets disable_udp = true.',
            ),
          ),
          const SizedBox(height: 12),
        ];
      case ProxyKind.tuic:
        return [
          _responsiveWrap([
            _field(_tuicServerName, 'TUIC SNI / server name', 320),
            _field(_tuicUuid, 'TUIC user UUID', 420),
          ]),
          const SizedBox(height: 8),
          _responsiveWrap([_field(_tuicPassword, 'TUIC password', 320)]),
          const SizedBox(height: 12),
        ];
      case ProxyKind.trojan:
        return [
          _responsiveWrap([_field(_trojanPassword, 'Trojan password', 360)]),
          const SizedBox(height: 12),
        ];
      case ProxyKind.mixed:
        return [
          _responsiveWrap([
            _field(_mixedUsername, 'Remote SOCKS username (optional)', 260),
            _field(_mixedPassword, 'Remote SOCKS password (optional)', 260),
          ]),
          const SizedBox(height: 12),
        ];
      case ProxyKind.shadowsocks:
        return [
          _responsiveWrap([
            _shadowsocksMethodDropdown(),
            _field(_shadowsocksPassword, 'Shadowsocks password', 320),
          ]),
          const SizedBox(height: 12),
        ];
    }
  }

  Widget _shadowsocksMethodDropdown() {
    const methods = [
      'chacha20-ietf-poly1305',
      'aes-256-gcm',
      'aes-128-gcm',
      '2022-blake3-aes-128-gcm',
      '2022-blake3-aes-256-gcm',
    ];
    final available = MediaQuery.sizeOf(context).width - 32;
    return SizedBox(
      width: available < 260 ? available : 260,
      child: DropdownButtonFormField<String>(
        initialValue: _shadowsocksMethod,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Method'),
        items: [
          for (final method in methods)
            DropdownMenuItem(value: method, child: Text(method)),
        ],
        onChanged: _busy
            ? null
            : (value) {
                if (value == null) return;
                setState(() => _shadowsocksMethod = value);
                _refreshStack();
              },
      ),
    );
  }

  List<Widget> _transportFields() {
    switch (_transportKind) {
      case TransportKind.raw:
        return const [];
      case TransportKind.kcp:
        return [
          _responsiveWrap([
            _field(_kcpSeed, 'KCP seed (optional)', 320),
            _field(_kcpMtu, 'KCP MTU', 180),
            _field(_kcpTti, 'KCP TTI (ms)', 180),
          ]),
          const SizedBox(height: 12),
        ];
      case TransportKind.meek:
        return [
          _responsiveWrap([
            _field(_meekPath, 'Meek path', 220),
            _field(_meekHost, 'Meek host header (optional)', 320),
          ]),
          const SizedBox(height: 12),
        ];
      case TransportKind.gdocsviewer:
        return [
          _responsiveWrap([
            _field(
              _gdocsviewerPathPrefix,
              'Google Docs Viewer path prefix',
              260,
            ),
            _field(
              _gdocsviewerSharedKey,
              'Google Docs Viewer shared key (optional)',
              360,
            ),
          ]),
          const SizedBox(height: 12),
        ];
      case TransportKind.quic:
        return [
          _responsiveWrap([
            _field(_quicServerName, 'QUIC SNI / server name', 320),
          ]),
          const SizedBox(height: 4),
          CheckboxListTile(
            value: _quicUdpEnabled,
            onChanged: (value) =>
                setState(() => _quicUdpEnabled = value ?? true),
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('Enable UDP relay'),
            subtitle: const Text(
              'Disable this only when the wrongsv quic transport sets udp_relay = false.',
            ),
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.webtransport:
        return [
          _responsiveWrap([
            _field(_webtransportAuthority, 'WebTransport authority / SNI', 320),
            _field(_webtransportPath, 'WebTransport path', 220),
          ]),
          const SizedBox(height: 4),
          CheckboxListTile(
            value: _webtransportUdpEnabled,
            onChanged: (value) =>
                setState(() => _webtransportUdpEnabled = value ?? true),
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('Enable UDP relay'),
            subtitle: const Text(
              'Disable this only when the wrongsv webtransport transport sets udp_relay = false.',
            ),
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.websocket:
        return [
          _responsiveWrap([
            _field(_wsPath, 'WebSocket path', 220),
            _field(_wsHost, 'WebSocket host header (optional)', 320),
          ]),
          const SizedBox(height: 12),
        ];
      case TransportKind.httpupgrade:
        return [
          _responsiveWrap([
            _field(_huPath, 'HTTPUpgrade path', 220),
            _field(_huHost, 'HTTPUpgrade host header (optional)', 320),
          ]),
          const SizedBox(height: 12),
        ];
      case TransportKind.xhttp:
        return [
          _responsiveWrap([
            _field(_xhttpPath, 'XHTTP path', 220),
            _field(_xhttpHost, 'XHTTP host header (optional)', 320),
          ]),
          const SizedBox(height: 12),
        ];
      case TransportKind.grpc:
        return [
          _responsiveWrap([_field(_grpcServiceName, 'gRPC service name', 260)]),
          const SizedBox(height: 12),
        ];
    }
  }

  List<Widget> _outerSecurityFields() {
    switch (_outerSecurityKind) {
      case OuterSecurityKind.none:
        return const [];
      case OuterSecurityKind.tls:
        return [
          _responsiveWrap([
            _field(_tlsServerName, 'TLS server name / SNI', 320),
            _field(_tlsAlpn, 'ALPN (comma separated)', 260),
          ]),
          const SizedBox(height: 8),
          Row(
            children: [
              Checkbox(
                value: _tlsInsecure,
                onChanged: _busy
                    ? null
                    : (value) {
                        setState(() => _tlsInsecure = value ?? false);
                        _refreshStack();
                      },
              ),
              const Text('Skip TLS certificate verification (insecure)'),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case OuterSecurityKind.reality:
        return [
          _responsiveWrap([
            _field(_realityServerName, 'REALITY SNI (cover server-name)', 320),
            _field(_realityShortId, 'short-id (8 hex)', 200),
          ]),
          const SizedBox(height: 8),
          _responsiveWrap([
            _field(
              _realityPublicKey,
              'public-key (server X25519, base64-url)',
              520,
            ),
          ]),
          const SizedBox(height: 8),
          _responsiveWrap([
            _field(
              _realityRawPubkey,
              'raw-pubkey for cert verify (hex, optional)',
              520,
            ),
          ]),
          const SizedBox(height: 12),
        ];
      case OuterSecurityKind.anytls:
        return [
          _responsiveWrap([
            _field(_anytlsServerName, 'AnyTLS SNI / server name', 320),
            _field(_anytlsPassword, 'AnyTLS password', 320),
          ]),
          const SizedBox(height: 8),
          Row(
            children: [
              Checkbox(
                value: _anytlsInsecureSkipVerify,
                onChanged: _busy
                    ? null
                    : (value) {
                        setState(
                          () => _anytlsInsecureSkipVerify = value ?? true,
                        );
                        _refreshStack();
                      },
              ),
              const Expanded(
                child: Text(
                  'Skip TLS certificate verification (default true — wrongsv anytls uses a self-signed cert)',
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case OuterSecurityKind.shadowtls:
        return [
          _responsiveWrap([
            _field(_shadowTlsServerName, 'ShadowTLS SNI / server name', 320),
            _field(_shadowTlsPassword, 'ShadowTLS password', 320),
          ]),
          const SizedBox(height: 8),
          const Text(
            'wrongsv shadowtls defaults to a cloudfront-style cover handshake; override the server name when the deployed fallback destination expects a different SNI.',
          ),
          const SizedBox(height: 12),
        ];
    }
  }

  String _formatResponse(NativeResponse response) {
    final buffer = StringBuffer(response.message);
    if (response.data.isNotEmpty) {
      for (final entry in response.data.entries) {
        buffer.writeln();
        buffer.write('${entry.key}: ${entry.value}');
      }
    }
    return buffer.toString();
  }
}

class _WrongsvReportView extends StatelessWidget {
  const _WrongsvReportView({required this.report, this.stackSummary});

  final WrongsvCapabilityReport report;
  final String? stackSummary;

  Color _supportColor(BuildContext context) {
    switch (report.activeSupport) {
      case 'supported':
        return Colors.green.shade700;
      case 'partial':
        return Colors.orange.shade800;
      default:
        return Theme.of(context).colorScheme.error;
    }
  }

  @override
  Widget build(BuildContext context) {
    final activeProfiles = report.profiles
        .where((profile) => profile.active)
        .toList();
    final WrongsvProfileSupport? activeProfile = activeProfiles.isEmpty
        ? null
        : activeProfiles.first;
    final previewProfiles = report.profiles.take(6).toList();

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.grey.shade50,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.grey.shade300),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _ReportChip(
                label: 'Active profile',
                value: report.activeProfile,
                color: Colors.blueGrey.shade700,
              ),
              _ReportChip(
                label: 'Support',
                value: report.activeSupport,
                color: _supportColor(context),
              ),
              _ReportChip(
                label: 'Payloads',
                value: report.payloadNetworks.join(', '),
                color: Colors.blue.shade700,
              ),
              _ReportChip(
                label: 'Carrier',
                value: report.baseCarriers.join(', '),
                color: Colors.teal.shade700,
              ),
            ],
          ),
          const SizedBox(height: 12),
          Text(
            report.activeReason,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
          if (stackSummary != null && stackSummary!.isNotEmpty) ...[
            const SizedBox(height: 8),
            Text(
              'Adapted stack: $stackSummary',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
          if (report.missingFields.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(
              'Missing client-side fields',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 6),
            for (final field in report.missingFields)
              Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: Text(
                  '${field.field}: ${field.reason}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
          ],
          if (activeProfile != null &&
              activeProfile.reason.isNotEmpty &&
              activeProfile.reason != report.activeReason) ...[
            const SizedBox(height: 12),
            Text(
              'Profile note: ${activeProfile.reason}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
          if (previewProfiles.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(
              'Recognized profiles',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 6),
            for (final profile in previewProfiles)
              Padding(
                padding: const EdgeInsets.only(bottom: 4),
                child: Text(
                  '${profile.displayName}: ${profile.support}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
          ],
        ],
      ),
    );
  }
}

class _ReportChip extends StatelessWidget {
  const _ReportChip({
    required this.label,
    required this.value,
    required this.color,
  });

  final String label;
  final String value;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      decoration: BoxDecoration(
        color: color.withAlpha(20),
        borderRadius: BorderRadius.circular(999),
      ),
      child: RichText(
        text: TextSpan(
          style: Theme.of(context).textTheme.bodySmall?.copyWith(color: color),
          children: [
            TextSpan(text: '$label: '),
            TextSpan(
              text: value,
              style: const TextStyle(fontWeight: FontWeight.w600),
            ),
          ],
        ),
      ),
    );
  }
}

class _ActivityEntry {
  const _ActivityEntry({
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

class _ActivityLogView extends StatelessWidget {
  const _ActivityLogView({required this.entries});

  final List<_ActivityEntry> entries;

  String _formatTime(DateTime value) {
    final local = value.toLocal();
    final hour = local.hour.toString().padLeft(2, '0');
    final minute = local.minute.toString().padLeft(2, '0');
    final second = local.second.toString().padLeft(2, '0');
    return '$hour:$minute:$second';
  }

  @override
  Widget build(BuildContext context) {
    if (entries.isEmpty) {
      return Text(
        'No activity yet',
        style: Theme.of(context).textTheme.bodySmall,
      );
    }

    return Column(
      children: [
        for (final entry in entries)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(
                  entry.success
                      ? Icons.check_circle_outline
                      : Icons.error_outline,
                  size: 18,
                  color: entry.success
                      ? Colors.green.shade700
                      : Theme.of(context).colorScheme.error,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        entry.title,
                        style: Theme.of(context).textTheme.labelLarge,
                      ),
                      const SizedBox(height: 2),
                      Text(
                        entry.detail,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  _formatTime(entry.timestamp),
                  style: Theme.of(context).textTheme.labelSmall,
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class _StatusBar extends StatelessWidget {
  const _StatusBar({
    required this.running,
    required this.busy,
    required this.status,
    required this.stackSummary,
    required this.nativeInfo,
  });

  final bool running;
  final bool busy;
  final String status;
  final String stackSummary;
  final String nativeInfo;

  @override
  Widget build(BuildContext context) {
    final color = running ? Colors.green.shade700 : Colors.grey.shade700;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(
              running ? Icons.check_circle : Icons.radio_button_unchecked,
              color: color,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                busy ? 'Working...' : status,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
          ],
        ),
        if (stackSummary.isNotEmpty) ...[
          const SizedBox(height: 4),
          Text(
            'Stack: $stackSummary',
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
        const SizedBox(height: 4),
        Text(
          nativeInfo,
          style: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(color: Colors.grey.shade700),
        ),
      ],
    );
  }
}

class _StatsGrid extends StatelessWidget {
  const _StatsGrid({required this.stats});

  final Map<String, Object?> stats;

  @override
  Widget build(BuildContext context) {
    final items = [
      ('Running', stats['running'] == true ? 'yes' : 'no'),
      ('Active', '${stats['active_connections'] ?? 0}'),
      ('Total', '${stats['total_connections'] ?? 0}'),
      ('Failed', '${stats['failed_connections'] ?? 0}'),
      ('Uploaded', '${stats['bytes_uploaded'] ?? 0} B'),
      ('Downloaded', '${stats['bytes_downloaded'] ?? 0} B'),
    ];

    return Wrap(
      spacing: 12,
      runSpacing: 12,
      children: [
        for (final item in items)
          SizedBox(
            width: 140,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(item.$1, style: Theme.of(context).textTheme.labelMedium),
                const SizedBox(height: 2),
                Text(item.$2, style: Theme.of(context).textTheme.titleMedium),
              ],
            ),
          ),
      ],
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({required this.title, required this.child});

  final String title;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            child,
          ],
        ),
      ),
    );
  }
}
