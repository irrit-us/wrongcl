import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

const _profileStoreVersion = 1;

class SavedProfile {
  const SavedProfile({
    required this.id,
    required this.name,
    required this.config,
    required this.stackSummary,
    required this.updatedAt,
    this.source = 'manual',
    this.sourcePath,
    this.activeProfile,
    this.supportState,
    this.supportReason,
    this.importReport,
  });

  final String id;
  final String name;
  final Map<String, Object?> config;
  final String stackSummary;
  final DateTime updatedAt;
  final String source;
  final String? sourcePath;
  final String? activeProfile;
  final String? supportState;
  final String? supportReason;
  final Map<String, Object?>? importReport;

  SavedProfile copyWith({
    String? id,
    String? name,
    Map<String, Object?>? config,
    String? stackSummary,
    DateTime? updatedAt,
    String? source,
    String? sourcePath,
    String? activeProfile,
    String? supportState,
    String? supportReason,
    Map<String, Object?>? importReport,
  }) {
    return SavedProfile(
      id: id ?? this.id,
      name: name ?? this.name,
      config: config ?? this.config,
      stackSummary: stackSummary ?? this.stackSummary,
      updatedAt: updatedAt ?? this.updatedAt,
      source: source ?? this.source,
      sourcePath: sourcePath ?? this.sourcePath,
      activeProfile: activeProfile ?? this.activeProfile,
      supportState: supportState ?? this.supportState,
      supportReason: supportReason ?? this.supportReason,
      importReport: importReport ?? this.importReport,
    );
  }

  Map<String, Object?> toJson() => {
    'id': id,
    'name': name,
    'config': config,
    'stack_summary': stackSummary,
    'updated_at': updatedAt.toIso8601String(),
    'source': source,
    'source_path': sourcePath,
    'active_profile': activeProfile,
    'support_state': supportState,
    'support_reason': supportReason,
    'import_report': importReport,
  };

  factory SavedProfile.fromJson(Map<String, Object?> json) {
    return SavedProfile(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? 'Unnamed profile',
      config: Map<String, Object?>.from(json['config'] as Map? ?? const {}),
      stackSummary: json['stack_summary'] as String? ?? '',
      updatedAt:
          DateTime.tryParse(json['updated_at'] as String? ?? '') ??
          DateTime.fromMillisecondsSinceEpoch(0),
      source: json['source'] as String? ?? 'manual',
      sourcePath: json['source_path'] as String?,
      activeProfile: json['active_profile'] as String?,
      supportState: json['support_state'] as String?,
      supportReason: json['support_reason'] as String?,
      importReport: json['import_report'] is Map
          ? Map<String, Object?>.from(json['import_report'] as Map)
          : null,
    );
  }
}

class ProfileStore {
  ProfileStore({this.file});

  final File? file;

  Future<List<SavedProfile>> loadProfiles() async {
    final file = await _resolveFile();
    if (!await file.exists()) {
      return const [];
    }
    final raw = await file.readAsString();
    if (raw.trim().isEmpty) {
      return const [];
    }
    final decoded = jsonDecode(raw);
    if (decoded is List) {
      return _decodeProfiles(decoded);
    }
    if (decoded is Map) {
      final document = Map<String, Object?>.from(decoded);
      final version = document['version'];
      if (version is! num || version != version.toInt()) {
        throw const FormatException(
          'profiles.json version must be an integer',
        );
      }
      if (version.toInt() != _profileStoreVersion) {
        throw FormatException(
          'unsupported profiles.json version: ${version.toInt()}',
        );
      }
      return _decodeProfiles(document['profiles']);
    }
    throw const FormatException(
      'profiles.json must contain a profile array or versioned profile document',
    );
  }

  Future<void> saveProfiles(List<SavedProfile> profiles) async {
    final file = await _resolveFile();
    await file.parent.create(recursive: true);
    final payload = jsonEncode({
      'version': _profileStoreVersion,
      'profiles': [for (final profile in profiles) profile.toJson()],
    });
    await file.writeAsString(payload);
  }

  List<SavedProfile> _decodeProfiles(Object? rawProfiles) {
    if (rawProfiles is! List) {
      throw const FormatException('profiles payload must contain a JSON array');
    }
    return rawProfiles
        .map((value) {
          if (value is! Map) {
            throw const FormatException(
              'each saved profile must be a JSON object',
            );
          }
          return SavedProfile.fromJson(Map<String, Object?>.from(value));
        })
        .toList()
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
  }

  Future<File> _resolveFile() async {
    if (file != null) {
      return file!;
    }
    return File(await _defaultProfilePath());
  }

  static Future<String> _defaultProfilePath() async {
    if (Platform.isWindows) {
      final base = Platform.environment['APPDATA'] ?? '.';
      return '$base\\wrongcl\\profiles.json';
    }
    if (Platform.isMacOS) {
      final home = Platform.environment['HOME'] ?? '.';
      return '$home/Library/Application Support/wrongcl/profiles.json';
    }
    final xdg = Platform.environment['XDG_CONFIG_HOME'];
    if (xdg != null && xdg.isNotEmpty) {
      return '$xdg/wrongcl/profiles.json';
    }
    if (Platform.isAndroid || Platform.isIOS) {
      final base = await getApplicationSupportDirectory();
      return '${base.path}/profiles.json';
    }
    final home = Platform.environment['HOME'] ?? '.';
    return '$home/.config/wrongcl/profiles.json';
  }
}
