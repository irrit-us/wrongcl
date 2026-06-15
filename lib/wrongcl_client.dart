import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

enum ProxyKind {
  vless('vless', 'VLESS'),
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
  anytls('anytls', 'AnyTLS');

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

  Map<String, Object?> toJson() => {
    'type': 'trojan',
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
    'server': {
      'host': serverHost,
      'port': serverPort,
      ...endpoint.toJson(),
    },
    'local': {
      'host': localHost,
      'port': localPort,
    },
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
}

typedef _NativeNoArg = Pointer<Utf8> Function();
typedef _DartNoArg = Pointer<Utf8> Function();

typedef _NativeJsonOnly = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _DartJsonOnly = Pointer<Utf8> Function(Pointer<Utf8>);

typedef _NativeProbeJson =
    Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Uint16, Pointer<Utf8>);
typedef _DartProbeJson =
    Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, int, Pointer<Utf8>);

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
  late final _DartFree _free;

  static DynamicLibrary _openLibrary() {
    if (Platform.isLinux) {
      return DynamicLibrary.open('libwrongcl_native.so');
    }
    throw UnsupportedError('wrongcl native library is only bundled for Linux');
  }

  @override
  NativeResponse version() => _take(_version());

  @override
  NativeResponse startProxy(ClientConfigInput config) {
    return _withJson(
      config.toJsonString(),
      (ptr) => _take(_startJson(ptr)),
    );
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
    return _withJson(
      config.toJsonString(),
      (ptr) => _take(_stackJson(ptr)),
    );
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
