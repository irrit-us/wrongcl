import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

enum ProxyKind {
  vless('vless', 'VLESS'),
  hysteria2('hysteria2', 'Hysteria2'),
  tuic('tuic', 'TUIC'),
  trojan('trojan', 'Trojan'),
  mixed('mixed', 'Mixed remote SOCKS/HTTP'),
  shadowsocks('shadowsocks', 'Shadowsocks');

  const ProxyKind(this.id, this.label);
  final String id;
  final String label;

  static ProxyKind fromId(String id) =>
      values.firstWhere((value) => value.id == id, orElse: () => vless);
}

enum TransportKind {
  raw('raw', 'raw TCP'),
  kcp('kcp', 'KCP'),
  meek('meek', 'Meek'),
  gdocsviewer('gdocsviewer', 'Google Docs Viewer'),
  quic('quic', 'QUIC'),
  webtransport('webtransport', 'WebTransport'),
  websocket('websocket', 'WebSocket'),
  httpupgrade('httpupgrade', 'HTTPUpgrade'),
  xhttp('xhttp', 'XHTTP'),
  grpc('grpc', 'gRPC');

  const TransportKind(this.id, this.label);
  final String id;
  final String label;

  static TransportKind fromId(String id) =>
      values.firstWhere((value) => value.id == id, orElse: () => raw);
}

enum OuterSecurityKind {
  none('none', 'none'),
  tls('tls', 'TLS'),
  reality('reality', 'REALITY'),
  anytls('anytls', 'AnyTLS'),
  shadowtls('shadowtls', 'ShadowTLS');

  const OuterSecurityKind(this.id, this.label);
  final String id;
  final String label;

  static OuterSecurityKind fromId(String id) =>
      values.firstWhere((value) => value.id == id, orElse: () => none);
}

class VlessConfig {
  const VlessConfig({required this.uuid, this.flow = ''});
  final String uuid;
  final String flow;

  Map<String, Object?> toJson() => {
    'type': 'vless',
    'uuid': uuid,
    'flow': flow,
  };
}

class TrojanConfig {
  const TrojanConfig({required this.password});
  final String password;

  Map<String, Object?> toJson() => {'type': 'trojan', 'password': password};
}

class Hysteria2Config {
  const Hysteria2Config({
    required this.serverName,
    required this.password,
    this.udpEnabled = true,
  });

  final String serverName;
  final String password;
  final bool udpEnabled;

  Map<String, Object?> toJson() => {
    'type': 'hysteria2',
    'server-name': serverName,
    'password': password,
    'udp-enabled': udpEnabled,
  };
}

class TuicConfig {
  const TuicConfig({
    required this.serverName,
    required this.uuid,
    required this.password,
  });

  final String serverName;
  final String uuid;
  final String password;

  Map<String, Object?> toJson() => {
    'type': 'tuic',
    'server-name': serverName,
    'uuid': uuid,
    'password': password,
  };
}

class MixedConfig {
  const MixedConfig({this.username, this.password});
  final String? username;
  final String? password;

  Map<String, Object?> toJson() => {
    'type': 'mixed',
    'username': username,
    'password': password,
  };
}

class ShadowsocksConfig {
  const ShadowsocksConfig({
    this.method = 'chacha20-ietf-poly1305',
    required this.password,
  });
  final String method;
  final String password;

  Map<String, Object?> toJson() => {
    'type': 'shadowsocks',
    'method': method,
    'password': password,
  };
}

class WsConfig {
  const WsConfig({this.path = '/ws', this.host});
  final String path;
  final String? host;

  Map<String, Object?> toJson() => {
    'type': 'websocket',
    'path': path,
    'host': host,
  };
}

class HuConfig {
  const HuConfig({this.path = '/up', this.host});
  final String path;
  final String? host;

  Map<String, Object?> toJson() => {
    'type': 'httpupgrade',
    'path': path,
    'host': host,
  };
}

class XhttpConfig {
  const XhttpConfig({this.path = '/xhttp', this.host});
  final String path;
  final String? host;

  Map<String, Object?> toJson() => {
    'type': 'xhttp',
    'path': path,
    'host': host,
  };
}

class GrpcConfig {
  const GrpcConfig({this.serviceName = 'GunService'});
  final String serviceName;

  Map<String, Object?> toJson() => {
    'type': 'grpc',
    'service-name': serviceName,
  };
}

class QuicConfig {
  const QuicConfig({required this.serverName, this.udpEnabled = true});

  final String serverName;
  final bool udpEnabled;

  Map<String, Object?> toJson() => {
    'type': 'quic',
    'server-name': serverName,
    'udp-enabled': udpEnabled,
  };
}

class WebTransportConfig {
  const WebTransportConfig({
    required this.authority,
    this.path = '/wt',
    this.udpEnabled = true,
  });

  final String authority;
  final String path;
  final bool udpEnabled;

  Map<String, Object?> toJson() => {
    'type': 'webtransport',
    'authority': authority,
    'path': path,
    'udp-enabled': udpEnabled,
  };
}

class KcpConfig {
  const KcpConfig({this.seed = '', this.mtu = 1350, this.tti = 50});

  final String seed;
  final int mtu;
  final int tti;

  Map<String, Object?> toJson() => {
    'type': 'kcp',
    'seed': seed,
    'mtu': mtu,
    'tti': tti,
  };
}

class MeekConfig {
  const MeekConfig({this.path = '/', this.host});

  final String path;
  final String? host;

  Map<String, Object?> toJson() => {'type': 'meek', 'path': path, 'host': host};
}

class GdocsViewerConfig {
  const GdocsViewerConfig({this.pathPrefix = '/gdocsviewer', this.sharedKey});

  final String pathPrefix;
  final String? sharedKey;

  Map<String, Object?> toJson() => {
    'type': 'gdocsviewer',
    'path-prefix': pathPrefix,
    'shared-key': sharedKey,
  };
}

class TlsConfig {
  const TlsConfig({
    required this.serverName,
    this.insecureSkipVerify = false,
    this.alpn = const [],
  });

  final String serverName;
  final bool insecureSkipVerify;
  final List<String> alpn;

  Map<String, Object?> toJson() => {
    'type': 'tls',
    'server-name': serverName,
    'insecure-skip-verify': insecureSkipVerify,
    'alpn': alpn,
  };
}

class RealityConfig {
  const RealityConfig({
    required this.serverName,
    required this.publicKey,
    required this.shortId,
    this.rawPubkey = '',
  });

  final String serverName;
  final String publicKey;
  final String shortId;
  final String rawPubkey;

  Map<String, Object?> toJson() => {
    'type': 'reality',
    'server-name': serverName,
    'public-key': publicKey,
    'short-id': shortId,
    'raw-pubkey': rawPubkey,
  };
}

class AnyTlsConfig {
  const AnyTlsConfig({
    required this.serverName,
    required this.password,
    this.insecureSkipVerify = true,
    this.alpn = const [],
  });

  final String serverName;
  final String password;
  final bool insecureSkipVerify;
  final List<String> alpn;

  Map<String, Object?> toJson() => {
    'type': 'any-tls',
    'server-name': serverName,
    'password': password,
    'insecure-skip-verify': insecureSkipVerify,
    'alpn': alpn,
  };
}

class ShadowTlsConfig {
  const ShadowTlsConfig({required this.serverName, required this.password});

  final String serverName;
  final String password;

  Map<String, Object?> toJson() => {
    'type': 'shadowtls',
    'server-name': serverName,
    'password': password,
  };
}

class EndpointConfig {
  const EndpointConfig({
    required this.proxy,
    required this.transport,
    required this.outerSecurity,
  });

  final Map<String, Object?> proxy;
  final Map<String, Object?> transport;
  final Map<String, Object?> outerSecurity;

  Map<String, Object?> toJson() => {
    'proxy': proxy,
    'transport': transport,
    'outer-security': outerSecurity,
  };
}

class ClientConfigInput {
  const ClientConfigInput({
    required this.serverHost,
    required this.serverPort,
    required this.localHost,
    required this.localPort,
    required this.endpoint,
  });

  final String serverHost;
  final int serverPort;
  final String localHost;
  final int localPort;
  final EndpointConfig endpoint;

  Map<String, Object?> toJson() => {
    'server': {'host': serverHost, 'port': serverPort, ...endpoint.toJson()},
    'local': {'host': localHost, 'port': localPort},
  };

  String toJsonString() => jsonEncode(toJson());
}

class ProbeRequest {
  const ProbeRequest({
    required this.config,
    required this.targetHost,
    required this.targetPort,
    required this.payload,
  });

  final ClientConfigInput config;
  final String targetHost;
  final int targetPort;
  final String payload;
}

class WrongsvAdaptRequest {
  const WrongsvAdaptRequest({
    required this.path,
    required this.serverHost,
    this.listenHost = '127.0.0.1',
    this.listenPort = 1080,
  });

  final String path;
  final String serverHost;
  final String listenHost;
  final int listenPort;
}

class WrongsvMissingField {
  const WrongsvMissingField({required this.field, required this.reason});

  final String field;
  final String reason;

  factory WrongsvMissingField.fromMap(Map<String, Object?> map) {
    return WrongsvMissingField(
      field: map['field'] as String? ?? '',
      reason: map['reason'] as String? ?? '',
    );
  }

  Map<String, Object?> toMap() => {'field': field, 'reason': reason};
}

class WrongsvProfileSupport {
  const WrongsvProfileSupport({
    required this.profile,
    required this.displayName,
    required this.implemented,
    required this.support,
    required this.active,
    required this.reason,
  });

  final String profile;
  final String displayName;
  final bool implemented;
  final String support;
  final bool active;
  final String reason;

  factory WrongsvProfileSupport.fromMap(Map<String, Object?> map) {
    return WrongsvProfileSupport(
      profile: map['profile'] as String? ?? '',
      displayName: map['display_name'] as String? ?? '',
      implemented: map['implemented'] == true,
      support: map['support'] as String? ?? 'unsupported',
      active: map['active'] == true,
      reason: map['reason'] as String? ?? '',
    );
  }

  Map<String, Object?> toMap() => {
    'profile': profile,
    'display_name': displayName,
    'implemented': implemented,
    'support': support,
    'active': active,
    'reason': reason,
  };
}

class WrongsvCapabilityReport {
  const WrongsvCapabilityReport({
    required this.activeProfile,
    required this.listen,
    required this.listenPort,
    required this.payloadNetworks,
    required this.baseCarriers,
    required this.activeSupport,
    required this.activeReason,
    required this.missingFields,
    required this.profiles,
  });

  final String activeProfile;
  final String listen;
  final int listenPort;
  final List<String> payloadNetworks;
  final List<String> baseCarriers;
  final String activeSupport;
  final String activeReason;
  final List<WrongsvMissingField> missingFields;
  final List<WrongsvProfileSupport> profiles;

  factory WrongsvCapabilityReport.fromMap(Map<String, Object?> map) {
    final missingFields = (map['missing_fields'] as List? ?? const [])
        .map(
          (value) => WrongsvMissingField.fromMap(
            Map<String, Object?>.from(value as Map),
          ),
        )
        .toList();
    final profiles = (map['profiles'] as List? ?? const [])
        .map(
          (value) => WrongsvProfileSupport.fromMap(
            Map<String, Object?>.from(value as Map),
          ),
        )
        .toList();
    return WrongsvCapabilityReport(
      activeProfile: map['active_profile'] as String? ?? '',
      listen: map['listen'] as String? ?? '',
      listenPort: (map['listen_port'] as num?)?.toInt() ?? 0,
      payloadNetworks: (map['payload_networks'] as List? ?? const [])
          .map((value) => '$value')
          .toList(),
      baseCarriers: (map['base_carriers'] as List? ?? const [])
          .map((value) => '$value')
          .toList(),
      activeSupport: map['active_support'] as String? ?? 'unsupported',
      activeReason: map['active_reason'] as String? ?? '',
      missingFields: missingFields,
      profiles: profiles,
    );
  }

  Map<String, Object?> toMap() => {
    'active_profile': activeProfile,
    'listen': listen,
    'listen_port': listenPort,
    'payload_networks': payloadNetworks,
    'base_carriers': baseCarriers,
    'active_support': activeSupport,
    'active_reason': activeReason,
    'missing_fields': [for (final field in missingFields) field.toMap()],
    'profiles': [for (final profile in profiles) profile.toMap()],
  };
}

class WrongsvAdaptResult {
  const WrongsvAdaptResult({
    required this.report,
    required this.config,
    required this.draftConfig,
    required this.stackSummary,
  });

  final WrongsvCapabilityReport report;
  final Map<String, Object?>? config;
  final Map<String, Object?>? draftConfig;
  final String stackSummary;

  Map<String, Object?>? get effectiveConfig => config ?? draftConfig;

  factory WrongsvAdaptResult.fromMap(Map<String, Object?> map) {
    final config = map['config'];
    final draftConfig = map['draft_config'];
    return WrongsvAdaptResult(
      report: WrongsvCapabilityReport.fromMap(
        Map<String, Object?>.from(map['report'] as Map? ?? const {}),
      ),
      config: config is Map ? Map<String, Object?>.from(config) : null,
      draftConfig: draftConfig is Map
          ? Map<String, Object?>.from(draftConfig)
          : null,
      stackSummary: map['stack_summary'] as String? ?? '',
    );
  }
}

class NativeResponse {
  const NativeResponse({
    required this.ok,
    required this.message,
    required this.data,
  });

  final bool ok;
  final String message;
  final Map<String, Object?> data;

  factory NativeResponse.fromJsonString(String raw) {
    final decoded = jsonDecode(raw);
    if (decoded is! Map<String, Object?>) {
      return NativeResponse(ok: false, message: raw, data: const {});
    }

    final data = decoded['data'];
    return NativeResponse(
      ok: decoded['ok'] == true,
      message: decoded['message'] as String? ?? raw,
      data: data is Map<String, Object?>
          ? data
          : Map<String, Object?>.from(data as Map? ?? const {}),
    );
  }
}

abstract interface class WrongclClient {
  NativeResponse version();

  NativeResponse startProxy(ClientConfigInput config);

  NativeResponse stopProxy();

  NativeResponse status();

  NativeResponse probe(ProbeRequest request);

  NativeResponse stackSummary(ClientConfigInput config);

  NativeResponse validateConfig(ClientConfigInput config);

  NativeResponse loadClientConfigFile(String path);

  NativeResponse exportConfigToml(ClientConfigInput config);

  NativeResponse inspectWrongsvConfig(String path);

  NativeResponse adaptWrongsvConfig(WrongsvAdaptRequest request);
}

typedef _NativeNoArg = Pointer<Utf8> Function();
typedef _DartNoArg = Pointer<Utf8> Function();

typedef _NativeJsonOnly = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _DartJsonOnly = Pointer<Utf8> Function(Pointer<Utf8>);

typedef _NativeProbeJson =
    Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Uint16, Pointer<Utf8>);
typedef _DartProbeJson =
    Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, int, Pointer<Utf8>);

typedef _NativeAdaptWrongsv =
    Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, Uint16);
typedef _DartAdaptWrongsv =
    Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, int);

typedef _NativeFree = Void Function(Pointer<Utf8>);
typedef _DartFree = void Function(Pointer<Utf8>);

class NativeWrongclClient implements WrongclClient {
  NativeWrongclClient({DynamicLibrary? library})
    : _library = library ?? _openLibrary() {
    _version = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_native_version',
    );
    _startJson = _library.lookupFunction<_NativeJsonOnly, _DartJsonOnly>(
      'wrongcl_start_proxy_json',
    );
    _stop = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_stop_proxy',
    );
    _status = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_proxy_status',
    );
    _probeJson = _library.lookupFunction<_NativeProbeJson, _DartProbeJson>(
      'wrongcl_probe_json',
    );
    _stackJson = _library.lookupFunction<_NativeJsonOnly, _DartJsonOnly>(
      'wrongcl_stack_summary_json',
    );
    _validateConfigJson = _library
        .lookupFunction<_NativeJsonOnly, _DartJsonOnly>(
          'wrongcl_validate_config_json',
        );
    _loadConfigFileJson = _library
        .lookupFunction<_NativeJsonOnly, _DartJsonOnly>(
          'wrongcl_load_config_file_json',
        );
    _exportConfigTomlJson = _library
        .lookupFunction<_NativeJsonOnly, _DartJsonOnly>(
          'wrongcl_export_config_toml_json',
        );
    _capabilitiesJson = _library.lookupFunction<_NativeJsonOnly, _DartJsonOnly>(
      'wrongcl_capabilities_json',
    );
    _adaptWrongsvJson = _library
        .lookupFunction<_NativeAdaptWrongsv, _DartAdaptWrongsv>(
          'wrongcl_adapt_wrongsv_config_json',
        );
    _free = _library.lookupFunction<_NativeFree, _DartFree>(
      'wrongcl_free_string',
    );
  }

  final DynamicLibrary _library;
  late final _DartNoArg _version;
  late final _DartJsonOnly _startJson;
  late final _DartNoArg _stop;
  late final _DartNoArg _status;
  late final _DartProbeJson _probeJson;
  late final _DartJsonOnly _stackJson;
  late final _DartJsonOnly _validateConfigJson;
  late final _DartJsonOnly _loadConfigFileJson;
  late final _DartJsonOnly _exportConfigTomlJson;
  late final _DartJsonOnly _capabilitiesJson;
  late final _DartAdaptWrongsv _adaptWrongsvJson;
  late final _DartFree _free;

  static DynamicLibrary _openLibrary() {
    if (Platform.isLinux) {
      return _openFromCandidates(['libwrongcl_native.so']);
    }
    if (Platform.isMacOS) {
      return _openFromCandidates([
        'libwrongcl_native.dylib',
        '${File(Platform.resolvedExecutable).parent.path}/libwrongcl_native.dylib',
        '${File(Platform.resolvedExecutable).parent.parent.path}/Frameworks/libwrongcl_native.dylib',
      ]);
    }
    if (Platform.isWindows) {
      return _openFromCandidates([
        'wrongcl_native.dll',
        '${File(Platform.resolvedExecutable).parent.path}\\wrongcl_native.dll',
      ]);
    }
    throw UnsupportedError(
      'wrongcl native library is not bundled for this platform',
    );
  }

  static DynamicLibrary _openFromCandidates(List<String> candidates) {
    Object? lastError;
    for (final candidate in candidates) {
      try {
        return DynamicLibrary.open(candidate);
      } catch (error) {
        lastError = error;
      }
    }
    throw UnsupportedError(
      'failed to load wrongcl native library from ${candidates.join(', ')}: $lastError',
    );
  }

  @override
  NativeResponse version() => _take(_version());

  @override
  NativeResponse startProxy(ClientConfigInput config) {
    return _withJson(config.toJsonString(), (ptr) => _take(_startJson(ptr)));
  }

  @override
  NativeResponse stopProxy() => _take(_stop());

  @override
  NativeResponse status() => _take(_status());

  @override
  NativeResponse probe(ProbeRequest request) {
    return _withJson(request.config.toJsonString(), (configPtr) {
      final targetPtr = request.targetHost.toNativeUtf8();
      final payloadPtr = request.payload.toNativeUtf8();
      try {
        return _take(
          _probeJson(configPtr, targetPtr, request.targetPort, payloadPtr),
        );
      } finally {
        calloc.free(targetPtr);
        calloc.free(payloadPtr);
      }
    });
  }

  @override
  NativeResponse stackSummary(ClientConfigInput config) {
    return _withJson(config.toJsonString(), (ptr) => _take(_stackJson(ptr)));
  }

  @override
  NativeResponse validateConfig(ClientConfigInput config) {
    return _withJson(
      config.toJsonString(),
      (ptr) => _take(_validateConfigJson(ptr)),
    );
  }

  @override
  NativeResponse loadClientConfigFile(String path) {
    final ptr = path.toNativeUtf8();
    try {
      return _take(_loadConfigFileJson(ptr));
    } finally {
      calloc.free(ptr);
    }
  }

  @override
  NativeResponse exportConfigToml(ClientConfigInput config) {
    return _withJson(
      config.toJsonString(),
      (ptr) => _take(_exportConfigTomlJson(ptr)),
    );
  }

  @override
  NativeResponse inspectWrongsvConfig(String path) {
    final ptr = path.toNativeUtf8();
    try {
      return _take(_capabilitiesJson(ptr));
    } finally {
      calloc.free(ptr);
    }
  }

  @override
  NativeResponse adaptWrongsvConfig(WrongsvAdaptRequest request) {
    final pathPtr = request.path.toNativeUtf8();
    final serverPtr = request.serverHost.toNativeUtf8();
    final listenPtr = request.listenHost.toNativeUtf8();
    try {
      return _take(
        _adaptWrongsvJson(pathPtr, serverPtr, listenPtr, request.listenPort),
      );
    } finally {
      calloc.free(pathPtr);
      calloc.free(serverPtr);
      calloc.free(listenPtr);
    }
  }

  NativeResponse _take(Pointer<Utf8> ptr) {
    if (ptr == nullptr) {
      return const NativeResponse(
        ok: false,
        message: 'native call returned null',
        data: {},
      );
    }

    try {
      return NativeResponse.fromJsonString(ptr.toDartString());
    } finally {
      _free(ptr);
    }
  }

  NativeResponse _withJson(
    String json,
    NativeResponse Function(Pointer<Utf8>) run,
  ) {
    final ptr = json.toNativeUtf8();
    try {
      return run(ptr);
    } finally {
      calloc.free(ptr);
    }
  }
}
