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

  test('profile store rejects unsupported payload versions', () async {
    final tempDir = Directory.systemTemp.createTempSync('wrongcl-profile-test');
    final file = File('${tempDir.path}/profiles.json');
    final store = ProfileStore(file: file);

    file.writeAsStringSync(
      jsonEncode({
        'version': 99,
        'profiles': [],
      }),
    );

    await expectLater(store.loadProfiles(), throwsFormatException);
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
