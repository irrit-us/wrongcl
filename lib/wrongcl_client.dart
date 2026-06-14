import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

class ClientSettings {
  const ClientSettings({
    required this.serverHost,
    required this.serverPort,
    required this.uuid,
    required this.localHost,
    required this.localPort,
    required this.targetHost,
    required this.targetPort,
    required this.payload,
  });

  final String serverHost;
  final int serverPort;
  final String uuid;
  final String localHost;
  final int localPort;
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

  NativeResponse startProxy(ClientSettings settings);

  NativeResponse stopProxy();

  NativeResponse status();

  NativeResponse probe(ClientSettings settings);
}

typedef _NativeVersion = Pointer<Utf8> Function();
typedef _DartVersion = Pointer<Utf8> Function();

typedef _NativeStart =
    Pointer<Utf8> Function(
      Pointer<Utf8>,
      Uint16,
      Pointer<Utf8>,
      Pointer<Utf8>,
      Uint16,
    );
typedef _DartStart =
    Pointer<Utf8> Function(
      Pointer<Utf8>,
      int,
      Pointer<Utf8>,
      Pointer<Utf8>,
      int,
    );

typedef _NativeStop = Pointer<Utf8> Function();
typedef _DartStop = Pointer<Utf8> Function();

typedef _NativeStatus = Pointer<Utf8> Function();
typedef _DartStatus = Pointer<Utf8> Function();

typedef _NativeProbe =
    Pointer<Utf8> Function(
      Pointer<Utf8>,
      Uint16,
      Pointer<Utf8>,
      Pointer<Utf8>,
      Uint16,
      Pointer<Utf8>,
    );
typedef _DartProbe =
    Pointer<Utf8> Function(
      Pointer<Utf8>,
      int,
      Pointer<Utf8>,
      Pointer<Utf8>,
      int,
      Pointer<Utf8>,
    );

typedef _NativeFree = Void Function(Pointer<Utf8>);
typedef _DartFree = void Function(Pointer<Utf8>);

class NativeWrongclClient implements WrongclClient {
  NativeWrongclClient({DynamicLibrary? library})
    : _library = library ?? _openLibrary() {
    _version = _library.lookupFunction<_NativeVersion, _DartVersion>(
      'wrongcl_native_version',
    );
    _start = _library.lookupFunction<_NativeStart, _DartStart>(
      'wrongcl_start_proxy',
    );
    _stop = _library.lookupFunction<_NativeStop, _DartStop>(
      'wrongcl_stop_proxy',
    );
    _status = _library.lookupFunction<_NativeStatus, _DartStatus>(
      'wrongcl_proxy_status',
    );
    _probe = _library.lookupFunction<_NativeProbe, _DartProbe>('wrongcl_probe');
    _free = _library.lookupFunction<_NativeFree, _DartFree>(
      'wrongcl_free_string',
    );
  }

  final DynamicLibrary _library;
  late final _DartVersion _version;
  late final _DartStart _start;
  late final _DartStop _stop;
  late final _DartStatus _status;
  late final _DartProbe _probe;
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
  NativeResponse startProxy(ClientSettings settings) {
    return _withUtf8(
      [settings.serverHost, settings.uuid, settings.localHost],
      (args) => _take(
        _start(
          args[0],
          settings.serverPort,
          args[1],
          args[2],
          settings.localPort,
        ),
      ),
    );
  }

  @override
  NativeResponse stopProxy() => _take(_stop());

  @override
  NativeResponse status() => _take(_status());

  @override
  NativeResponse probe(ClientSettings settings) {
    return _withUtf8(
      [
        settings.serverHost,
        settings.uuid,
        settings.targetHost,
        settings.payload,
      ],
      (args) => _take(
        _probe(
          args[0],
          settings.serverPort,
          args[1],
          args[2],
          settings.targetPort,
          args[3],
        ),
      ),
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

  T _withUtf8<T>(List<String> values, T Function(List<Pointer<Utf8>>) run) {
    final pointers = <Pointer<Utf8>>[];
    try {
      for (final value in values) {
        pointers.add(value.toNativeUtf8());
      }
      return run(pointers);
    } finally {
      for (final pointer in pointers) {
        calloc.free(pointer);
      }
    }
  }
}
