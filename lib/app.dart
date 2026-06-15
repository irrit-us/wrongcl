import 'package:flutter/material.dart';

import 'wrongcl_client.dart';

class WrongclApp extends StatelessWidget {
  WrongclApp({super.key, WrongclClient? client})
    : client = client ?? NativeWrongclClient();

  final WrongclClient client;

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
      home: ClientHome(client: client),
    );
  }
}

class ClientHome extends StatefulWidget {
  const ClientHome({super.key, required this.client});

  final WrongclClient client;

  @override
  State<ClientHome> createState() => _ClientHomeState();
}

class _ClientHomeState extends State<ClientHome> {
  final _serverHost = TextEditingController(text: '127.0.0.1');
  final _serverPort = TextEditingController(text: '443');
  final _uuid = TextEditingController(
    text: '12345678-1234-1234-1234-123456789abc',
  );
  final _trojanPassword = TextEditingController();
  final _mixedUsername = TextEditingController();
  final _mixedPassword = TextEditingController();
  final _shadowsocksPassword = TextEditingController();
  String _shadowsocksMethod = 'chacha20-ietf-poly1305';
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
  String _status = 'Stopped';
  Map<String, Object?> _stats = const {};
  NativeResponse? _lastResponse;

  @override
  void initState() {
    super.initState();
    final version = widget.client.version();
    _nativeInfo = version.ok
        ? _formatVersion(version.data)
        : version.message;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _refreshStack();
      _run('status', () => widget.client.status());
    });
  }

  @override
  void dispose() {
    _serverHost.dispose();
    _serverPort.dispose();
    _uuid.dispose();
    _trojanPassword.dispose();
    _mixedUsername.dispose();
    _mixedPassword.dispose();
    _shadowsocksPassword.dispose();
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

  Future<void> _run(String action, NativeResponse Function() call) async {
    setState(() {
      _busy = true;
      _lastResponse = null;
    });

    final response = await Future<NativeResponse>(call);
    if (!mounted) {
      return;
    }

    final proxyData = response.data['proxy'];
    final summary = response.data['stack'] as String?;
    final stats = proxyData is Map<String, Object?>
        ? proxyData
        : response.data;
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
      }
      if (running is bool) {
        _running = running;
        _status = running ? 'Running at $localHost:$localPort' : 'Stopped';
      } else if (!response.ok) {
        _status = '$action failed';
      }
    });
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
                        title: 'Connection Manager',
                        child: _StatsGrid(stats: _stats),
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
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Local SOCKS5',
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
                                  onPressed: _busy
                                      ? null
                                      : () => _run(
                                          'start',
                                          () => widget.client.startProxy(
                                            _buildConfig(),
                                          ),
                                        ),
                                  icon: const Icon(Icons.play_arrow),
                                  label: const Text('Start proxy'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _run(
                                          'stop',
                                          () => widget.client.stopProxy(),
                                        ),
                                  icon: const Icon(Icons.stop),
                                  label: const Text('Stop'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _busy
                                      ? null
                                      : () => _run(
                                          'status',
                                          () => widget.client.status(),
                                        ),
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

  Widget _field(TextEditingController controller, String label, double width) {
    final available = MediaQuery.sizeOf(context).width - 32;
    return SizedBox(
      width: available < width ? available : width,
      child: TextField(
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
                      value == ProxyKind.shadowsocks) {
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
                          _outerSecurityKind == OuterSecurityKind.anytls)) {
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
    final disabled = _busy ||
        _proxyKind == ProxyKind.mixed ||
        _proxyKind == ProxyKind.shadowsocks ||
        _outerSecurityKind == OuterSecurityKind.reality ||
        _outerSecurityKind == OuterSecurityKind.anytls;
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
                setState(() => _transportKind = value);
                _refreshStack();
              },
      ),
    );
  }

  Widget _outerSecurityDropdown() {
    final available = MediaQuery.sizeOf(context).width - 32;
    final disabled = _busy ||
        _proxyKind == ProxyKind.mixed ||
        _proxyKind == ProxyKind.shadowsocks ||
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
                    kind != OuterSecurityKind.anytls) ||
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
                      value == OuterSecurityKind.anytls) {
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
          _responsiveWrap([
            _field(_grpcServiceName, 'gRPC service name', 260),
          ]),
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
            _field(_realityPublicKey, 'public-key (server X25519, base64-url)', 520),
          ]),
          const SizedBox(height: 8),
          _responsiveWrap([
            _field(_realityRawPubkey, 'raw-pubkey for cert verify (hex, optional)', 520),
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
