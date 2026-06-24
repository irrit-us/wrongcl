import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/app_settings_store.dart';
import 'package:wrongcl/theme/wrongcl_colors.dart';

void main() {
  test('app settings store saves and loads theme mode', () async {
    final tempDir = Directory.systemTemp.createTempSync(
      'wrongcl-settings-test',
    );
    final store = AppSettingsStore(
      file: File('${tempDir.path}/app_settings.json'),
    );

    await store.save(const AppSettings(themeMode: ThemeMode.dark));
    final loaded = await store.load();

    expect(loaded.themeMode, ThemeMode.dark);
  });

  test('app settings store saves and loads locale code', () async {
    final tempDir = Directory.systemTemp.createTempSync(
      'wrongcl-settings-test',
    );
    final store = AppSettingsStore(
      file: File('${tempDir.path}/app_settings.json'),
    );

    await store.save(
      const AppSettings(themeMode: ThemeMode.system, localeCode: 'zh'),
    );
    final loaded = await store.load();

    expect(loaded.localeCode, 'zh');
  });

  test('app settings store saves and loads theme variant', () async {
    final tempDir = Directory.systemTemp.createTempSync(
      'wrongcl-settings-test',
    );
    final store = AppSettingsStore(
      file: File('${tempDir.path}/app_settings.json'),
    );

    await store.save(
      const AppSettings(themeVariant: WrongclThemeVariant.catppuccin),
    );
    final loaded = await store.load();

    expect(loaded.themeVariant, WrongclThemeVariant.catppuccin);
  });

  test('app settings store falls back to wrongcl variant for missing field',
      () async {
    final tempDir = Directory.systemTemp.createTempSync(
      'wrongcl-settings-test',
    );
    final file = File('${tempDir.path}/app_settings.json');
    await file.writeAsString(
      '{"version":1,"theme_mode":"system","locale_code":"en"}',
    );
    final store = AppSettingsStore(file: file);

    final loaded = await store.load();

    expect(loaded.themeVariant, WrongclThemeVariant.wrongcl);
  });
}
