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
  final _path = TextEditingController(text: '/');
  final _hostHeader = TextEditingController();
  final _localHost = TextEditingController(text: '127.0.0.1');
  final _localPort = TextEditingController(text: '1080');
  final _targetHost = TextEditingController(text: 'example.com');
  final _targetPort = TextEditingController(text: '80');
  final _payload = TextEditingController(
    text: 'HEAD / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n',
  );

  bool _busy = false;
  bool _running = false;
  String _protocol = 'raw-vless-tcp';
  String _nativeInfo = 'Native Rust client not checked';
  String _status = 'Stopped';
  Map<String, Object?> _stats = const {};
  NativeResponse? _lastResponse;

  @override
  void initState() {
    super.initState();
    final version = widget.client.version();
    _nativeInfo = version.ok
        ? 'Native ${version.data['version']} | protocols: ${version.data['protocols']}'
        : version.message;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _run('status', () => widget.client.status());
    });
  }

  @override
  void dispose() {
    _serverHost.dispose();
    _serverPort.dispose();
    _uuid.dispose();
    _path.dispose();
    _hostHeader.dispose();
    _localHost.dispose();
    _localPort.dispose();
    _targetHost.dispose();
    _targetPort.dispose();
    _payload.dispose();
    super.dispose();
  }

  ClientSettings _settings() {
    return ClientSettings(
      serverHost: _serverHost.text,
      serverPort: int.tryParse(_serverPort.text) ?? 0,
      uuid: _uuid.text,
      localHost: _localHost.text,
      localPort: int.tryParse(_localPort.text) ?? 0,
      protocol: _protocol,
      path: _path.text,
      hostHeader: _hostHeader.text,
      targetHost: _targetHost.text,
      targetPort: int.tryParse(_targetPort.text) ?? 0,
      payload: _payload.text,
    );
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

    final running = response.data['running'];
    final localHost = response.data['local_host'];
    final localPort = response.data['local_port'];
    setState(() {
      _busy = false;
      _lastResponse = response;
      if (response.ok) {
        _stats = response.data;
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
                        nativeInfo: _nativeInfo,
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Connection Manager',
                        child: _StatsGrid(stats: _stats),
                      ),
                      const SizedBox(height: 16),
                      _Section(
                        title: 'Server',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _responsiveWrap([
                              _protocolField(),
                              _field(_serverHost, 'Server host', 300),
                              _field(_serverPort, 'Server port', 150),
                              _field(_uuid, 'User UUID', 420),
                              _field(_path, 'Carrier path', 180),
                              _field(_hostHeader, 'Host header', 260),
                            ]),
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
                                            _settings(),
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
                                      () => widget.client.probe(_settings()),
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
      ),
    );
  }

  Widget _protocolField() {
    final available = MediaQuery.sizeOf(context).width - 32;
    return SizedBox(
      width: available < 230 ? available : 230,
      child: DropdownButtonFormField<String>(
        initialValue: _protocol,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Protocol'),
        items: const [
          DropdownMenuItem(value: 'raw-vless-tcp', child: Text('Raw TCP')),
          DropdownMenuItem(value: 'vless-websocket', child: Text('WebSocket')),
          DropdownMenuItem(
            value: 'vless-httpupgrade',
            child: Text('HTTPUpgrade'),
          ),
        ],
        onChanged: _busy
            ? null
            : (value) {
                if (value == null) {
                  return;
                }
                setState(() {
                  _protocol = value;
                  if (value == 'vless-websocket' && _path.text == '/') {
                    _path.text = '/ws';
                  } else if (value == 'vless-httpupgrade' &&
                      _path.text == '/') {
                    _path.text = '/up';
                  }
                });
              },
      ),
    );
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
    required this.nativeInfo,
  });

  final bool running;
  final bool busy;
  final String status;
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
