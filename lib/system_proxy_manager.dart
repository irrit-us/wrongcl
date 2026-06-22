import 'dart:io';

enum SystemProxyPlatform { linux, windows, macos, unsupported }

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
  SystemProxyManager({ProcessRunner? runner, this.platform})
    : _runner = runner ?? Process.run;

  final ProcessRunner _runner;
  final SystemProxyPlatform? platform;

  Future<SystemProxyStatus> loadStatus() async {
    switch (_resolvePlatform()) {
      case SystemProxyPlatform.linux:
        return _loadLinuxStatus();
      case SystemProxyPlatform.windows:
        return _loadWindowsStatus();
      case SystemProxyPlatform.macos:
        return _loadMacosStatus();
      case SystemProxyPlatform.unsupported:
        return const SystemProxyStatus(
          supported: false,
          enabled: false,
          mode: 'unsupported',
          message:
              'System proxy integration is not implemented for this platform yet',
        );
    }
  }

  Future<SystemProxyStatus> _loadLinuxStatus() async {
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

  Future<SystemProxyStatus> _loadWindowsStatus() async {
    final query = await _runner('reg', [
      'query',
      r'HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings',
      '/v',
      'ProxyEnable',
    ]);
    if (query.exitCode != 0) {
      return SystemProxyStatus(
        supported: false,
        enabled: false,
        mode: 'unavailable',
        message: _stderrOrFallback(
          query,
          'Windows Internet Settings proxy state cannot be read',
        ),
      );
    }

    final enabled = _parseWindowsProxyEnabled(query.stdout.toString());
    final serverQuery = await _runner('reg', [
      'query',
      r'HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings',
      '/v',
      'ProxyServer',
    ]);
    final proxyServer = serverQuery.exitCode == 0
        ? _parseWindowsProxyServer(serverQuery.stdout.toString())
        : '';
    return SystemProxyStatus(
      supported: true,
      enabled: enabled,
      mode: enabled ? 'manual' : 'none',
      message: enabled
          ? proxyServer.isEmpty
                ? 'Enabled'
                : 'Enabled: $proxyServer'
          : 'Disabled',
    );
  }

  Future<SystemProxyStatus> _loadMacosStatus() async {
    return const SystemProxyStatus(
      supported: false,
      enabled: false,
      mode: 'planned',
      message:
          'System proxy integration for macOS is planned but not implemented yet',
    );
  }

  Future<void> enableSocks(String host, int port) async {
    switch (_resolvePlatform()) {
      case SystemProxyPlatform.linux:
        await _enableLinuxSocks(host, port);
        return;
      case SystemProxyPlatform.windows:
        await _enableWindowsSocks(host, port);
        return;
      case SystemProxyPlatform.macos:
        throw UnsupportedError(
          'System proxy integration for macOS is planned but not implemented yet',
        );
      case SystemProxyPlatform.unsupported:
        throw UnsupportedError(
          'System proxy integration is not implemented for this platform yet',
        );
    }
  }

  Future<void> _enableLinuxSocks(String host, int port) async {
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
    switch (_resolvePlatform()) {
      case SystemProxyPlatform.linux:
        await _disableLinux();
        return;
      case SystemProxyPlatform.windows:
        await _disableWindows();
        return;
      case SystemProxyPlatform.macos:
        throw UnsupportedError(
          'System proxy integration for macOS is planned but not implemented yet',
        );
      case SystemProxyPlatform.unsupported:
        throw UnsupportedError(
          'System proxy integration is not implemented for this platform yet',
        );
    }
  }

  Future<void> _disableLinux() async {
    await _runChecked('gsettings', [
      'set',
      'org.gnome.system.proxy',
      'mode',
      'none',
    ]);
  }

  Future<void> _enableWindowsSocks(String host, int port) async {
    await _runChecked('reg', [
      'add',
      r'HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings',
      '/v',
      'ProxyServer',
      '/t',
      'REG_SZ',
      '/d',
      'socks=$host:$port',
      '/f',
    ]);
    await _runChecked('reg', [
      'add',
      r'HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings',
      '/v',
      'ProxyEnable',
      '/t',
      'REG_DWORD',
      '/d',
      '1',
      '/f',
    ]);
    await _refreshWindowsInternetSettings();
  }

  Future<void> _disableWindows() async {
    await _runChecked('reg', [
      'add',
      r'HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings',
      '/v',
      'ProxyEnable',
      '/t',
      'REG_DWORD',
      '/d',
      '0',
      '/f',
    ]);
    await _refreshWindowsInternetSettings();
  }

  SystemProxyPlatform _resolvePlatform() {
    final selectedPlatform = platform;
    if (selectedPlatform != null) {
      return selectedPlatform;
    }
    if (Platform.isLinux) {
      return SystemProxyPlatform.linux;
    }
    if (Platform.isWindows) {
      return SystemProxyPlatform.windows;
    }
    if (Platform.isMacOS) {
      return SystemProxyPlatform.macos;
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

  bool _parseWindowsProxyEnabled(String stdout) {
    final text = stdout.toUpperCase();
    return text.contains('0X1') || RegExp(r'\b1\b').hasMatch(text);
  }

  String _parseWindowsProxyServer(String stdout) {
    final match = RegExp(
      r'ProxyServer\s+REG_\w+\s+(.+)$',
      multiLine: true,
    ).firstMatch(stdout);
    return match?.group(1)?.trim() ?? '';
  }

  Future<void> _refreshWindowsInternetSettings() async {
    const script = r'''
$signature = @"
[DllImport("wininet.dll", SetLastError = true)]
public static extern bool InternetSetOption(int hInternet, int dwOption, IntPtr lpBuffer, int dwBufferLength);
"@;
$type = Add-Type -MemberDefinition $signature -Name WinInetRefresh -Namespace Wrongcl -PassThru;
[void]$type::InternetSetOption(0, 39, [IntPtr]::Zero, 0);
[void]$type::InternetSetOption(0, 37, [IntPtr]::Zero, 0);
''';
    await _runChecked('powershell', [
      '-NoProfile',
      '-NonInteractive',
      '-Command',
      script,
    ]);
  }
}
