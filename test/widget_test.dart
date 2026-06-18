import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/app.dart';
import 'package:wrongcl/autostart_manager.dart';
import 'package:wrongcl/desktop_shell_controller.dart';
import 'package:wrongcl/profile_store.dart';
import 'package:wrongcl/system_proxy_manager.dart';
import 'package:wrongcl/wrongcl_client.dart';

void main() {
  testWidgets('client renders import and profile workflow', (tester) async {
    final client = FakeWrongclClient();
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
    final profileStore = ProfileStore(
      file: File('${tempDir.path}/profiles.json'),
    );
    final autostartManager = AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
    );
    final systemProxyManager = SystemProxyManager(
      runner: (executable, arguments) async =>
          ProcessResult(0, 0, "'none'\n", ''),
      platform: SystemProxyPlatform.linux,
    );

    await tester.pumpWidget(
      WrongclApp(
        client: client,
        profileStore: profileStore,
        autostartManager: autostartManager,
        systemProxyManager: systemProxyManager,
      ),
    );
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.text('Wrongcl'), findsOneWidget);
    expect(find.text('Profiles'), findsOneWidget);
    expect(find.text('Endpoint'), findsOneWidget);
    expect(find.text('Connection Manager'), findsOneWidget);
    expect(find.text('wrongsv Import'), findsOneWidget);
    expect(find.text('Start proxy'), findsOneWidget);
    expect(find.text('Run probe'), findsOneWidget);
    expect(find.text('Inspect wrongsv'), findsOneWidget);
    expect(find.text('Adapt into form'), findsOneWidget);
    expect(find.text('New blank'), findsOneWidget);
    expect(find.text('Validate current'), findsOneWidget);
    expect(find.text('Enable system proxy'), findsOneWidget);
    final inspectButton = find.widgetWithText(
      OutlinedButton,
      'Inspect wrongsv',
    );
    final adaptButton = find.widgetWithText(FilledButton, 'Adapt into form');
    final saveButton = find.widgetWithText(FilledButton, 'Save current');

    await tester.enterText(
      find.byKey(const ValueKey('wrongsv-config-path')),
      '/tmp/server.toml',
    );
    await tester.pumpAndSettle();
    await tester.ensureVisible(inspectButton);
    await tester.pumpAndSettle();
    await tester.tap(inspectButton);
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pumpAndSettle();

    expect(client.inspectCount, 1);
    expect(find.textContaining('wrongsv capabilities inspected'), findsWidgets);

    await tester.ensureVisible(adaptButton);
    await tester.pumpAndSettle();
    await tester.tap(adaptButton);
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pumpAndSettle();

    expect(client.adaptCount, 1);
    expect(find.textContaining('wrongsv config adapted'), findsWidgets);

    await tester.enterText(
      find.byKey(const ValueKey('profile-name')),
      'saved profile',
    );
    await tester.pumpAndSettle();
    await tester.ensureVisible(saveButton);
    await tester.pumpAndSettle();
    await tester.tap(saveButton);
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pumpAndSettle();

    expect(find.text('saved profile'), findsOneWidget);
  });

  testWidgets('client controls start proxy and run probe', (tester) async {
    final client = FakeWrongclClient();
    final desktopShellController = FakeDesktopShellController();
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
    final profileStore = ProfileStore(
      file: File('${tempDir.path}/profiles.json'),
    );
    final autostartManager = AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
    );
    final systemProxyManager = SystemProxyManager(
      runner: (executable, arguments) async =>
          ProcessResult(0, 0, "'none'\n", ''),
      platform: SystemProxyPlatform.linux,
    );

    await tester.pumpWidget(
      WrongclApp(
        client: client,
        profileStore: profileStore,
        autostartManager: autostartManager,
        systemProxyManager: systemProxyManager,
        desktopShellController: desktopShellController,
      ),
    );
    await tester.pump(const Duration(milliseconds: 300));

    final startButton = find.widgetWithText(FilledButton, 'Start proxy');
    final probeButton = find.widgetWithText(FilledButton, 'Run probe');

    await tester.ensureVisible(startButton);
    await tester.pumpAndSettle();
    final start = tester.widget<FilledButton>(startButton);
    expect(start.onPressed, isNotNull);
    start.onPressed!();
    await tester.pumpAndSettle();

    expect(client.startCount, 1);
    expect(find.textContaining('local proxy started'), findsWidgets);
    expect(
      desktopShellController.syncedStates.any((state) => state.running),
      isTrue,
    );

    await tester.ensureVisible(probeButton);
    await tester.pumpAndSettle();
    final probe = tester.widget<FilledButton>(probeButton);
    expect(probe.onPressed, isNotNull);
    probe.onPressed!();
    await tester.pumpAndSettle();

    expect(client.probeCount, 1);
    expect(find.textContaining('probe succeeded'), findsWidgets);
  });

  testWidgets('health view tracks probe success and last error', (
    tester,
  ) async {
    final client = FlakyProbeWrongclClient();
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
    final profileStore = ProfileStore(
      file: File('${tempDir.path}/profiles.json'),
    );
    final autostartManager = AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
    );
    final systemProxyManager = SystemProxyManager(
      runner: (executable, arguments) async =>
          ProcessResult(0, 0, "'none'\n", ''),
      platform: SystemProxyPlatform.linux,
    );

    await tester.pumpWidget(
      WrongclApp(
        client: client,
        profileStore: profileStore,
        autostartManager: autostartManager,
        systemProxyManager: systemProxyManager,
      ),
    );
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.text('Health'), findsOneWidget);
    expect(find.text('No probe recorded'), findsOneWidget);
    expect(find.text('No error recorded'), findsOneWidget);

    final startButton = find.widgetWithText(FilledButton, 'Start proxy');
    await tester.ensureVisible(startButton);
    await tester.pumpAndSettle();
    tester.widget<FilledButton>(startButton).onPressed!();
    await tester.pumpAndSettle();

    final probeButton = find.widgetWithText(FilledButton, 'Run probe');
    await tester.ensureVisible(probeButton);
    await tester.pumpAndSettle();
    tester.widget<FilledButton>(probeButton).onPressed!();
    await tester.pumpAndSettle();

    expect(find.textContaining('4 bytes | pong'), findsOneWidget);
    expect(find.text('No error recorded'), findsOneWidget);

    tester.widget<FilledButton>(probeButton).onPressed!();
    await tester.pumpAndSettle();

    expect(find.textContaining('probe: upstream timed out'), findsOneWidget);
    expect(find.textContaining('4 bytes | pong'), findsOneWidget);
  });

  testWidgets('client config load and export workflow', (tester) async {
    final client = FakeWrongclClient();
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
    final profileStore = ProfileStore(
      file: File('${tempDir.path}/profiles.json'),
    );
    final autostartManager = AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
    );
    final systemProxyManager = SystemProxyManager(
      runner: (executable, arguments) async =>
          ProcessResult(0, 0, "'none'\n", ''),
      platform: SystemProxyPlatform.linux,
    );
    final configPath = File('${tempDir.path}/client.json');

    await tester.pumpWidget(
      WrongclApp(
        client: client,
        profileStore: profileStore,
        autostartManager: autostartManager,
        systemProxyManager: systemProxyManager,
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const ValueKey('client-config-path')),
      configPath.path,
    );
    await tester.pumpAndSettle();

    final exportButton = find.widgetWithText(
      OutlinedButton,
      'Export current JSON',
    );
    final export = tester.widget<OutlinedButton>(exportButton);
    expect(export.onPressed, isNotNull);
    export.onPressed!();
    await tester.pump(const Duration(milliseconds: 300));

    expect(configPath.existsSync(), isTrue);
    expect(configPath.readAsStringSync(), contains('"server"'));

    final tomlPath = File('${tempDir.path}/client.toml');
    await tester.enterText(
      find.byKey(const ValueKey('client-config-path')),
      tomlPath.path,
    );
    await tester.pumpAndSettle();

    final exportTomlButton = find.widgetWithText(
      OutlinedButton,
      'Export current TOML',
    );
    final exportToml = tester.widget<OutlinedButton>(exportTomlButton);
    expect(exportToml.onPressed, isNotNull);
    exportToml.onPressed!();
    await tester.pump(const Duration(milliseconds: 300));

    expect(tomlPath.existsSync(), isTrue);
    expect(tomlPath.readAsStringSync(), contains('[server]'));

    await tester.enterText(
      find.byKey(const ValueKey('client-config-path')),
      configPath.path,
    );
    await tester.pumpAndSettle();

    final loadButton = find.widgetWithText(
      OutlinedButton,
      'Load client config',
    );
    final load = tester.widget<OutlinedButton>(loadButton);
    expect(load.onPressed, isNotNull);
    load.onPressed!();
    await tester.pump(const Duration(milliseconds: 300));

    expect(client.loadConfigCount, 1);
    final serverPortField = tester.widget<TextField>(
      find.widgetWithText(TextField, 'Server port'),
    );
    final trojanPasswordField = tester.widget<TextField>(
      find.widgetWithText(TextField, 'Trojan password'),
    );
    expect(serverPortField.controller?.text, '9000');
    expect(trojanPasswordField.controller?.text, 'loaded-password');
  });

  testWidgets('partial wrongsv adapt shows missing field prompt', (
    tester,
  ) async {
    final client = PartialAdaptWrongclClient();
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
    final profileStore = ProfileStore(
      file: File('${tempDir.path}/profiles.json'),
    );
    final autostartManager = AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
    );
    final systemProxyManager = SystemProxyManager(
      runner: (executable, arguments) async =>
          ProcessResult(0, 0, "'none'\n", ''),
      platform: SystemProxyPlatform.linux,
    );

    await tester.pumpWidget(
      WrongclApp(
        client: client,
        profileStore: profileStore,
        autostartManager: autostartManager,
        systemProxyManager: systemProxyManager,
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const ValueKey('wrongsv-config-path')),
      '/tmp/reality.toml',
    );
    await tester.pumpAndSettle();

    final adaptButton = find.widgetWithText(FilledButton, 'Adapt into form');
    await tester.ensureVisible(adaptButton);
    await tester.pumpAndSettle();
    await tester.tap(adaptButton);
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pumpAndSettle();

    expect(client.adaptCount, 1);
    expect(find.textContaining('Adapted draft config'), findsOneWidget);
    expect(find.text('Fill required client-side fields'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('missing-reality.public-key')),
      findsOneWidget,
    );

    await tester.enterText(
      find.byKey(const ValueKey('profile-name')),
      'reality draft',
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Save current'));
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pumpAndSettle();

    await tester.pumpWidget(
      WrongclApp(client: client, profileStore: profileStore),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('reality draft'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Load selected'));
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pumpAndSettle();

    expect(find.text('Fill required client-side fields'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('missing-reality.public-key')),
      findsOneWidget,
    );
  });

  testWidgets('new blank resets profile draft fields', (tester) async {
    final client = FakeWrongclClient();
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
    final profileStore = ProfileStore(
      file: File('${tempDir.path}/profiles.json'),
    );
    final autostartManager = AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
    );
    final systemProxyManager = SystemProxyManager(
      runner: (executable, arguments) async =>
          ProcessResult(0, 0, "'none'\n", ''),
      platform: SystemProxyPlatform.linux,
    );

    await tester.pumpWidget(
      WrongclApp(
        client: client,
        profileStore: profileStore,
        autostartManager: autostartManager,
        systemProxyManager: systemProxyManager,
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const ValueKey('profile-name')),
      'temp profile',
    );
    await tester.enterText(
      find.byKey(const ValueKey('wrongsv-config-path')),
      '/tmp/server.toml',
    );
    await tester.pumpAndSettle();

    final newBlankButton = find.widgetWithText(OutlinedButton, 'New blank');
    await tester.ensureVisible(newBlankButton);
    await tester.pumpAndSettle();
    await tester.tap(newBlankButton);
    await tester.pumpAndSettle();

    final profileNameField = tester.widget<TextField>(
      find.byKey(const ValueKey('profile-name')),
    );
    final wrongsvPathField = tester.widget<TextField>(
      find.byKey(const ValueKey('wrongsv-config-path')),
    );
    expect(profileNameField.controller?.text, 'default');
    expect(wrongsvPathField.controller?.text, '');
  });

  testWidgets('deleting a saved profile requires confirmation', (tester) async {
    final client = FakeWrongclClient();
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
    final profileStore = MemoryProfileStore([
      SavedProfile(
        id: 'delete-me',
        name: 'delete me',
        config: const {
          'server': {
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
          'local': {'host': '127.0.0.1', 'port': 1080},
        },
        stackSummary: 'VLESS → raw → TCP',
        updatedAt: DateTime(2026, 6, 17, 12, 0),
      ),
    ]);
    final autostartManager = AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
    );
    final systemProxyManager = SystemProxyManager(
      runner: (executable, arguments) async =>
          ProcessResult(0, 0, "'none'\n", ''),
      platform: SystemProxyPlatform.linux,
    );

    await tester.pumpWidget(
      WrongclApp(
        client: client,
        profileStore: profileStore,
        autostartManager: autostartManager,
        systemProxyManager: systemProxyManager,
      ),
    );
    await tester.pumpAndSettle();

    final profileTile = find.widgetWithText(ListTile, 'delete me');
    expect(profileTile, findsOneWidget);
    final selectedProfile = tester.widget<ListTile>(profileTile);
    expect(selectedProfile.onTap, isNotNull);
    selectedProfile.onTap!();
    await tester.pumpAndSettle();

    final deleteButton = find.widgetWithText(OutlinedButton, 'Delete selected');
    final delete = tester.widget<OutlinedButton>(deleteButton);
    expect(delete.onPressed, isNotNull);
    delete.onPressed!();
    await tester.pumpAndSettle();

    expect(find.text('Delete saved profile?'), findsOneWidget);
    expect(find.textContaining('Delete "delete me"'), findsOneWidget);

    await tester.tap(find.text('Cancel'));
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pumpAndSettle();
    expect(find.text('Delete saved profile?'), findsNothing);
    expect(profileTile, findsOneWidget);
    expect(find.text('Deleted profile delete me'), findsNothing);

    final deleteAgain = tester.widget<OutlinedButton>(deleteButton);
    expect(deleteAgain.onPressed, isNotNull);
    deleteAgain.onPressed!();
    await tester.pumpAndSettle();
    await tester.tap(find.text('Delete profile'));
    await tester.pump(const Duration(milliseconds: 100));
    await tester.pumpAndSettle();
    expect(find.text('Delete saved profile?'), findsNothing);
    expect(find.text('Deleted profile delete me'), findsOneWidget);
    expect(find.text('No saved profiles'), findsOneWidget);
    expect(profileTile, findsNothing);
  });

  test('profile store persists saved metadata', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final store = ProfileStore(file: File('${tempDir.path}/profiles.json'));

    final now = DateTime(2026, 6, 17, 12, 0);
    final profile = SavedProfile(
      id: 'one',
      name: 'saved profile',
      config: const {
        'server': {'host': '127.0.0.1'},
      },
      stackSummary: 'VLESS → raw → TCP',
      updatedAt: now,
      source: 'wrongsv',
      sourcePath: '/tmp/server.toml',
      activeProfile: 'reality',
      supportState: 'partial',
      supportReason: 'missing fields: reality.public-key',
      importReport: const {
        'active_profile': 'reality',
        'active_support': 'partial',
        'missing_fields': [
          {'field': 'reality.public-key', 'reason': 'required'},
        ],
        'profiles': [],
      },
    );

    final duplicate = profile.copyWith(
      id: 'two',
      name: 'saved profile copy',
      updatedAt: now.add(const Duration(minutes: 1)),
    );

    await store.saveProfiles([profile, duplicate]);
    final loaded = await store.loadProfiles();

    expect(loaded.length, 2);
    expect(loaded.first.name, 'saved profile copy');
    expect(loaded.first.sourcePath, '/tmp/server.toml');
    expect(loaded.first.importReport?['active_profile'], 'reality');
    expect(loaded.last.name, 'saved profile');
  });

  test('autostart manager toggles desktop entry file', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
    final autostartFile = File('${tempDir.path}/wrongcl.desktop');
    final autostartManager = AutostartManager(
      file: autostartFile,
      executablePath: '/tmp/wrongcl',
      platform: AutostartPlatform.linux,
    );

    final initial = await autostartManager.loadStatus();
    expect(initial.supported, isTrue);
    expect(initial.enabled, isFalse);

    await autostartManager.enable();
    expect(autostartFile.existsSync(), isTrue);
    expect(autostartFile.readAsStringSync(), contains('Exec=/tmp/wrongcl'));

    final enabled = await autostartManager.loadStatus();
    expect(enabled.enabled, isTrue);

    await autostartManager.disable();
    expect(autostartFile.existsSync(), isFalse);
  });

  test('autostart manager writes macOS launch agent plist', () async {
    final tempDir = Directory.systemTemp.createTempSync(
      'wrongcl-autostart-test',
    );
    final autostartFile = File('${tempDir.path}/us.irrit.wrongcl.plist');
    final manager = AutostartManager(
      file: autostartFile,
      executablePath: '/Applications/wrongcl.app/Contents/MacOS/wrongcl',
      platform: AutostartPlatform.macos,
    );

    await manager.enable();
    final text = autostartFile.readAsStringSync();
    expect(text, contains('<string>us.irrit.wrongcl</string>'));
    expect(
      text,
      contains(
        '<string>/Applications/wrongcl.app/Contents/MacOS/wrongcl</string>',
      ),
    );
  });

  test('autostart manager writes Windows startup script', () async {
    final tempDir = Directory.systemTemp.createTempSync(
      'wrongcl-autostart-test',
    );
    final autostartFile = File('${tempDir.path}/wrongcl.cmd');
    final manager = AutostartManager(
      file: autostartFile,
      executablePath: r'C:\Program Files\wrongcl\wrongcl.exe',
      platform: AutostartPlatform.windows,
    );

    await manager.enable();
    final text = autostartFile.readAsStringSync();
    expect(text, contains('@echo off'));
    expect(text, contains(r'start "" "C:\Program Files\wrongcl\wrongcl.exe"'));
  });

  test(
    'system proxy manager enables and disables GNOME SOCKS proxy commands',
    () async {
      final calls = <String>[];
      final manager = SystemProxyManager(
        runner: (executable, arguments) async {
          calls.add('$executable ${arguments.join(' ')}');
          if (arguments.length >= 3 &&
              arguments[0] == 'get' &&
              arguments[1] == 'org.gnome.system.proxy') {
            return ProcessResult(0, 0, "'none'\n", '');
          }
          return ProcessResult(0, 0, '', '');
        },
        platform: SystemProxyPlatform.linux,
      );

      final initial = await manager.loadStatus();
      expect(initial.supported, isTrue);
      expect(initial.enabled, isFalse);

      await manager.enableSocks('127.0.0.1', 1080);
      await manager.disable();

      expect(
        calls,
        contains('gsettings set org.gnome.system.proxy mode manual'),
      );
      expect(
        calls,
        contains('gsettings set org.gnome.system.proxy.socks host 127.0.0.1'),
      );
      expect(
        calls,
        contains('gsettings set org.gnome.system.proxy.socks port 1080'),
      );
      expect(calls, contains('gsettings set org.gnome.system.proxy mode none'));
    },
  );
}

class FakeWrongclClient implements WrongclClient {
  int inspectCount = 0;
  int adaptCount = 0;
  int loadConfigCount = 0;
  int startCount = 0;
  int probeCount = 0;

  @override
  NativeResponse version() {
    return const NativeResponse(
      ok: true,
      message: 'native ready',
      data: {
        'version': 'test',
        'proxies': [
          'vless',
          'hysteria2',
          'tuic',
          'trojan',
          'mixed',
          'shadowsocks',
        ],
        'transports': ['raw', 'kcp', 'quic', 'websocket', 'httpupgrade'],
        'outer_security': ['none', 'tls', 'reality', 'anytls', 'shadowtls'],
      },
    );
  }

  @override
  NativeResponse startProxy(ClientConfigInput config) {
    startCount += 1;
    return NativeResponse(
      ok: true,
      message: 'local proxy started',
      data: {
        'stack': 'VLESS → raw → TCP',
        'proxy': {
          'running': true,
          'local_host': config.localHost,
          'local_port': config.localPort,
          'active_connections': 0,
          'total_connections': 0,
          'failed_connections': 0,
          'bytes_uploaded': 0,
          'bytes_downloaded': 0,
        },
      },
    );
  }

  @override
  NativeResponse stopProxy() {
    return const NativeResponse(
      ok: true,
      message: 'local proxy stopped',
      data: {
        'running': false,
        'active_connections': 0,
        'total_connections': 0,
        'failed_connections': 0,
        'bytes_uploaded': 0,
        'bytes_downloaded': 0,
      },
    );
  }

  @override
  NativeResponse status() {
    return const NativeResponse(
      ok: true,
      message: 'local proxy is stopped',
      data: {
        'running': false,
        'active_connections': 0,
        'total_connections': 0,
        'failed_connections': 0,
        'bytes_uploaded': 0,
        'bytes_downloaded': 0,
      },
    );
  }

  @override
  NativeResponse probe(ProbeRequest request) {
    probeCount += 1;
    return const NativeResponse(
      ok: true,
      message: 'probe succeeded',
      data: {
        'stack': 'VLESS → raw → TCP',
        'probe': {'bytes_read': 4, 'preview': 'pong'},
      },
    );
  }

  @override
  NativeResponse stackSummary(ClientConfigInput config) {
    return const NativeResponse(
      ok: true,
      message: 'stack resolved',
      data: {
        'stack': 'VLESS → raw → TCP',
        'proxy': 'vless',
        'transport': 'raw',
        'outer_security': 'none',
      },
    );
  }

  @override
  NativeResponse validateConfig(ClientConfigInput config) {
    return const NativeResponse(
      ok: true,
      message: 'client config validated',
      data: {
        'stack': 'VLESS → raw → TCP',
        'proxy': 'vless',
        'transport': 'raw',
        'outer_security': 'none',
      },
    );
  }

  @override
  NativeResponse loadClientConfigFile(String path) {
    loadConfigCount += 1;
    return const NativeResponse(
      ok: true,
      message: 'client config loaded',
      data: {
        'config': {
          'server': {
            'host': '127.0.0.1',
            'port': 9000,
            'proxy': {'type': 'trojan', 'password': 'loaded-password'},
            'transport': {'type': 'raw'},
            'outer-security': {
              'type': 'tls',
              'server-name': 'loaded.example',
              'insecure-skip-verify': false,
              'alpn': ['h2'],
            },
          },
          'local': {'host': '127.0.0.1', 'port': 1090},
        },
        'stack': 'Trojan → raw → TLS → TCP',
        'proxy': 'trojan',
        'transport': 'raw',
        'outer_security': 'tls',
      },
    );
  }

  @override
  NativeResponse exportConfigToml(ClientConfigInput config) {
    return const NativeResponse(
      ok: true,
      message: 'client config exported as TOML',
      data: {'toml': '[server]\nhost = "127.0.0.1"\nport = 443\n'},
    );
  }

  @override
  NativeResponse inspectWrongsvConfig(String path) {
    inspectCount += 1;
    return const NativeResponse(
      ok: true,
      message: 'wrongsv capabilities inspected',
      data: {
        'active_profile': 'raw',
        'listen': '0.0.0.0:443',
        'listen_port': 443,
        'payload_networks': ['tcp', 'udp'],
        'base_carriers': ['tcp'],
        'active_support': 'supported',
        'active_reason': 'test fixture',
        'missing_fields': [],
        'profiles': [],
      },
    );
  }

  @override
  NativeResponse adaptWrongsvConfig(WrongsvAdaptRequest request) {
    adaptCount += 1;
    return NativeResponse(
      ok: true,
      message: 'wrongsv config adapted',
      data: {
        'report': {
          'active_profile': 'raw',
          'listen': '0.0.0.0:443',
          'listen_port': 443,
          'payload_networks': ['tcp', 'udp'],
          'base_carriers': ['tcp'],
          'active_support': 'supported',
          'active_reason': 'test fixture',
          'missing_fields': [],
          'profiles': [],
        },
        'config': {
          'server': {
            'host': request.serverHost,
            'port': 443,
            'proxy': {
              'type': 'vless',
              'uuid': '12345678-1234-1234-1234-123456789abc',
              'flow': '',
            },
            'transport': {'type': 'raw'},
            'outer-security': {'type': 'none'},
          },
          'local': {'host': request.listenHost, 'port': request.listenPort},
        },
        'stack_summary': 'VLESS → raw → TCP',
      },
    );
  }
}

class FakeDesktopShellController implements DesktopShellController {
  DesktopShellActions? actions;
  DesktopShellState? attachedState;
  final List<DesktopShellState> syncedStates = [];

  @override
  Future<void> attach({
    required DesktopShellActions actions,
    required DesktopShellState initialState,
  }) async {
    this.actions = actions;
    attachedState = initialState;
  }

  @override
  Future<void> bootstrap() async {}

  @override
  Future<void> dispose() async {}

  @override
  Future<void> sync(DesktopShellState state) async {
    syncedStates.add(state);
  }
}

class PartialAdaptWrongclClient extends FakeWrongclClient {
  @override
  NativeResponse inspectWrongsvConfig(String path) {
    inspectCount += 1;
    return const NativeResponse(
      ok: true,
      message: 'wrongsv capabilities inspected',
      data: {
        'active_profile': 'reality',
        'listen': '0.0.0.0:443',
        'listen_port': 443,
        'payload_networks': ['tcp', 'udp'],
        'base_carriers': ['tcp'],
        'active_support': 'partial',
        'active_reason': 'missing fields: reality.public-key',
        'missing_fields': [
          {
            'field': 'reality.public-key',
            'reason':
                'wrongsv server configs keep the REALITY private key; wrongcl needs the matching client public-key supplied separately',
          },
        ],
        'profiles': [],
      },
    );
  }

  @override
  NativeResponse adaptWrongsvConfig(WrongsvAdaptRequest request) {
    adaptCount += 1;
    return NativeResponse(
      ok: true,
      message: 'wrongsv config adapted',
      data: {
        'report': {
          'active_profile': 'reality',
          'listen': '0.0.0.0:443',
          'listen_port': 443,
          'payload_networks': ['tcp', 'udp'],
          'base_carriers': ['tcp'],
          'active_support': 'partial',
          'active_reason': 'missing fields: reality.public-key',
          'missing_fields': [
            {
              'field': 'reality.public-key',
              'reason':
                  'wrongsv server configs keep the REALITY private key; wrongcl needs the matching client public-key supplied separately',
            },
          ],
          'profiles': [],
        },
        'config': null,
        'draft_config': {
          'server': {
            'host': request.serverHost,
            'port': 443,
            'proxy': {
              'type': 'vless',
              'uuid': '12345678-1234-1234-1234-123456789abc',
              'flow': '',
            },
            'transport': {'type': 'raw'},
            'outer-security': {
              'type': 'reality',
              'server-name': 'www.microsoft.com',
              'public-key': '',
              'short-id': 'aaaaaaaa',
              'raw-pubkey': '',
            },
          },
          'local': {'host': request.listenHost, 'port': request.listenPort},
        },
        'stack_summary': 'VLESS → raw → REALITY → TCP',
      },
    );
  }
}

class FlakyProbeWrongclClient extends FakeWrongclClient {
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
      message: 'upstream timed out',
      data: {},
    );
  }
}

class MemoryProfileStore extends ProfileStore {
  MemoryProfileStore([List<SavedProfile> initialProfiles = const []])
    : _profiles = [...initialProfiles],
      super(
        file: File('${Directory.systemTemp.path}/wrongcl-memory-store.json'),
      );

  List<SavedProfile> _profiles;

  @override
  Future<List<SavedProfile>> loadProfiles() async {
    final profiles = [..._profiles];
    profiles.sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    return profiles;
  }

  @override
  Future<void> saveProfiles(List<SavedProfile> profiles) async {
    _profiles = [...profiles];
  }
}
