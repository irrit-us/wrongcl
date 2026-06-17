import 'dart:io';

enum SystemProxyPlatform {
  linux,
  unsupported,
}

class SystemProxyStatus {
  const SystemProxyStatus({
    required this.supported,
    required this.enabled,
    required this.mode,
    required this.message,
  });

  final bool supported;
  final bool enabled;
  final String mode;
  final String message;
}

typedef ProcessRunner =
    Future<ProcessResult> Function(String executable, List<String> arguments);

class SystemProxyManager {
  SystemProxyManager({
    ProcessRunner? runner,
    this.platform,
  }) : _runner = runner ?? Process.run;

  final ProcessRunner _runner;
  final SystemProxyPlatform? platform;

  Future<SystemProxyStatus> loadStatus() async {
    if (_resolvePlatform() != SystemProxyPlatform.linux) {
      return const SystemProxyStatus(
        supported: false,
        enabled: false,
        mode: 'unsupported',
        message: 'System proxy integration is not implemented for this platform yet',
      );
    }

    final modeResult = await _runner('gsettings', [
      'get',
      'org.gnome.system.proxy',
      'mode',
    ]);
    if (modeResult.exitCode != 0) {
      return SystemProxyStatus(
        supported: false,
        enabled: false,
        mode: 'unavailable',
        message: _stderrOrFallback(
          modeResult,
          'gsettings is unavailable or GNOME proxy settings cannot be read',
        ),
      );
    }

    final mode = modeResult.stdout.toString().trim().replaceAll("'", '');
    return SystemProxyStatus(
      supported: true,
      enabled: mode == 'manual',
      mode: mode,
      message: mode == 'manual' ? 'Enabled' : 'Disabled',
    );
  }

  Future<void> enableSocks(String host, int port) async {
    if (_resolvePlatform() != SystemProxyPlatform.linux) {
      throw UnsupportedError(
        'System proxy integration is not implemented for this platform yet',
      );
    }
    await _runChecked('gsettings', [
      'set',
      'org.gnome.system.proxy',
      'mode',
      'manual',
    ]);
    await _runChecked('gsettings', [
      'set',
      'org.gnome.system.proxy.socks',
      'host',
      host,
    ]);
    await _runChecked('gsettings', [
      'set',
      'org.gnome.system.proxy.socks',
      'port',
      '$port',
    ]);
  }

  Future<void> disable() async {
    if (_resolvePlatform() != SystemProxyPlatform.linux) {
      throw UnsupportedError(
        'System proxy integration is not implemented for this platform yet',
      );
    }
    await _runChecked('gsettings', [
      'set',
      'org.gnome.system.proxy',
      'mode',
      'none',
    ]);
  }

  SystemProxyPlatform _resolvePlatform() {
    final selectedPlatform = platform;
    if (selectedPlatform != null) {
      return selectedPlatform;
    }
    if (Platform.isLinux) {
      return SystemProxyPlatform.linux;
    }
    return SystemProxyPlatform.unsupported;
  }

  Future<void> _runChecked(String executable, List<String> args) async {
    final result = await _runner(executable, args);
    if (result.exitCode != 0) {
      throw ProcessException(
        executable,
        args,
        _stderrOrFallback(result, 'system proxy command failed'),
        result.exitCode,
      );
    }
  }

  String _stderrOrFallback(ProcessResult result, String fallback) {
    final stderr = result.stderr.toString().trim();
    return stderr.isEmpty ? fallback : stderr;
  }
}
