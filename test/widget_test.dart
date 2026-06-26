import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/app.dart';
import 'package:wrongcl/app_settings_store.dart';
import 'package:wrongcl/autostart_manager.dart';
import 'package:wrongcl/client_home_controller.dart';
import 'package:wrongcl/desktop_shell_controller.dart';
import 'package:wrongcl/profile_store.dart';
import 'package:wrongcl/system_proxy_manager.dart';
import 'package:wrongcl/theme/wrongcl_colors.dart';
import 'package:wrongcl/widgets/mode_strip.dart';
import 'package:wrongcl/wrongcl_client.dart';

class _Harness {
  _Harness({
    required this.client,
    required this.profileStore,
    required this.autostartManager,
    required this.systemProxyManager,
    required this.desktopShellController,
    required this.appSettingsStore,
  });

  final WrongclClient client;
  final ProfileStore profileStore;
  final AutostartManager autostartManager;
  final SystemProxyManager systemProxyManager;
  final FakeDesktopShellController desktopShellController;
  final AppSettingsStore appSettingsStore;

  Widget build() {
    return WrongclApp(
      client: client,
      profileStore: profileStore,
      autostartManager: autostartManager,
      systemProxyManager: systemProxyManager,
      desktopShellController: desktopShellController,
      appSettingsStore: appSettingsStore,
    );
  }
}

_Harness _makeHarness({
  WrongclClient? client,
  SystemProxyPlatform platform = SystemProxyPlatform.linux,
}) {
  final tempDir = Directory.systemTemp.createTempSync('wrongcl-widget-test');
  return _Harness(
    client: client ?? FakeWrongclClient(),
    profileStore: ProfileStore(file: File('${tempDir.path}/profiles.json')),
    autostartManager: AutostartManager(
      file: File('${tempDir.path}/wrongcl.desktop'),
      executablePath: '/tmp/wrongcl',
      platform: AutostartPlatform.linux,
    ),
    systemProxyManager: SystemProxyManager(
      runner: (executable, arguments) async =>
          ProcessResult(0, 0, "'none'\n", ''),
      platform: platform,
    ),
    desktopShellController: FakeDesktopShellController(),
    appSettingsStore: AppSettingsStore(
      file: File('${tempDir.path}/app_settings.json'),
    ),
  );
}

Future<void> _pumpReady(WidgetTester tester, _Harness harness) async {
  await tester.pumpWidget(harness.build());
  await tester.pump(const Duration(milliseconds: 300));
  await tester.pumpAndSettle();
}

Future<void> _tapEntryChip(WidgetTester tester, String label) async {
  final chip = find.widgetWithText(InkWell, label).first;
  await tester.ensureVisible(chip);
  await tester.tap(chip);
  await tester.pumpAndSettle();
}

Future<void> _closeSubpage(WidgetTester tester) async {
  await tester.tap(find.byTooltip('Close'));
  await tester.pumpAndSettle();
}

int _modeStripCellCount(WidgetTester tester) {
  return tester
      .widgetList<Expanded>(
        find.descendant(
          of: find.byType(ModeStrip),
          matching: find.byType(Expanded),
        ),
      )
      .length;
}

Future<void> _addUserMode(WidgetTester tester, String name) async {
  await tester.tap(find.text('Add'));
  await tester.pumpAndSettle();

  await tester.enterText(find.byType(TextFormField).first, name);
  await tester.tap(find.byType(DropdownButtonFormField<String>).first);
  await tester.pumpAndSettle();
  await tester.tap(find.text('default').last);
  await tester.pumpAndSettle();
  await tester.tap(find.widgetWithText(FilledButton, 'Save'));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('main view renders mode strip with Global/Rule/Direct', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    expect(find.text('Global'), findsOneWidget);
    expect(find.text('Rule'), findsOneWidget);
    expect(find.text('Direct'), findsOneWidget);
    expect(find.text('Add'), findsOneWidget);
    expect(_modeStripCellCount(tester), 7);
  });

  testWidgets('main view fits the 768x552 dashboard target without scroll', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(768, 552));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    expect(tester.takeException(), isNull);
    expect(find.byType(Scrollable), findsNothing);
    expect(tester.getTopLeft(find.text('Global')).dy, greaterThanOrEqualTo(0));
    expect(
      tester.getBottomLeft(find.text('Advanced')).dy,
      lessThanOrEqualTo(552),
    );
  });

  testWidgets('main view renders SysProxy/Runtime/TUN control column', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    expect(find.text('SysProxy'), findsOneWidget);
    expect(find.text('Runtime'), findsOneWidget);
    expect(find.text('TUN'), findsOneWidget);
    expect(find.textContaining('TUN driver lands in Phase 7'), findsOneWidget);
    expect(find.text('Stopped'), findsOneWidget);
  });

  testWidgets('runtime Start tap drives client.startProxy', (tester) async {
    final client = FakeWrongclClient();
    final harness = _makeHarness(client: client);
    await _pumpReady(tester, harness);

    await tester.tap(find.byTooltip('Start'));
    await tester.pumpAndSettle();

    expect(client.startCount, 1);
    expect(find.byTooltip('Stop'), findsOneWidget);
    expect(find.text('Running'), findsOneWidget);
    expect(
      harness.desktopShellController.syncedStates.any((s) => s.running),
      isTrue,
    );

    await tester.tap(find.byTooltip('Stop'));
    await tester.pumpAndSettle();
    expect(find.text('Stopped'), findsOneWidget);
  });

  testWidgets(
    'SysProxy switch is disabled with reason on unsupported platform',
    (tester) async {
      final harness = _makeHarness(platform: SystemProxyPlatform.unsupported);
      await _pumpReady(tester, harness);

      expect(
        find.textContaining('not implemented for this platform'),
        findsWidgets,
      );
    },
  );

  test('system proxy manager reports planned status on macOS', () async {
    final manager = SystemProxyManager(
      runner: (executable, arguments) async => ProcessResult(0, 0, '', ''),
      platform: SystemProxyPlatform.macos,
    );

    final status = await manager.loadStatus();
    expect(status.supported, isFalse);
    expect(status.enabled, isFalse);
    expect(status.mode, 'planned');
    expect(status.message, contains('planned but not implemented'));
  });

  final subpages = <(String, String)>[
    ('Profiles', 'Current draft'),
    ('Proxies', 'Start the proxy to inspect endpoints and groups.'),
    ('Connections', 'No active connections.'),
    ('Requests', 'No captured requests yet'),
    ('Logs', 'No log entries captured yet.'),
    ('Basic', 'Enable autostart'),
    ('Network', 'Local proxy listen address'),
    ('DNS', 'Resolver backend'),
    ('Advanced', 'Refresh status'),
  ];

  for (final (label, marker) in subpages) {
    testWidgets('entry chip "$label" opens subpage and Close returns home', (
      tester,
    ) async {
      final harness = _makeHarness();
      await _pumpReady(tester, harness);

      await _tapEntryChip(tester, label);
      expect(find.textContaining(marker), findsWidgets);
      expect(find.byTooltip('Close'), findsOneWidget);

      await _closeSubpage(tester);
      expect(find.byTooltip('Close'), findsNothing);
      expect(find.text('Runtime'), findsOneWidget);
    });
  }

  testWidgets('profiles subpage exposes Save current and New buttons', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'Profiles');

    final saveButton = find.widgetWithText(FilledButton, 'Save current');
    final newButton = find.widgetWithText(OutlinedButton, 'New');
    expect(saveButton, findsOneWidget);
    expect(newButton, findsOneWidget);
    expect(tester.widget<FilledButton>(saveButton).onPressed, isNotNull);
    expect(tester.widget<OutlinedButton>(newButton).onPressed, isNotNull);
  });

  testWidgets('profiles subpage can inspect and adapt a wrongsv config', (
    tester,
  ) async {
    final client = FakeWrongclClient();
    final harness = _makeHarness(client: client);
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'Profiles');
    await tester.enterText(
      find.widgetWithText(TextField, 'wrongsv config path'),
      '/tmp/server.toml',
    );
    final inspectButton = find.widgetWithText(
      OutlinedButton,
      'Inspect wrongsv',
    );
    await tester.dragUntilVisible(
      inspectButton,
      find.byType(Scrollable).last,
      const Offset(0, -120),
    );
    await tester.pumpAndSettle();
    await tester.tap(inspectButton);
    await tester.pumpAndSettle();
    final adaptButton = find.widgetWithText(FilledButton, 'Adapt wrongsv');
    await tester.dragUntilVisible(
      adaptButton,
      find.byType(Scrollable).last,
      const Offset(0, -120),
    );
    await tester.pumpAndSettle();
    await tester.tap(adaptButton);
    await tester.pumpAndSettle();

    expect(client.inspectCount, 1);
    expect(client.adaptCount, 1);
    expect(find.textContaining('raw is supported'), findsWidgets);
  });

  testWidgets(
    'profiles subpage shows missing-field prompts for partial wrongsv import',
    (tester) async {
      final client = PartialWrongsvClient();
      final harness = _makeHarness(client: client);
      await _pumpReady(tester, harness);

      await _tapEntryChip(tester, 'Profiles');
      await tester.enterText(
        find.widgetWithText(TextField, 'wrongsv config path'),
        '/tmp/server.toml',
      );
      await tester.tap(find.widgetWithText(OutlinedButton, 'Inspect wrongsv'));
      await tester.pumpAndSettle();

      expect(find.text('REALITY public-key (required)'), findsOneWidget);
    },
  );

  testWidgets('basic theme selector updates MaterialApp theme mode', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'Basic');
    final modeDropdown = find.byType(DropdownButtonFormField<ThemeMode>).first;
    await tester.dragUntilVisible(
      modeDropdown,
      find.byType(Scrollable).last,
      const Offset(0, -120),
    );
    await tester.pumpAndSettle();
    await tester.tap(modeDropdown);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Dark').last);
    await tester.pumpAndSettle();

    expect(
      tester.widget<MaterialApp>(find.byType(MaterialApp)).themeMode,
      ThemeMode.dark,
    );
  });

  testWidgets('basic language selector updates MaterialApp locale', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'Basic');
    await tester.tap(find.byType(DropdownButtonFormField<String>).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('简体中文').last);
    await tester.pumpAndSettle();

    expect(
      tester.widget<MaterialApp>(find.byType(MaterialApp)).locale,
      const Locale('zh', 'CN'),
    );
  });

  final additionalLanguages = <(String, String, Locale)>[
    ('Spanish', 'Español', Locale('es')),
    ('Arabic', 'العربية', Locale('ar')),
    ('French', 'Français', Locale('fr')),
  ];

  for (final (name, nativeLabel, expected) in additionalLanguages) {
    testWidgets(
      'basic language selector switches MaterialApp locale to $name',
      (tester) async {
        final harness = _makeHarness();
        await _pumpReady(tester, harness);

        await _tapEntryChip(tester, 'Basic');
        await tester.tap(find.byType(DropdownButtonFormField<String>).first);
        await tester.pumpAndSettle();
        await tester.tap(find.text(nativeLabel).last);
        await tester.pumpAndSettle();

        expect(
          tester.widget<MaterialApp>(find.byType(MaterialApp)).locale,
          expected,
        );
      },
    );
  }

  testWidgets('selecting Arabic flips text direction to RTL', (tester) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'Basic');
    await tester.tap(find.byType(DropdownButtonFormField<String>).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('العربية').last);
    await tester.pumpAndSettle();

    final scaffold = find.byType(Scaffold).first;
    expect(Directionality.of(tester.element(scaffold)), TextDirection.rtl);
  });

  testWidgets('language picker stays operable while controller is busy', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);
    await _tapEntryChip(tester, 'Basic');

    final controller = tester
        .widgetList<AnimatedBuilder>(find.byType(AnimatedBuilder))
        .map((w) => w.animation)
        .whereType<ClientHomeController>()
        .first;

    final blocker = Completer<void>();
    unawaited(controller.runTask('test-block', () => blocker.future));
    await tester.pump();
    expect(controller.busy, isTrue);

    await tester.tap(find.byType(DropdownButtonFormField<String>).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Français').last);
    await tester.pumpAndSettle();

    expect(
      tester.widget<MaterialApp>(find.byType(MaterialApp)).locale,
      const Locale('fr'),
    );
    expect(controller.busy, isTrue);

    blocker.complete();
    await tester.pumpAndSettle();
  });

  testWidgets('all five supported locales are declared on MaterialApp', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    final supported = tester
        .widget<MaterialApp>(find.byType(MaterialApp))
        .supportedLocales
        .toList(growable: false);
    expect(supported, contains(const Locale('en')));
    expect(supported, contains(const Locale('zh', 'CN')));
    expect(supported, contains(const Locale('es')));
    expect(supported, contains(const Locale('ar')));
    expect(supported, contains(const Locale('fr')));
  });

  testWidgets('basic palette selector swaps the active WrongclColors palette', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'Basic');

    final dropdown = tester
        .widget<DropdownButtonFormField<WrongclThemeVariant>>(
          find.byType(DropdownButtonFormField<WrongclThemeVariant>),
        );
    expect(dropdown.initialValue, WrongclThemeVariant.wrongcl);
    expect(dropdown.onChanged, isNotNull);

    await tester.runAsync(() async {
      dropdown.onChanged!(WrongclThemeVariant.nord);
      await Future<void>.delayed(const Duration(milliseconds: 200));
    });
    final stored = await tester.runAsync(() => harness.appSettingsStore.load());
    expect(stored!.themeVariant, WrongclThemeVariant.nord);
  });

  testWidgets('advanced subpage validate config calls the native client', (
    tester,
  ) async {
    final client = FakeWrongclClient();
    final harness = _makeHarness(client: client);
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'Advanced');

    final validateButton = find.widgetWithText(
      OutlinedButton,
      'Validate config',
    );
    expect(validateButton, findsOneWidget);
    await tester.tap(validateButton);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));

    expect(client.validateCount, 1);
  });

  testWidgets('advanced raw config editor can load the current draft', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'Advanced');
    final loadButton = find.widgetWithText(
      OutlinedButton,
      'Load current draft',
    );
    await tester.dragUntilVisible(
      loadButton,
      find.byType(Scrollable).last,
      const Offset(0, -120),
    );
    await tester.pumpAndSettle();
    tester.widget<OutlinedButton>(loadButton).onPressed!();
    await tester.pumpAndSettle();

    expect(find.textContaining('"endpoints"'), findsWidgets);
  });

  testWidgets('add-mode chip opens the user-mode form', (tester) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await tester.tap(find.text('Add'));
    await tester.pumpAndSettle();

    expect(find.text('New user mode'), findsOneWidget);
    expect(find.widgetWithText(TextFormField, ''), findsWidgets);

    await tester.tap(find.byTooltip('Close'));
    await tester.pumpAndSettle();
    expect(find.text('New user mode'), findsNothing);
  });

  testWidgets('add-mode dialog saves a user mode while proxy is stopped', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await tester.tap(find.text('Add'));
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextFormField).first, 'office');
    await tester.tap(find.byType(DropdownButtonFormField<String>).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('default').last);
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Save'));
    await tester.pumpAndSettle();

    expect(find.text('New user mode'), findsNothing);
    expect(find.text('Office'), findsOneWidget);
  });

  testWidgets('Add disappears after the sixth mode slot is occupied', (
    tester,
  ) async {
    final harness = _makeHarness();
    await _pumpReady(tester, harness);

    await _addUserMode(tester, 'office');
    await _addUserMode(tester, 'home');
    await _addUserMode(tester, 'travel');

    expect(find.text('Office'), findsOneWidget);
    expect(find.text('Home'), findsOneWidget);
    expect(find.text('Travel'), findsOneWidget);
    expect(find.text('Add'), findsNothing);
    expect(_modeStripCellCount(tester), 7);
  });

  testWidgets('mode strip can switch active mode before runtime starts', (
    tester,
  ) async {
    final client = FakeWrongclClient();
    final harness = _makeHarness(client: client);
    await _pumpReady(tester, harness);

    await tester.tap(find.text('Rule'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Start'));
    await tester.pumpAndSettle();

    expect(client.lastStartConfig, isNotNull);
    expect(client.lastStartConfig!.activeMode, 'rule');
  });

  testWidgets('DNS settings page applies DoH settings to the next start', (
    tester,
  ) async {
    final client = FakeWrongclClient();
    final harness = _makeHarness(client: client);
    await _pumpReady(tester, harness);

    await _tapEntryChip(tester, 'DNS');
    await tester.tap(
      find.byType(DropdownButtonFormField<DnsBackendKind>).first,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('DoH').last);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byType(TextField).first,
      'https://1.1.1.1/dns-query',
    );
    await tester.tap(find.widgetWithText(FilledButton, 'Apply DNS settings'));
    await tester.pumpAndSettle();
    await _closeSubpage(tester);

    await tester.tap(find.byTooltip('Start'));
    await tester.pumpAndSettle();

    expect(client.lastStartConfig, isNotNull);
    expect(client.lastStartConfig!.dns, {
      'backend': {'kind': 'doh', 'url': 'https://1.1.1.1/dns-query'},
    });
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

  test('system proxy manager enables and disables Windows SOCKS proxy', () async {
    final calls = <String>[];
    final manager = SystemProxyManager(
      runner: (executable, arguments) async {
        calls.add('$executable ${arguments.join(' ')}');
        if (executable == 'reg' &&
            arguments.length >= 4 &&
            arguments[0] == 'query' &&
            arguments[3] == 'ProxyEnable') {
          return ProcessResult(
            0,
            0,
            'HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings\n'
                '    ProxyEnable    REG_DWORD    0x1\n',
            '',
          );
        }
        if (executable == 'reg' &&
            arguments.length >= 4 &&
            arguments[0] == 'query' &&
            arguments[3] == 'ProxyServer') {
          return ProcessResult(
            0,
            0,
            'HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings\n'
                '    ProxyServer    REG_SZ    socks=127.0.0.1:1080\n',
            '',
          );
        }
        return ProcessResult(0, 0, '', '');
      },
      platform: SystemProxyPlatform.windows,
    );

    final initial = await manager.loadStatus();
    expect(initial.supported, isTrue);
    expect(initial.enabled, isTrue);
    expect(initial.message, contains('socks=127.0.0.1:1080'));

    await manager.enableSocks('127.0.0.1', 1080);
    await manager.disable();

    expect(
      calls,
      contains(
        'reg add HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings /v ProxyServer /t REG_SZ /d socks=127.0.0.1:1080 /f',
      ),
    );
    expect(
      calls,
      contains(
        'reg add HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings /v ProxyEnable /t REG_DWORD /d 1 /f',
      ),
    );
    expect(
      calls,
      contains(
        'reg add HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings /v ProxyEnable /t REG_DWORD /d 0 /f',
      ),
    );
    expect(
      calls.where(
        (entry) =>
            entry.startsWith('powershell -NoProfile -NonInteractive -Command '),
      ),
      isNotEmpty,
    );
  });
}

class FakeWrongclClient implements WrongclClient {
  int inspectCount = 0;
  int adaptCount = 0;
  int loadConfigCount = 0;
  int startCount = 0;
  int probeCount = 0;
  int validateCount = 0;
  int connectionsListCount = 0;
  int closeConnectionCount = 0;
  int closeMatchingCount = 0;
  int logsSinceCount = 0;
  List<Map<String, Object?>> liveConnections = const [];
  List<Map<String, Object?>> pendingLogs = const [];
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
  NativeResponse version() {
    return const NativeResponse(
      ok: true,
      message: 'native ready',
      data: {
        'version': 'test',
        'proxies': [
          'vless',
          'naive',
          'hysteria2',
          'tuic',
          'trojan',
          'mixed',
          'shadowsocks',
          'wireguard',
        ],
        'transports': [
          'raw',
          'kcp',
          'meek',
          'gdocsviewer',
          'quic',
          'webtransport',
          'websocket',
          'httpupgrade',
          'xhttp',
          'grpc',
        ],
        'outer_security': ['none', 'tls', 'reality', 'anytls', 'shadowtls'],
      },
    );
  }

  @override
  NativeResponse startProxy(ClientConfigInput config) {
    startCount += 1;
    lastStartConfig = config;
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
    validateCount += 1;
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
          'endpoints': [
            {
              'name': 'default',
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
          ],
          'active': {'type': 'endpoint', 'name': 'default'},
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
      data: {
        'toml':
            '[[endpoints]]\nname = "default"\nhost = "127.0.0.1"\nport = 443\n',
      },
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
          'endpoints': [
            {
              'name': 'default',
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
          ],
          'active': {'type': 'endpoint', 'name': 'default'},
          'local': {'host': request.listenHost, 'port': request.listenPort},
        },
        'stack_summary': 'VLESS → raw → TCP',
      },
    );
  }

  @override
  NativeResponse connectionsList() {
    connectionsListCount += 1;
    final totals = liveConnections.fold<List<int>>(
      [0, 0],
      (acc, c) => [
        acc[0] + ((c['bytes_up'] as num?)?.toInt() ?? 0),
        acc[1] + ((c['bytes_down'] as num?)?.toInt() ?? 0),
      ],
    );
    return NativeResponse(
      ok: true,
      message: 'connections snapshot',
      data: {
        'connections': liveConnections,
        'active': liveConnections.length,
        'total': liveConnections.length,
        'failed': 0,
        'bytes_uploaded': totals[0],
        'bytes_downloaded': totals[1],
      },
    );
  }

  @override
  NativeResponse connectionClose(int id) {
    closeConnectionCount += 1;
    final before = liveConnections.length;
    liveConnections = liveConnections
        .where((c) => ((c['id'] as num?)?.toInt() ?? -1) != id)
        .toList(growable: false);
    final removed = liveConnections.length != before;
    return NativeResponse(
      ok: true,
      message: removed ? 'connection close requested' : 'connection not found',
      data: {'id': id, 'closed': removed},
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
    logsSinceCount += 1;
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

class PartialWrongsvClient extends FakeWrongclClient {
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
        'payload_networks': ['tcp'],
        'base_carriers': ['tcp'],
        'active_support': 'partial',
        'active_reason': 'missing fields: reality.public-key',
        'missing_fields': [
          {'field': 'reality.public-key', 'reason': 'required'},
        ],
        'profiles': [],
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

  @override
  bool get hasNativeWindowShell => false;
}
