import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import 'theme/wrongcl_colors.dart';
import 'widgets/entry_chip.dart';

const _appSettingsVersion = 1;

class AppSettings {
  const AppSettings({
    this.themeMode = ThemeMode.system,
    this.localeCode = 'en',
    this.themeVariant = WrongclThemeVariant.wrongcl,
    this.chipIconSide = ChipIconSide.left,
  });

  final ThemeMode themeMode;
  final String localeCode;
  final WrongclThemeVariant themeVariant;
  final ChipIconSide chipIconSide;

  AppSettings copyWith({
    ThemeMode? themeMode,
    String? localeCode,
    WrongclThemeVariant? themeVariant,
    ChipIconSide? chipIconSide,
  }) {
    return AppSettings(
      themeMode: themeMode ?? this.themeMode,
      localeCode: localeCode ?? this.localeCode,
      themeVariant: themeVariant ?? this.themeVariant,
      chipIconSide: chipIconSide ?? this.chipIconSide,
    );
  }

  Map<String, Object?> toJson() => {
        'version': _appSettingsVersion,
        'theme_mode': _themeModeId(themeMode),
        'locale_code': localeCode,
        'theme_variant': themeVariant.id,
        'chip_icon_side': _chipIconSideId(chipIconSide),
      };

  factory AppSettings.fromJson(Map<String, Object?> json) {
    final version = json['version'];
    if (version is num && version.toInt() != _appSettingsVersion) {
      throw FormatException('unsupported app settings version: ${version.toInt()}');
    }
    return AppSettings(
      themeMode: _themeModeFromId(json['theme_mode'] as String? ?? 'system'),
      localeCode: json['locale_code'] as String? ?? 'en',
      themeVariant: WrongclThemeVariantId.fromId(
        json['theme_variant'] as String?,
      ),
      chipIconSide:
          _chipIconSideFromId(json['chip_icon_side'] as String? ?? 'left'),
    );
  }

  static String _themeModeId(ThemeMode mode) => switch (mode) {
        ThemeMode.system => 'system',
        ThemeMode.light => 'light',
        ThemeMode.dark => 'dark',
      };

  static ThemeMode _themeModeFromId(String id) => switch (id) {
        'light' => ThemeMode.light,
        'dark' => ThemeMode.dark,
        _ => ThemeMode.system,
      };

  static String _chipIconSideId(ChipIconSide side) => switch (side) {
        ChipIconSide.left => 'left',
        ChipIconSide.right => 'right',
      };

  static ChipIconSide _chipIconSideFromId(String id) => switch (id) {
        'right' => ChipIconSide.right,
        _ => ChipIconSide.left,
      };
}

class AppSettingsStore {
  AppSettingsStore({this.file});

  final File? file;

  Future<AppSettings> load() async {
    final resolved = await _resolveFile();
    if (!await resolved.exists()) {
      return const AppSettings();
    }
    final raw = await resolved.readAsString();
    if (raw.trim().isEmpty) {
      return const AppSettings();
    }
    final decoded = jsonDecode(raw);
    if (decoded is! Map<String, Object?>) {
      throw const FormatException('app settings must be a JSON object');
    }
    return AppSettings.fromJson(decoded);
  }

  Future<void> save(AppSettings settings) async {
    final resolved = await _resolveFile();
    await resolved.parent.create(recursive: true);
    await resolved.writeAsString(jsonEncode(settings.toJson()));
  }

  Future<File> _resolveFile() async {
    if (file != null) {
      return file!;
    }
    return File(await _defaultPath());
  }

  static Future<String> _defaultPath() async {
    if (Platform.isWindows) {
      final base = Platform.environment['APPDATA'] ?? '.';
      return '$base\\wrongcl\\app_settings.json';
    }
    if (Platform.isMacOS) {
      final home = Platform.environment['HOME'] ?? '.';
      return '$home/Library/Application Support/wrongcl/app_settings.json';
    }
    final xdg = Platform.environment['XDG_CONFIG_HOME'];
    if (xdg != null && xdg.isNotEmpty) {
      return '$xdg/wrongcl/app_settings.json';
    }
    if (Platform.isAndroid || Platform.isIOS) {
      final base = await getApplicationSupportDirectory();
      return '${base.path}/app_settings.json';
    }
    final home = Platform.environment['HOME'] ?? '.';
    return '$home/.config/wrongcl/app_settings.json';
  }
}
