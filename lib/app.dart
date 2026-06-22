import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import 'app_settings_store.dart';
import 'autostart_manager.dart';
import 'client_home_controller.dart';
import 'desktop_shell_controller.dart';
import 'main_view.dart';
import 'profile_store.dart';
import 'subviews/connections_view.dart';
import 'subviews/logs_view.dart';
import 'subviews/mode_picker_view.dart';
import 'subviews/profiles_view.dart';
import 'subviews/proxies_view.dart';
import 'subviews/requests_view.dart';
import 'subviews/settings/advanced_view.dart';
import 'subviews/settings/basic_view.dart';
import 'subviews/settings/dns_view.dart';
import 'subviews/settings/network_view.dart';
import 'system_proxy_manager.dart';
import 'wrongcl_client.dart';

class WrongclApp extends StatefulWidget {
  const WrongclApp({
    super.key,
    this.client,
    this.profileStore,
    this.autostartManager,
    this.systemProxyManager,
    this.desktopShellController,
    this.appSettingsStore,
  });

  final WrongclClient? client;
  final ProfileStore? profileStore;
  final AutostartManager? autostartManager;
  final SystemProxyManager? systemProxyManager;
  final DesktopShellController? desktopShellController;
  final AppSettingsStore? appSettingsStore;

  @override
  State<WrongclApp> createState() => _WrongclAppState();
}

class _WrongclAppState extends State<WrongclApp> {
  late final WrongclClient _client;
  late final ProfileStore _profileStore;
  late final AutostartManager _autostartManager;
  late final SystemProxyManager _systemProxyManager;
  late final DesktopShellController _desktopShellController;
  late final AppSettingsStore _appSettingsStore;
  ThemeMode _themeMode = ThemeMode.system;
  Locale _locale = const Locale('en');

  @override
  void initState() {
    super.initState();
    _client = widget.client ?? NativeWrongclClient();
    _profileStore = widget.profileStore ?? ProfileStore();
    _autostartManager = widget.autostartManager ?? AutostartManager();
    _systemProxyManager = widget.systemProxyManager ?? SystemProxyManager();
    _desktopShellController =
        widget.desktopShellController ?? const NoopDesktopShellController();
    _appSettingsStore = widget.appSettingsStore ?? AppSettingsStore();
    unawaited(_loadAppSettings());
  }

  Future<void> _loadAppSettings() async {
    try {
      final settings = await _appSettingsStore.load();
      if (!mounted) return;
      setState(() {
        _themeMode = settings.themeMode;
        _locale = _localeFromCode(settings.localeCode);
      });
    } catch (_) {}
  }

  Future<void> _setThemeMode(ThemeMode value) async {
    setState(() {
      _themeMode = value;
    });
    await _appSettingsStore.save(
      AppSettings(themeMode: value, localeCode: _locale.languageCode),
    );
  }

  Future<void> _setLocaleCode(String value) async {
    final locale = _localeFromCode(value);
    setState(() {
      _locale = locale;
    });
    await _appSettingsStore.save(
      AppSettings(themeMode: _themeMode, localeCode: locale.languageCode),
    );
  }

  Locale _localeFromCode(String code) {
    switch (code) {
      case 'zh':
        return const Locale('zh', 'CN');
      default:
        return const Locale('en');
    }
  }

  @override
  Widget build(BuildContext context) {
    final lightScheme = ColorScheme.fromSeed(
      seedColor: const Color(0xFF006D77),
      brightness: Brightness.light,
    );
    final darkScheme = ColorScheme.fromSeed(
      seedColor: const Color(0xFF4A8C8A),
      brightness: Brightness.dark,
    );
    return MaterialApp(
      title: 'Wrongcl',
      themeMode: _themeMode,
      locale: _locale,
      supportedLocales: const [Locale('en'), Locale('zh', 'CN')],
      localizationsDelegates: const [
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      theme: ThemeData(
        colorScheme: lightScheme.copyWith(
          surface: const Color(0xFFF7F6F2),
          surfaceContainerHighest: const Color(0xFFE5E2DA),
        ),
        scaffoldBackgroundColor: const Color(0xFFF2EFE8),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFFF2EFE8),
          surfaceTintColor: Colors.transparent,
          elevation: 0,
        ),
        inputDecorationTheme: const InputDecorationTheme(
          border: OutlineInputBorder(),
          isDense: true,
        ),
        cardTheme: const CardThemeData(
          margin: EdgeInsets.zero,
          color: Color(0xFFFBFAF7),
          surfaceTintColor: Colors.transparent,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.all(Radius.circular(18)),
          ),
        ),
        dividerTheme: const DividerThemeData(
          color: Color(0xFFD7D2C8),
          thickness: 1,
        ),
        filledButtonTheme: FilledButtonThemeData(
          style: FilledButton.styleFrom(
            backgroundColor: const Color(0xFF111111),
            foregroundColor: Colors.white,
            padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 16),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(14),
            ),
          ),
        ),
        outlinedButtonTheme: OutlinedButtonThemeData(
          style: OutlinedButton.styleFrom(
            foregroundColor: const Color(0xFF1F2933),
            side: const BorderSide(color: Color(0xFFB8B1A4)),
            padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 16),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(14),
            ),
          ),
        ),
      ),
      darkTheme: ThemeData(
        colorScheme: darkScheme.copyWith(
          surface: const Color(0xFF171C1F),
          surfaceContainerHighest: const Color(0xFF253038),
        ),
        scaffoldBackgroundColor: const Color(0xFF101417),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF101417),
          surfaceTintColor: Colors.transparent,
          elevation: 0,
        ),
        inputDecorationTheme: const InputDecorationTheme(
          border: OutlineInputBorder(),
          isDense: true,
        ),
        cardTheme: const CardThemeData(
          margin: EdgeInsets.zero,
          color: Color(0xFF171C1F),
          surfaceTintColor: Colors.transparent,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.all(Radius.circular(18)),
          ),
        ),
        dividerTheme: const DividerThemeData(
          color: Color(0xFF2F3940),
          thickness: 1,
        ),
        filledButtonTheme: FilledButtonThemeData(
          style: FilledButton.styleFrom(
            backgroundColor: const Color(0xFFE6F1EF),
            foregroundColor: const Color(0xFF101417),
            padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 16),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(14),
            ),
          ),
        ),
        outlinedButtonTheme: OutlinedButtonThemeData(
          style: OutlinedButton.styleFrom(
            foregroundColor: const Color(0xFFE6F1EF),
            side: const BorderSide(color: Color(0xFF4B5963)),
            padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 16),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(14),
            ),
          ),
        ),
      ),
      home: ClientHome(
        client: _client,
        profileStore: _profileStore,
        autostartManager: _autostartManager,
        systemProxyManager: _systemProxyManager,
        desktopShellController: _desktopShellController,
        themeMode: _themeMode,
        onThemeModeChanged: _setThemeMode,
        locale: _locale,
        onLocaleCodeChanged: _setLocaleCode,
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
    required this.themeMode,
    required this.onThemeModeChanged,
    required this.locale,
    required this.onLocaleCodeChanged,
  });

  final WrongclClient client;
  final ProfileStore profileStore;
  final AutostartManager autostartManager;
  final SystemProxyManager systemProxyManager;
  final DesktopShellController desktopShellController;
  final ThemeMode themeMode;
  final Future<void> Function(ThemeMode value) onThemeModeChanged;
  final Locale locale;
  final Future<void> Function(String value) onLocaleCodeChanged;

  @override
  State<ClientHome> createState() => _ClientHomeState();
}

class _ClientHomeState extends State<ClientHome> {
  late final ClientHomeController controller;

  @override
  void initState() {
    super.initState();
    controller = ClientHomeController(
      client: widget.client,
      profileStore: widget.profileStore,
      autostartManager: widget.autostartManager,
      systemProxyManager: widget.systemProxyManager,
      desktopShellController: widget.desktopShellController,
    );
    unawaited(controller.init());
  }

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: controller,
      builder: (context, _) {
        return Scaffold(
          body: Stack(
            children: [
              MainView(controller: controller),
              if (controller.showingSubpage)
                Positioned.fill(child: _buildActiveSubpage()),
            ],
          ),
        );
      },
    );
  }

  Widget _buildActiveSubpage() {
    final close = controller.closeSubView;
    switch (controller.activeRoute) {
      case HomeRoute.dashboard:
        return const SizedBox.shrink();
      case HomeRoute.profiles:
        return ProfilesView(controller: controller, onClose: close);
      case HomeRoute.proxies:
        return ProxiesView(controller: controller, onClose: close);
      case HomeRoute.connections:
        return ConnectionsView(controller: controller, onClose: close);
      case HomeRoute.requests:
        return RequestsView(controller: controller, onClose: close);
      case HomeRoute.logs:
        return LogsView(controller: controller, onClose: close);
      case HomeRoute.modePicker:
        return ModePickerView(controller: controller, onClose: close);
      case HomeRoute.settingsBasic:
        return BasicSettingsView(
          controller: controller,
          onClose: close,
          themeMode: widget.themeMode,
          onThemeModeChanged: widget.onThemeModeChanged,
          locale: widget.locale,
          onLocaleCodeChanged: widget.onLocaleCodeChanged,
        );
      case HomeRoute.settingsNetwork:
        return NetworkSettingsView(controller: controller, onClose: close);
      case HomeRoute.settingsDns:
        return DnsSettingsView(controller: controller, onClose: close);
      case HomeRoute.settingsAdvanced:
        return AdvancedSettingsView(controller: controller, onClose: close);
    }
  }
}
