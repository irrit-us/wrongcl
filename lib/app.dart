import 'dart:async';

import 'package:flutter/material.dart';

import 'autostart_manager.dart';
import 'client_home_controller.dart';
import 'dashboard_view.dart';
import 'desktop_shell_controller.dart';
import 'profile_store.dart';
import 'subviews/editor_view.dart';
import 'subviews/import_view.dart';
import 'subviews/profiles_view.dart';
import 'subviews/runtime_view.dart';
import 'subviews/settings_view.dart';
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
    final scheme = ColorScheme.fromSeed(
      seedColor: const Color(0xFF006D77),
      brightness: Brightness.light,
    );
    return MaterialApp(
      title: 'Wrongcl',
      theme: ThemeData(
        colorScheme: scheme.copyWith(
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
          appBar: AppBar(
            title: Row(
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(10),
                  child: Image.asset(
                    'assets/brand/wrongcl_app_mark.png',
                    width: 30,
                    height: 30,
                    fit: BoxFit.cover,
                  ),
                ),
                const SizedBox(width: 12),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      'Wrongcl',
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                    Text(
                      'Control surface',
                      style: Theme.of(context).textTheme.labelSmall,
                    ),
                  ],
                ),
              ],
            ),
            titleSpacing: 18,
            actions: [
              if (controller.showingSecondaryPanel || controller.showingHeavyMode)
                Padding(
                  padding: const EdgeInsets.only(right: 12),
                  child: Center(
                    child: Text(
                      controller.activeRouteLabel,
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                  ),
                ),
            ],
          ),
          body: SafeArea(
            child: controller.showingHeavyMode
                ? _buildHeavyModeShell(context)
                : _buildPrimarySurfaceShell(context),
          ),
        );
      },
    );
  }

  Widget _buildPrimarySurfaceShell(BuildContext context) {
    return Stack(
      children: [
        DashboardView(
          controller: controller,
          snapshot: controller.dashboardSnapshot,
        ),
        if (controller.showingSecondaryPanel) ...[
          Positioned.fill(
            child: GestureDetector(
              onTap: controller.closeSubView,
              child: Container(color: Colors.black.withAlpha(40)),
            ),
          ),
          Align(
            alignment: Alignment.centerRight,
            child: Material(
              elevation: 16,
              color: Theme.of(context).colorScheme.surface,
              child: SizedBox(
                width: MediaQuery.sizeOf(context).width > 1100 ? 520 : 440,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Padding(
                      padding: const EdgeInsets.fromLTRB(16, 16, 12, 0),
                      child: Row(
                        children: [
                          Expanded(
                            child: Text(
                              controller.activeRouteLabel,
                              style: Theme.of(context).textTheme.titleMedium,
                            ),
                          ),
                          IconButton(
                            tooltip: 'Close panel',
                            onPressed: controller.closeSubView,
                            icon: const Icon(Icons.close),
                          ),
                        ],
                      ),
                    ),
                    const Divider(height: 1),
                    Expanded(child: _buildActiveView()),
                  ],
                ),
              ),
            ),
          ),
        ],
      ],
    );
  }

  Widget _buildHeavyModeShell(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          child: Row(
            children: [
              OutlinedButton.icon(
                onPressed: controller.closeSubView,
                icon: const Icon(Icons.arrow_back),
                label: const Text('Back to control surface'),
              ),
              const SizedBox(width: 12),
              Text(
                controller.activeRouteLabel,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ],
          ),
        ),
        Expanded(child: _buildActiveView()),
      ],
    );
  }

  Widget _buildActiveView() {
    switch (controller.activeRoute) {
      case HomeRoute.dashboard:
        return DashboardView(
          controller: controller,
          snapshot: controller.dashboardSnapshot,
        );
      case HomeRoute.profiles:
        return ProfilesView(controller: controller);
      case HomeRoute.importView:
        return ImportView(controller: controller);
      case HomeRoute.editor:
        return EditorView(controller: controller);
      case HomeRoute.runtime:
        return RuntimeView(controller: controller);
      case HomeRoute.settings:
        return SettingsView(controller: controller);
    }
  }
}
