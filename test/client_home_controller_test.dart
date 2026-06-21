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

  test('controller records probe success and failure events truthfully', () async {
    final client = _SignalTestClient();
    final controller = _buildController(client);

    await controller.runProbe();
    await controller.runProbe();

    final events = controller.dashboardSnapshot.signalSnapshot.recentProbeOutcomes;
    expect(events.length, 2);
    expect(events.first.success, isTrue);
    expect(events.last.success, isFalse);
  });
}

ClientHomeController _buildController(WrongclClient client) {
  final tempDir = Directory.systemTemp.createTempSync('wrongcl-controller-test');
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

  @override
  NativeResponse version() => const NativeResponse(
        ok: true,
        message: 'ready',
        data: {'version': 'test', 'proxies': [], 'transports': [], 'outer_security': []},
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
    return const NativeResponse(
      ok: false,
      message: 'probe failed',
      data: {},
    );
  }

  @override
  NativeResponse stackSummary(ClientConfigInput config) => const NativeResponse(
        ok: true,
        message: 'stack',
        data: {'stack': 'VLESS → raw → TCP'},
      );

  @override
  NativeResponse validateConfig(ClientConfigInput config) => const NativeResponse(
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
  NativeResponse inspectWrongsvConfig(String path) => throw UnimplementedError();

  @override
  NativeResponse adaptWrongsvConfig(WrongsvAdaptRequest request) =>
      throw UnimplementedError();
}
