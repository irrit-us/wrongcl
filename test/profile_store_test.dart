import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/profile_store.dart';

void main() {
  test('profile store loads legacy array payloads', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final file = File('${tempDir.path}/profiles.json');
    final store = ProfileStore(file: file);

    file.writeAsStringSync(
      jsonEncode([
        _profileJson(
          id: 'older',
          name: 'older profile',
          updatedAt: DateTime(2026, 6, 17, 11, 0),
        ),
        _profileJson(
          id: 'newer',
          name: 'newer profile',
          updatedAt: DateTime(2026, 6, 17, 12, 0),
        ),
      ]),
    );

    final loaded = await store.loadProfiles();

    expect(loaded.map((profile) => profile.id), ['newer', 'older']);
    expect(loaded.first.name, 'newer profile');
  });

  test('profile store saves and loads versioned payloads', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final file = File('${tempDir.path}/profiles.json');
    final store = ProfileStore(file: file);

    final profiles = [
      SavedProfile(
        id: 'one',
        name: 'saved profile',
        config: const {
          'server': {'host': '127.0.0.1'},
        },
        stackSummary: 'VLESS -> raw -> TCP',
        updatedAt: DateTime(2026, 6, 17, 12, 0),
      ),
    ];

    await store.saveProfiles(profiles);

    final raw = jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
    expect(raw['version'], 1);
    expect(raw['profiles'], isA<List<Object?>>());

    final loaded = await store.loadProfiles();
    expect(loaded.length, 1);
    expect(loaded.first.id, 'one');
  });

  test('profile store backs up unsupported payload versions', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final file = File('${tempDir.path}/profiles.json');
    final store = ProfileStore(file: file);

    file.writeAsStringSync(
      jsonEncode({
        'version': 99,
        'profiles': [],
      }),
    );

    final loaded = await store.loadProfiles();
    expect(loaded, isEmpty);
    expect(file.existsSync(), isFalse);
    expect(store.lastCorruptBackupPath, isNotNull);
    expect(store.lastCorruptBackupPath, startsWith('${file.path}.corrupt-'));
    expect(File(store.lastCorruptBackupPath!).existsSync(), isTrue);
  });

  test('profile store backs up syntactically broken JSON', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final file = File('${tempDir.path}/profiles.json');
    file.writeAsStringSync('{ this is not json');
    final store = ProfileStore(file: file);

    final loaded = await store.loadProfiles();
    expect(loaded, isEmpty);
    expect(file.existsSync(), isFalse);
    expect(store.lastCorruptBackupPath, isNotNull);
    expect(File(store.lastCorruptBackupPath!).existsSync(), isTrue);
  });

  test('profile store backs up structurally broken payloads', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final file = File('${tempDir.path}/profiles.json');
    file.writeAsStringSync(
      jsonEncode({
        'version': 1,
        'profiles': [42],
      }),
    );
    final store = ProfileStore(file: file);

    final loaded = await store.loadProfiles();
    expect(loaded, isEmpty);
    expect(file.existsSync(), isFalse);
    expect(store.lastCorruptBackupPath, isNotNull);
  });

  test('profile store clears lastCorruptBackupPath on successful load',
      () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final file = File('${tempDir.path}/profiles.json');
    final store = ProfileStore(file: file);
    store.lastCorruptBackupPath = '/stale/path';

    file.writeAsStringSync(jsonEncode([]));
    await store.loadProfiles();

    expect(store.lastCorruptBackupPath, isNull);
  });

  test('profile store writes via .tmp then renames over target', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final file = File('${tempDir.path}/profiles.json');
    file.writeAsStringSync('PRE-EXISTING');
    final originalLength = file.lengthSync();

    final store = ProfileStore(file: file);
    await store.saveProfiles([
      SavedProfile(
        id: 'one',
        name: 'p',
        config: const {},
        stackSummary: '',
        updatedAt: DateTime(2026, 6, 17),
      ),
    ]);

    expect(file.existsSync(), isTrue);
    expect(File('${file.path}.tmp').existsSync(), isFalse);
    expect(file.lengthSync(), isNot(originalLength));
    final reloaded = await store.loadProfiles();
    expect(reloaded.single.id, 'one');
  });
}

Map<String, Object?> _profileJson({
  required String id,
  required String name,
  required DateTime updatedAt,
}) {
  return {
    'id': id,
    'name': name,
    'config': {
      'server': {'host': '127.0.0.1'},
    },
    'stack_summary': 'VLESS -> raw -> TCP',
    'updated_at': updatedAt.toIso8601String(),
    'source': 'manual',
  };
}
