import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/autostart_manager.dart';
import 'package:wrongcl/client_home_controller.dart';
import 'package:wrongcl/desktop_shell_controller.dart';
import 'package:wrongcl/profile_store.dart';
import 'package:wrongcl/system_proxy_manager.dart';
import 'package:wrongcl/wrongcl_client.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('controller signal snapshot starts empty before runtime samples', () {
    final controller = _buildController(_SignalTestClient());

    final snapshot = controller.dashboardSnapshot.signalSnapshot;
    expect(snapshot.activeConnectionsSeries.points, isEmpty);
    expect(snapshot.uploadedBytesSeries.points, isEmpty);
    expect(snapshot.recentProbeOutcomes, isEmpty);
  });

  test('controller history appends runtime points and caps at 60', () async {
    final client = _SignalTestClient();
    final controller = _buildController(client);

    for (var i = 0; i < 65; i++) {
      await controller.refreshStatus();
    }

    final snapshot = controller.dashboardSnapshot.signalSnapshot;
    expect(snapshot.activeConnectionsSeries.points.length, 60);
    expect(snapshot.activeConnectionsSeries.points.first.value, 6);
    expect(snapshot.activeConnectionsSeries.points.last.value, 65);
  });

  test(
    'controller records probe success and failure events truthfully',
    () async {
      final client = _SignalTestClient();
      final controller = _buildController(client);

      await controller.runProbe();
      await controller.runProbe();

      final events =
          controller.dashboardSnapshot.signalSnapshot.recentProbeOutcomes;
      expect(events.length, 2);
      expect(events.first.success, isTrue);
      expect(events.last.success, isFalse);
    },
  );

  test(
    'refreshConnections parses live connections from native snapshot',
    () async {
      final client = _ConnectionsTestClient();
      client.liveConnections = [
        {
          'id': 7,
          'started_at_unix_ms': 1700000000000,
          'peer_addr': '127.0.0.1:55432',
          'target': 'example.com:443',
          'source_pid': null,
          'source_app': 'curl',
          'url': null,
          'bytes_up': 128,
          'bytes_down': 4096,
          'state': 'active',
        },
      ];
      final controller = _buildController(client);

      await controller.refreshConnections();

      expect(controller.activeConnections.length, 1);
      final entry = controller.activeConnections.single;
      expect(entry.id, 7);
      expect(entry.target, 'example.com:443');
      expect(entry.sourceApp, 'curl');
      expect(entry.bytesUp, 128);
      expect(entry.bytesDown, 4096);
    },
  );

  test(
    'closeConnection asks native to close and removes the row locally',
    () async {
      final client = _ConnectionsTestClient();
      client.liveConnections = [
        {
          'id': 11,
          'started_at_unix_ms': 1700000000000,
          'peer_addr': '127.0.0.1:0',
          'target': 'a.example:443',
          'source_app': '',
          'bytes_up': 0,
          'bytes_down': 0,
          'state': 'active',
        },
      ];
      final controller = _buildController(client);
      await controller.refreshConnections();
      expect(controller.activeConnections, hasLength(1));

      controller.closeConnection(11);

      expect(client.closeConnectionCount, 1);
      expect(controller.activeConnections, isEmpty);
    },
  );

  test(
    'closeAllConnections invokes connectionsCloseMatching with empty filter',
    () async {
      final client = _ConnectionsTestClient();
      client.liveConnections = [
        {
          'id': 1,
          'started_at_unix_ms': 1700000000000,
          'peer_addr': '127.0.0.1:0',
          'target': 'a.example:443',
          'source_app': '',
          'bytes_up': 0,
          'bytes_down': 0,
          'state': 'active',
        },
        {
          'id': 2,
          'started_at_unix_ms': 1700000000000,
          'peer_addr': '127.0.0.1:0',
          'target': 'b.example:443',
          'source_app': '',
          'bytes_up': 0,
          'bytes_down': 0,
          'state': 'active',
        },
      ];
      final controller = _buildController(client);
      await controller.refreshConnections();
      expect(controller.activeConnections, hasLength(2));

      controller.closeAllConnections();

      expect(client.closeMatchingCount, 1);
      expect(client.lastCloseMatchingFilter, isEmpty);
      expect(controller.activeConnections, isEmpty);
    },
  );

  test(
    'refreshLogs appends entries and advances cursor across calls',
    () async {
      final client = _ConnectionsTestClient();
      client.pendingLogs = [
        {
          'seq': 1,
          'ts_unix_ms': 1700000000000,
          'level': 'INFO',
          'target': 'wrongcl::proxy',
          'message': 'first',
          'fields': const <String, Object?>{},
        },
        {
          'seq': 2,
          'ts_unix_ms': 1700000000050,
          'level': 'WARN',
          'target': 'wrongcl::proxy',
          'message': 'second',
          'fields': const <String, Object?>{},
        },
      ];
      final controller = _buildController(client);

      await controller.refreshLogs();
      expect(controller.recentLogs.map((e) => e.message), ['first', 'second']);

      client.pendingLogs = [
        ...client.pendingLogs,
        {
          'seq': 3,
          'ts_unix_ms': 1700000000100,
          'level': 'ERROR',
          'target': 'wrongcl::proxy',
          'message': 'third',
          'fields': const <String, Object?>{},
        },
      ];
      await controller.refreshLogs();
      expect(controller.recentLogs.map((e) => e.message), [
        'first',
        'second',
        'third',
      ]);
      expect(controller.recentLogs.last.level, 'ERROR');
    },
  );

  test('log level filter narrows visible logs by minimum severity', () async {
    final client = _ConnectionsTestClient();
    client.pendingLogs = [
      {
        'seq': 1,
        'ts_unix_ms': 1700000000000,
        'level': 'DEBUG',
        'target': 'wrongcl::proxy',
        'message': 'debug',
        'fields': const <String, Object?>{},
      },
      {
        'seq': 2,
        'ts_unix_ms': 1700000000050,
        'level': 'INFO',
        'target': 'wrongcl::proxy',
        'message': 'info',
        'fields': const <String, Object?>{},
      },
      {
        'seq': 3,
        'ts_unix_ms': 1700000000100,
        'level': 'ERROR',
        'target': 'wrongcl::proxy',
        'message': 'error',
        'fields': const <String, Object?>{},
      },
    ];
    final controller = _buildController(client);

    await controller.refreshLogs();
    controller.setLogLevelFilter(LogLevelFilter.warn);

    expect(controller.visibleLogs.map((entry) => entry.message), ['error']);
  });

  test(
    'refreshRequests appends entries and advances cursor across calls',
    () async {
      final client = _ConnectionsTestClient();
      client.pendingRequests = [
        {
          'seq': 1,
          'ts_unix_ms': 1700000000000,
          'conn_id': 11,
          'target': 'example.com:443',
          'method': 'CONNECT',
          'url': null,
          'host': 'example.com:443',
          'source_pid': null,
          'source_app': 'curl',
        },
        {
          'seq': 2,
          'ts_unix_ms': 1700000000100,
          'conn_id': 12,
          'target': 'example.com:80',
          'method': 'GET',
          'url': 'GET /index',
          'host': 'example.com',
          'source_pid': 4242,
          'source_app': 'wget',
        },
      ];
      final controller = _buildController(client);

      await controller.refreshRequests();
      expect(controller.recentRequests.map((r) => r.method), [
        'CONNECT',
        'GET',
      ]);
      expect(controller.recentRequests.first.host, 'example.com:443');
      expect(controller.recentRequests.last.sourcePid, 4242);

      client.pendingRequests = [
        ...client.pendingRequests,
        {
          'seq': 3,
          'ts_unix_ms': 1700000000200,
          'conn_id': 13,
          'target': 'b.example:443',
          'method': 'CONNECT',
          'url': null,
          'host': 'b.example:443',
          'source_pid': null,
          'source_app': '',
        },
      ];
      await controller.refreshRequests();
      expect(controller.recentRequests.map((r) => r.target), [
        'example.com:443',
        'example.com:80',
        'b.example:443',
      ]);
    },
  );

  test('buildConfig preserves loaded routing config fields', () {
    final controller = _buildController(_ConnectionsTestClient());

    controller.applyConfigMap(_phaseFiveConfigMap());

    final config = controller.buildConfig();
    expect(config.endpoints.map((e) => e.name), ['default', 'backup']);
    expect(config.active.kind, 'group');
    expect(config.active.name, 'auto');
    expect(config.groups.single.selected, 'backup');
    expect(config.scripts.map((s) => s.name), ['split']);
    expect(config.modes.map((m) => m.name), [
      'global',
      'rule',
      'direct',
      'travel',
    ]);
    expect(config.activeMode, 'travel');
    expect(config.dns['mode'], 'system');
  });

  test(
    'upsertUserMode stores a new draft mode while proxy is stopped',
    () async {
      final controller = _buildController(_ConnectionsTestClient());

      controller.applyConfigMap(_phaseFiveConfigMap());

      final response = await controller.upsertUserMode(
        const RouterMode(
          name: 'office',
          kind: 'user',
          proxy: 'auto',
          script: 'split',
        ),
      );

      expect(response.ok, isTrue);
      expect(controller.modeSlots.map((slot) => slot.id), contains('office'));
      expect(
        controller.currentRouterSnapshot.modes.map((m) => m.name),
        contains('office'),
      );
      expect(
        controller.buildConfig().modes.map((m) => m.name),
        contains('office'),
      );
    },
  );

  test('startProxy sends loaded routing config to native', () async {
    final client = _ConnectionsTestClient();
    final controller = _buildController(client);

    controller.applyConfigMap(_phaseFiveConfigMap());

    await controller.startProxy();

    final config = client.lastStartConfig;
    expect(config, isNotNull);
    expect(config!.groups.single.name, 'auto');
    expect(config.scripts.single.name, 'split');
    expect(config.modes.last.name, 'travel');
    expect(config.activeMode, 'travel');
  });

  test('setDnsSettings stores DNS backend while proxy is stopped', () async {
    final controller = _buildController(_ConnectionsTestClient());

    final response = await controller.setDnsSettings(
      const DnsSettingsInput(
        kind: DnsBackendKind.doh,
        url: 'https://1.1.1.1/dns-query',
      ),
    );

    expect(response.ok, isTrue);
    expect(controller.buildConfig().dns, {
      'backend': {'kind': 'doh', 'url': 'https://1.1.1.1/dns-query'},
    });
  });

  test('loadTunStatus updates tun availability from native status', () async {
    final client = _ConnectionsTestClient();
    client.tunStatus = const {
      'supported': false,
      'enabled': false,
      'disabled_reason':
          'Needs privileges: CAP_NET_ADMIN is required for TUN setup.',
    };
    final controller = _buildController(client);

    await controller.loadTunStatus();

    expect(controller.tunAvailability.supported, isFalse);
    expect(controller.tunAvailability.enabled, isFalse);
    expect(
      controller.tunAvailability.disabledReason,
      'Needs privileges: CAP_NET_ADMIN is required for TUN setup.',
    );
  });

  test('applyRawConfigEditorJson updates the current draft', () async {
    final controller = _buildController(_ConnectionsTestClient());
    controller.rawConfigEditor.text = '''
{
  "endpoints": [
    {
      "name": "default",
      "host": "config.example",
      "port": 9443,
      "proxy": {
        "type": "trojan",
        "password": "secret"
      },
      "transport": {
        "type": "raw"
      },
      "outer-security": {
        "type": "tls",
        "server-name": "config.example"
      }
    }
  ],
  "active": {"type": "endpoint", "name": "default"},
  "local": {"host": "127.0.0.1", "port": 2080}
}
''';

    await controller.applyRawConfigEditorJson();

    final config = controller.buildConfig();
    expect(config.endpoints.single.host, 'config.example');
    expect(config.endpoints.single.port, 9443);
    expect(config.localPort, 2080);
  });

  test('local protocol toggles flow into the built config', () {
    final controller = _buildController(_ConnectionsTestClient());

    controller.setLocalSocksEnabled(false);

    final config = controller.buildConfig();
    expect(config.allowSocks, isFalse);
    expect(config.allowHttp, isTrue);
  });
}

ClientHomeController _buildController(WrongclClient client) {
  final tempDir = Directory.systemTemp.createTempSync(
    'wrongcl-controller-test',
  );
  return ClientHomeController(
    client: client,
    profileStore: ProfileStore(file: File('${tempDir.path}/profiles.json')),
    autostartManager: AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
    ),
    systemProxyManager: SystemProxyManager(
      platform: SystemProxyPlatform.unsupported,
    ),
    desktopShellController: const NoopDesktopShellController(),
  );
}

class _SignalTestClient implements WrongclClient {
  int statusCount = 0;
  int probeCount = 0;
  Map<String, Object?> dnsSettings = const {
    'backend': {'kind': 'system'},
  };
  Map<String, Object?> tunStatus = const {
    'supported': false,
    'enabled': false,
    'disabled_reason': 'TUN driver lands in Phase 7 of PLAN.md',
  };

  @override
  NativeResponse version() => const NativeResponse(
    ok: true,
    message: 'ready',
    data: {
      'version': 'test',
      'proxies': [],
      'transports': [],
      'outer_security': [],
    },
  );

  @override
  NativeResponse startProxy(ClientConfigInput config) {
    return const NativeResponse(
      ok: true,
      message: 'started',
      data: {
        'stack': 'VLESS → raw → TCP',
        'proxy': {
          'running': true,
          'local_host': '127.0.0.1',
          'local_port': 1080,
          'active_connections': 1,
          'total_connections': 1,
          'failed_connections': 0,
          'bytes_uploaded': 128,
          'bytes_downloaded': 256,
        },
      },
    );
  }

  @override
  NativeResponse stopProxy() => const NativeResponse(
    ok: true,
    message: 'stopped',
    data: {
      'running': false,
      'active_connections': 0,
      'total_connections': 1,
      'failed_connections': 0,
      'bytes_uploaded': 128,
      'bytes_downloaded': 256,
    },
  );

  @override
  NativeResponse status() {
    statusCount += 1;
    return NativeResponse(
      ok: true,
      message: 'status ok',
      data: {
        'running': true,
        'local_host': '127.0.0.1',
        'local_port': 1080,
        'active_connections': statusCount,
        'total_connections': statusCount,
        'failed_connections': statusCount ~/ 10,
        'bytes_uploaded': statusCount * 100,
        'bytes_downloaded': statusCount * 120,
      },
    );
  }

  @override
  NativeResponse probe(ProbeRequest request) {
    probeCount += 1;
    if (probeCount == 1) {
      return const NativeResponse(
        ok: true,
        message: 'probe succeeded',
        data: {
          'stack': 'VLESS → raw → TCP',
          'probe': {'bytes_read': 4, 'preview': 'pong'},
        },
      );
    }
    return const NativeResponse(ok: false, message: 'probe failed', data: {});
  }

  @override
  NativeResponse stackSummary(ClientConfigInput config) => const NativeResponse(
    ok: true,
    message: 'stack',
    data: {'stack': 'VLESS → raw → TCP'},
  );

  @override
  NativeResponse validateConfig(ClientConfigInput config) =>
      const NativeResponse(
        ok: true,
        message: 'valid',
        data: {'stack': 'VLESS → raw → TCP'},
      );

  @override
  NativeResponse loadClientConfigFile(String path) =>
      throw UnimplementedError();

  @override
  NativeResponse exportConfigToml(ClientConfigInput config) =>
      throw UnimplementedError();

  @override
  NativeResponse inspectWrongsvConfig(String path) =>
      throw UnimplementedError();

  @override
  NativeResponse adaptWrongsvConfig(WrongsvAdaptRequest request) =>
      throw UnimplementedError();

  @override
  NativeResponse connectionsList() => const NativeResponse(
    ok: true,
    message: 'connections snapshot',
    data: {
      'connections': [],
      'active': 0,
      'total': 0,
      'failed': 0,
      'bytes_uploaded': 0,
      'bytes_downloaded': 0,
    },
  );

  @override
  NativeResponse connectionClose(int id) => NativeResponse(
    ok: true,
    message: 'connection close requested',
    data: {'id': id, 'closed': false},
  );

  @override
  NativeResponse connectionsCloseMatching(Map<String, Object?> filter) =>
      const NativeResponse(
        ok: true,
        message: 'connections close requested',
        data: {'closed': 0},
      );

  @override
  NativeResponse logsSince(int cursor) => const NativeResponse(
    ok: true,
    message: 'logs snapshot',
    data: {'entries': [], 'cursor': 0, 'capacity': 2000},
  );

  @override
  NativeResponse requestsSince(int cursor) => const NativeResponse(
    ok: true,
    message: 'requests snapshot',
    data: {'entries': [], 'cursor': 0, 'capacity': 500},
  );

  @override
  NativeResponse proxyGroupsJson() => const NativeResponse(
    ok: true,
    message: 'proxy groups snapshot',
    data: {'endpoints': [], 'groups': [], 'active': null},
  );

  @override
  NativeResponse proxyGroupSelect(String group, String member) =>
      const NativeResponse(
        ok: true,
        message: 'group member selected',
        data: {},
      );

  @override
  NativeResponse dnsSettingsJson() => NativeResponse(
    ok: true,
    message: 'dns settings snapshot',
    data: dnsSettings,
  );

  @override
  NativeResponse dnsSettingsSet(Map<String, Object?> settings) {
    dnsSettings = Map<String, Object?>.from(settings);
    return NativeResponse(
      ok: true,
      message: 'DNS settings saved',
      data: dnsSettings,
    );
  }

  @override
  NativeResponse tunStatusJson() =>
      NativeResponse(ok: true, message: 'TUN status snapshot', data: tunStatus);

  @override
  NativeResponse tunEnable(Map<String, Object?> config) => NativeResponse(
    ok: false,
    message: tunStatus['disabled_reason'] as String,
    data: tunStatus,
  );

  @override
  NativeResponse tunDisable() =>
      NativeResponse(ok: true, message: 'TUN disabled', data: tunStatus);

  @override
  NativeResponse routerSnapshotJson() => const NativeResponse(
    ok: true,
    message: 'router snapshot',
    data: {
      'modes': [
        {'name': 'global', 'kind': 'global', 'proxy': null, 'script': null},
        {'name': 'rule', 'kind': 'rule', 'proxy': null, 'script': null},
        {'name': 'direct', 'kind': 'direct', 'proxy': null, 'script': null},
      ],
      'scripts': [],
      'active_mode': 'global',
    },
  );

  @override
  NativeResponse routerSetActiveMode(String name) => NativeResponse(
    ok: true,
    message: 'active mode set',
    data: {'active_mode': name},
  );

  @override
  NativeResponse routerSetScript(Map<String, Object?> script) =>
      const NativeResponse(ok: true, message: 'script saved', data: {});

  @override
  NativeResponse routerRemoveScript(String name) =>
      const NativeResponse(ok: true, message: 'script removed', data: {});

  @override
  NativeResponse routerUpsertUserMode(Map<String, Object?> mode) =>
      const NativeResponse(ok: true, message: 'mode saved', data: {});

  @override
  NativeResponse routerRemoveUserMode(String name) =>
      const NativeResponse(ok: true, message: 'mode removed', data: {});
}

class _ConnectionsTestClient implements WrongclClient {
  List<Map<String, Object?>> liveConnections = const [];
  List<Map<String, Object?>> pendingLogs = const [];
  List<Map<String, Object?>> pendingRequests = const [];
  int closeConnectionCount = 0;
  int closeMatchingCount = 0;
  Map<String, Object?>? lastCloseMatchingFilter;
  ClientConfigInput? lastStartConfig;
  Map<String, Object?> dnsSettings = const {
    'backend': {'kind': 'system'},
  };
  Map<String, Object?> tunStatus = const {
    'supported': false,
    'enabled': false,
    'disabled_reason': 'TUN driver lands in Phase 7 of PLAN.md',
  };

  @override
  NativeResponse version() => const NativeResponse(
    ok: true,
    message: 'ready',
    data: {
      'version': 'test',
      'proxies': [],
      'transports': [],
      'outer_security': [],
    },
  );

  @override
  NativeResponse startProxy(ClientConfigInput config) {
    lastStartConfig = config;
    return const NativeResponse(ok: true, message: 'started', data: {});
  }

  @override
  NativeResponse stopProxy() =>
      const NativeResponse(ok: true, message: 'stopped', data: {});

  @override
  NativeResponse status() =>
      const NativeResponse(ok: true, message: 'status', data: {});

  @override
  NativeResponse probe(ProbeRequest request) =>
      const NativeResponse(ok: true, message: 'probed', data: {});

  @override
  NativeResponse stackSummary(ClientConfigInput config) =>
      const NativeResponse(ok: true, message: 'stack', data: {});

  @override
  NativeResponse validateConfig(ClientConfigInput config) =>
      const NativeResponse(ok: true, message: 'valid', data: {});

  @override
  NativeResponse loadClientConfigFile(String path) =>
      throw UnimplementedError();

  @override
  NativeResponse exportConfigToml(ClientConfigInput config) =>
      throw UnimplementedError();

  @override
  NativeResponse inspectWrongsvConfig(String path) =>
      throw UnimplementedError();

  @override
  NativeResponse adaptWrongsvConfig(WrongsvAdaptRequest request) =>
      throw UnimplementedError();

  @override
  NativeResponse connectionsList() {
    return NativeResponse(
      ok: true,
      message: 'connections snapshot',
      data: {
        'connections': liveConnections,
        'active': liveConnections.length,
        'total': liveConnections.length,
        'failed': 0,
        'bytes_uploaded': 0,
        'bytes_downloaded': 0,
      },
    );
  }

  @override
  NativeResponse connectionClose(int id) {
    closeConnectionCount += 1;
    liveConnections = liveConnections
        .where((c) => ((c['id'] as num?)?.toInt() ?? -1) != id)
        .toList(growable: false);
    return NativeResponse(
      ok: true,
      message: 'connection close requested',
      data: {'id': id, 'closed': true},
    );
  }

  @override
  NativeResponse connectionsCloseMatching(Map<String, Object?> filter) {
    closeMatchingCount += 1;
    lastCloseMatchingFilter = Map<String, Object?>.from(filter);
    final closed = liveConnections.length;
    liveConnections = const [];
    return NativeResponse(
      ok: true,
      message: 'connections close requested',
      data: {'closed': closed},
    );
  }

  @override
  NativeResponse logsSince(int cursor) {
    final start = cursor.clamp(0, pendingLogs.length);
    final delivered = pendingLogs.sublist(start);
    return NativeResponse(
      ok: true,
      message: 'logs snapshot',
      data: {
        'entries': delivered,
        'cursor': pendingLogs.length,
        'capacity': 2000,
      },
    );
  }

  @override
  NativeResponse requestsSince(int cursor) {
    final start = cursor.clamp(0, pendingRequests.length);
    final delivered = pendingRequests.sublist(start);
    return NativeResponse(
      ok: true,
      message: 'requests snapshot',
      data: {
        'entries': delivered,
        'cursor': pendingRequests.length,
        'capacity': 500,
      },
    );
  }

  @override
  NativeResponse proxyGroupsJson() => const NativeResponse(
    ok: true,
    message: 'proxy groups snapshot',
    data: {'endpoints': [], 'groups': [], 'active': null},
  );

  @override
  NativeResponse proxyGroupSelect(String group, String member) =>
      const NativeResponse(
        ok: true,
        message: 'group member selected',
        data: {},
      );

  @override
  NativeResponse dnsSettingsJson() => NativeResponse(
    ok: true,
    message: 'dns settings snapshot',
    data: dnsSettings,
  );

  @override
  NativeResponse dnsSettingsSet(Map<String, Object?> settings) {
    dnsSettings = Map<String, Object?>.from(settings);
    return NativeResponse(
      ok: true,
      message: 'DNS settings saved',
      data: dnsSettings,
    );
  }

  @override
  NativeResponse tunStatusJson() =>
      NativeResponse(ok: true, message: 'TUN status snapshot', data: tunStatus);

  @override
  NativeResponse tunEnable(Map<String, Object?> config) => NativeResponse(
    ok: false,
    message: tunStatus['disabled_reason'] as String,
    data: tunStatus,
  );

  @override
  NativeResponse tunDisable() =>
      NativeResponse(ok: true, message: 'TUN disabled', data: tunStatus);

  @override
  NativeResponse routerSnapshotJson() => const NativeResponse(
    ok: true,
    message: 'router snapshot',
    data: {
      'modes': [
        {'name': 'global', 'kind': 'global', 'proxy': null, 'script': null},
        {'name': 'rule', 'kind': 'rule', 'proxy': null, 'script': null},
        {'name': 'direct', 'kind': 'direct', 'proxy': null, 'script': null},
      ],
      'scripts': [],
      'active_mode': 'global',
    },
  );

  @override
  NativeResponse routerSetActiveMode(String name) => NativeResponse(
    ok: true,
    message: 'active mode set',
    data: {'active_mode': name},
  );

  @override
  NativeResponse routerSetScript(Map<String, Object?> script) =>
      const NativeResponse(ok: true, message: 'script saved', data: {});

  @override
  NativeResponse routerRemoveScript(String name) =>
      const NativeResponse(ok: true, message: 'script removed', data: {});

  @override
  NativeResponse routerUpsertUserMode(Map<String, Object?> mode) =>
      const NativeResponse(ok: true, message: 'mode saved', data: {});

  @override
  NativeResponse routerRemoveUserMode(String name) =>
      const NativeResponse(ok: true, message: 'mode removed', data: {});
}

Map<String, Object?> _phaseFiveConfigMap() {
  return {
    'endpoints': [
      {
        'name': 'default',
        'host': '127.0.0.1',
        'port': 443,
        'proxy': {
          'type': 'vless',
          'uuid': '12345678-1234-1234-1234-123456789abc',
          'flow': '',
        },
        'transport': {'type': 'raw'},
        'outer-security': {'type': 'none'},
      },
      {
        'name': 'backup',
        'host': 'backup.example',
        'port': 8443,
        'proxy': {'type': 'trojan', 'password': 'secret'},
        'transport': {'type': 'raw'},
        'outer-security': {'type': 'tls', 'server-name': 'backup.example'},
      },
    ],
    'groups': [
      {
        'name': 'auto',
        'kind': 'select',
        'members': ['default', 'backup'],
        'selected': 'backup',
      },
    ],
    'scripts': [
      {
        'name': 'split',
        'rules': [
          {'kind': 'match', 'action': 'proxy', 'name': 'auto'},
        ],
      },
    ],
    'modes': [
      {'name': 'global', 'kind': 'global'},
      {'name': 'rule', 'kind': 'rule'},
      {'name': 'direct', 'kind': 'direct'},
      {'name': 'travel', 'kind': 'user', 'proxy': 'auto', 'script': 'split'},
    ],
    'active_mode': 'travel',
    'active': {'type': 'group', 'name': 'auto'},
    'dns': {'mode': 'system'},
    'local': {'host': '127.0.0.1', 'port': 1080},
  };
}
