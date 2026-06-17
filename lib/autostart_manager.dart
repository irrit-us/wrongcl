import 'dart:io';

enum AutostartPlatform {
  linux,
  macos,
  windows,
  unsupported,
}

class AutostartStatus {
  const AutostartStatus({
    required this.supported,
    required this.enabled,
    required this.path,
    required this.message,
  });

  final bool supported;
  final bool enabled;
  final String path;
  final String message;
}

class AutostartManager {
  AutostartManager({
    this.file,
    this.executablePath,
    this.platform,
  });

  final File? file;
  final String? executablePath;
  final AutostartPlatform? platform;

  Future<AutostartStatus> loadStatus() async {
    final file = _resolveFile();
    if (file == null) {
      return const AutostartStatus(
        supported: false,
        enabled: false,
        path: '',
        message: 'Start-on-login is not implemented for this platform yet',
      );
    }
    final enabled = await file.exists();
    return AutostartStatus(
      supported: true,
      enabled: enabled,
      path: file.path,
      message: enabled ? 'Enabled' : 'Disabled',
    );
  }

  Future<void> enable() async {
    final file = _resolveFile();
    if (file == null) {
      throw UnsupportedError(
        'Start-on-login is not implemented for this platform yet',
      );
    }
    final executable = _resolveExecutablePath();
    await file.parent.create(recursive: true);
    await file.writeAsString(_autostartContents(executable));
  }

  Future<void> disable() async {
    final file = _resolveFile();
    if (file == null) {
      throw UnsupportedError(
        'Start-on-login is not implemented for this platform yet',
      );
    }
    if (await file.exists()) {
      await file.delete();
    }
  }

  File? _resolveFile() {
    if (file != null) {
      return file;
    }
    switch (_resolvePlatform()) {
      case AutostartPlatform.linux:
        final home = Platform.environment['HOME'] ?? '.';
        return File('$home/.config/autostart/wrongcl.desktop');
      case AutostartPlatform.macos:
        final home = Platform.environment['HOME'] ?? '.';
        return File('$home/Library/LaunchAgents/us.irrit.wrongcl.plist');
      case AutostartPlatform.windows:
        final appData = Platform.environment['APPDATA'] ?? '.';
        return File(
          '$appData\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\wrongcl.cmd',
        );
      case AutostartPlatform.unsupported:
        return null;
    }
  }

  String _resolveExecutablePath() {
    return executablePath ?? Platform.resolvedExecutable;
  }

  AutostartPlatform _resolvePlatform() {
    if (platform != null) {
      return platform!;
    }
    if (Platform.isLinux) {
      return AutostartPlatform.linux;
    }
    if (Platform.isMacOS) {
      return AutostartPlatform.macos;
    }
    if (Platform.isWindows) {
      return AutostartPlatform.windows;
    }
    return AutostartPlatform.unsupported;
  }

  String _autostartContents(String executablePath) {
    switch (_resolvePlatform()) {
      case AutostartPlatform.linux:
        return _linuxDesktopEntry(executablePath);
      case AutostartPlatform.macos:
        return _macosLaunchAgent(executablePath);
      case AutostartPlatform.windows:
        return _windowsStartupScript(executablePath);
      case AutostartPlatform.unsupported:
        throw UnsupportedError(
          'Start-on-login is not implemented for this platform yet',
        );
    }
  }

  String _linuxDesktopEntry(String executablePath) {
    return '''
[Desktop Entry]
Type=Application
Name=wrongcl
Exec=$executablePath
Terminal=false
X-GNOME-Autostart-enabled=true
''';
  }

  String _macosLaunchAgent(String executablePath) {
    return '''
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>us.irrit.wrongcl</string>
  <key>ProgramArguments</key>
  <array>
    <string>$executablePath</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
''';
  }

  String _windowsStartupScript(String executablePath) {
    return '''
@echo off
start "" "$executablePath"
''';
  }
}
