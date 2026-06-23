import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

enum ProxyKind {
  vless('vless', 'VLESS'),
  naive('naive', 'Naive'),
  hysteria2('hysteria2', 'Hysteria2'),
  tuic('tuic', 'TUIC'),
  trojan('trojan', 'Trojan'),
  mixed('mixed', 'Mixed remote SOCKS/HTTP'),
  shadowsocks('shadowsocks', 'Shadowsocks'),
  wireguard('wireguard', 'WireGuard');

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

class NaiveConfig {
  const NaiveConfig({
    required this.username,
    required this.password,
    this.paddingHeaderName = 'Padding',
  });

  final String username;
  final String password;
  final String paddingHeaderName;

  Map<String, Object?> toJson() => {
    'type': 'naive',
    'username': username,
    'password': password,
    'padding-header-name': paddingHeaderName,
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
    this.obfsType,
    this.obfsPassword,
    this.obfsMinPacketSize,
    this.obfsMaxPacketSize,
  });

  final String serverName;
  final String password;
  final bool udpEnabled;
  final String? obfsType;
  final String? obfsPassword;
  final int? obfsMinPacketSize;
  final int? obfsMaxPacketSize;

  Map<String, Object?> toJson() => {
    'type': 'hysteria2',
    'server-name': serverName,
    'password': password,
    'udp-enabled': udpEnabled,
    'obfs-type': obfsType,
    'obfs-password': obfsPassword,
    'obfs-min-packet-size': obfsMinPacketSize,
    'obfs-max-packet-size': obfsMaxPacketSize,
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

class WireGuardConfig {
  const WireGuardConfig({
    required this.privateKey,
    required this.peerPublicKey,
    required this.clientIp,
    required this.allowedIps,
    this.preSharedKey,
    this.mtu = 1400,
  });

  final String privateKey;
  final String peerPublicKey;
  final String? preSharedKey;
  final String clientIp;
  final List<String> allowedIps;
  final int mtu;

  Map<String, Object?> toJson() => {
    'type': 'wireguard',
    'private-key': privateKey,
    'peer-public-key': peerPublicKey,
    'pre-shared-key': preSharedKey,
    'client-ip': clientIp,
    'allowed-ips': allowedIps,
    'mtu': mtu,
  };
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

  factory EndpointConfig.fromMap(Map<String, Object?> map) {
    return EndpointConfig(
      proxy: Map<String, Object?>.from(map['proxy'] as Map? ?? const {}),
      transport: Map<String, Object?>.from(
        map['transport'] as Map? ?? const {},
      ),
      outerSecurity: Map<String, Object?>.from(
        map['outer-security'] as Map? ?? const {},
      ),
    );
  }

  Map<String, Object?> toJson() => {
    'proxy': proxy,
    'transport': transport,
    'outer-security': outerSecurity,
  };
}

class NamedEndpointInput {
  const NamedEndpointInput({
    required this.name,
    required this.host,
    required this.port,
    required this.endpoint,
  });

  final String name;
  final String host;
  final int port;
  final EndpointConfig endpoint;

  factory NamedEndpointInput.fromMap(Map<String, Object?> map) {
    return NamedEndpointInput(
      name: map['name'] as String? ?? '',
      host: map['host'] as String? ?? '',
      port: (map['port'] as num?)?.toInt() ?? 0,
      endpoint: EndpointConfig.fromMap(map),
    );
  }

  Map<String, Object?> toJson() => {
    'name': name,
    'host': host,
    'port': port,
    ...endpoint.toJson(),
  };
}

enum ProxyGroupKind {
  select('select', 'Select'),
  fallback('fallback', 'Fallback'),
  urlTest('url-test', 'URL test');

  const ProxyGroupKind(this.id, this.label);
  final String id;
  final String label;

  static ProxyGroupKind fromId(String id) =>
      values.firstWhere((value) => value.id == id, orElse: () => select);
}

class ProxyGroupInput {
  const ProxyGroupInput({
    required this.name,
    required this.kind,
    required this.members,
    this.selected,
  });

  final String name;
  final ProxyGroupKind kind;
  final List<String> members;
  final String? selected;

  factory ProxyGroupInput.fromMap(Map<String, Object?> map) {
    return ProxyGroupInput(
      name: map['name'] as String? ?? '',
      kind: ProxyGroupKind.fromId(map['kind'] as String? ?? 'select'),
      members: [
        for (final member in (map['members'] as List? ?? const [])) '$member',
      ],
      selected: map['selected'] as String?,
    );
  }

  Map<String, Object?> toJson() => {
    'name': name,
    'kind': kind.id,
    'members': members,
    if (selected != null) 'selected': selected,
  };
}

class ActiveSelectionInput {
  const ActiveSelectionInput._({required this.kind, required this.name});
  const ActiveSelectionInput.endpoint(String name)
    : this._(kind: 'endpoint', name: name);
  const ActiveSelectionInput.group(String name)
    : this._(kind: 'group', name: name);

  final String kind;
  final String name;

  factory ActiveSelectionInput.fromMap(Map<String, Object?> map) {
    final kind = map['type'] as String? ?? 'endpoint';
    final name = map['name'] as String? ?? '';
    return kind == 'group'
        ? ActiveSelectionInput.group(name)
        : ActiveSelectionInput.endpoint(name);
  }

  Map<String, Object?> toJson() => {'type': kind, 'name': name};
}

enum DnsBackendKind {
  system('system', 'System'),
  udp('udp', 'UDP'),
  doh('doh', 'DoH');

  const DnsBackendKind(this.id, this.label);

  final String id;
  final String label;

  static DnsBackendKind fromId(String id) {
    return values.firstWhere((value) => value.id == id, orElse: () => system);
  }
}

class DnsSettingsInput {
  const DnsSettingsInput({required this.kind, this.server, this.url});

  const DnsSettingsInput.system() : this(kind: DnsBackendKind.system);

  final DnsBackendKind kind;
  final String? server;
  final String? url;

  factory DnsSettingsInput.fromMap(Map<String, Object?> map) {
    final legacyMode = map['mode'] as String?;
    if (legacyMode != null && legacyMode.isNotEmpty) {
      return DnsSettingsInput(kind: DnsBackendKind.fromId(legacyMode));
    }

    final backendRaw = map['backend'];
    final backend = backendRaw is Map
        ? Map<String, Object?>.from(backendRaw)
        : map;
    final kind = DnsBackendKind.fromId(backend['kind'] as String? ?? 'system');
    return DnsSettingsInput(
      kind: kind,
      server: backend['server'] as String?,
      url: backend['url'] as String?,
    );
  }

  DnsSettingsInput normalized() {
    final rawServer = server?.trim() ?? '';
    final rawUrl = url?.trim() ?? '';
    final normalizedServer = rawServer.startsWith('udp://')
        ? rawServer.substring('udp://'.length)
        : rawServer;
    return DnsSettingsInput(
      kind: kind,
      server: normalizedServer.isEmpty ? null : normalizedServer,
      url: rawUrl.isEmpty ? null : rawUrl,
    );
  }

  String? validateMessage() {
    final value = normalized();
    switch (value.kind) {
      case DnsBackendKind.system:
        return null;
      case DnsBackendKind.udp:
        final server = value.server;
        if (server == null || server.isEmpty) {
          return 'UDP resolver requires a server address.';
        }
        if (!server.contains(':')) {
          return 'UDP resolver must look like udp://1.1.1.1:53.';
        }
        return null;
      case DnsBackendKind.doh:
        final url = value.url;
        if (url == null || url.isEmpty) {
          return 'DoH resolver requires an HTTPS URL.';
        }
        if (!url.startsWith('https://')) {
          return 'DoH resolver URL must start with https://.';
        }
        return null;
    }
  }

  Map<String, Object?> toMap() {
    final value = normalized();
    switch (value.kind) {
      case DnsBackendKind.system:
        return {
          'backend': {'kind': 'system'},
        };
      case DnsBackendKind.udp:
        return {
          'backend': {'kind': 'udp', 'server': value.server},
        };
      case DnsBackendKind.doh:
        return {
          'backend': {'kind': 'doh', 'url': value.url},
        };
    }
  }
}

const List<RouterMode> kBuiltinRouterModes = [
  RouterMode(name: 'global', kind: 'global'),
  RouterMode(name: 'rule', kind: 'rule'),
  RouterMode(name: 'direct', kind: 'direct'),
];

class ClientConfigInput {
  const ClientConfigInput({
    required this.endpoints,
    this.groups = const [],
    required this.active,
    required this.localHost,
    required this.localPort,
    this.allowSocks = true,
    this.allowHttp = true,
    this.scripts = const [],
    this.modes = kBuiltinRouterModes,
    this.activeMode = 'global',
    this.dns = const {},
  });

  final List<NamedEndpointInput> endpoints;
  final List<ProxyGroupInput> groups;
  final ActiveSelectionInput active;
  final String localHost;
  final int localPort;
  final bool allowSocks;
  final bool allowHttp;
  final List<RouterScript> scripts;
  final List<RouterMode> modes;
  final String? activeMode;
  final Map<String, Object?> dns;

  factory ClientConfigInput.fromMap(Map<String, Object?> map) {
    final endpoints = (map['endpoints'] as List? ?? const [])
        .whereType<Map>()
        .map(
          (entry) =>
              NamedEndpointInput.fromMap(Map<String, Object?>.from(entry)),
        )
        .toList(growable: false);
    final groups = (map['groups'] as List? ?? const [])
        .whereType<Map>()
        .map(
          (entry) => ProxyGroupInput.fromMap(Map<String, Object?>.from(entry)),
        )
        .toList(growable: false);
    final scripts = (map['scripts'] as List? ?? const [])
        .whereType<Map>()
        .map((entry) => RouterScript.fromMap(Map<String, Object?>.from(entry)))
        .toList(growable: false);
    final parsedModes = (map['modes'] as List? ?? const [])
        .whereType<Map>()
        .map((entry) => RouterMode.fromMap(Map<String, Object?>.from(entry)))
        .toList(growable: false);
    final local = Map<String, Object?>.from(map['local'] as Map? ?? const {});
    final activeRaw = map['active'];
    return ClientConfigInput(
      endpoints: endpoints,
      groups: groups,
      active: activeRaw is Map
          ? ActiveSelectionInput.fromMap(Map<String, Object?>.from(activeRaw))
          : const ActiveSelectionInput.endpoint('default'),
      localHost: local['host'] as String? ?? '127.0.0.1',
      localPort: (local['port'] as num?)?.toInt() ?? 1080,
      allowSocks: local['allow_socks'] != false,
      allowHttp: local['allow_http'] != false,
      scripts: scripts,
      modes: parsedModes.isEmpty ? kBuiltinRouterModes : parsedModes,
      activeMode: map['active_mode'] as String? ?? 'global',
      dns: Map<String, Object?>.from(map['dns'] as Map? ?? const {}),
    );
  }

  ClientConfigInput copyWith({
    List<NamedEndpointInput>? endpoints,
    List<ProxyGroupInput>? groups,
    ActiveSelectionInput? active,
    String? localHost,
    int? localPort,
    bool? allowSocks,
    bool? allowHttp,
    List<RouterScript>? scripts,
    List<RouterMode>? modes,
    String? activeMode,
    Map<String, Object?>? dns,
  }) {
    return ClientConfigInput(
      endpoints: endpoints ?? this.endpoints,
      groups: groups ?? this.groups,
      active: active ?? this.active,
      localHost: localHost ?? this.localHost,
      localPort: localPort ?? this.localPort,
      allowSocks: allowSocks ?? this.allowSocks,
      allowHttp: allowHttp ?? this.allowHttp,
      scripts: scripts ?? this.scripts,
      modes: modes ?? this.modes,
      activeMode: activeMode ?? this.activeMode,
      dns: dns ?? this.dns,
    );
  }

  Map<String, Object?> toJson() => {
    'endpoints': [for (final e in endpoints) e.toJson()],
    if (groups.isNotEmpty) 'groups': [for (final g in groups) g.toJson()],
    if (scripts.isNotEmpty) 'scripts': [for (final s in scripts) s.toMap()],
    if (modes.isNotEmpty) 'modes': [for (final m in modes) m.toMap()],
    if (activeMode != null) 'active_mode': activeMode,
    'active': active.toJson(),
    if (dns.isNotEmpty) 'dns': dns,
    'local': {
      'host': localHost,
      'port': localPort,
      'allow_socks': allowSocks,
      'allow_http': allowHttp,
    },
  };

  String toJsonString() => jsonEncode(toJson());
}

class ProxyEndpointInfo {
  const ProxyEndpointInfo({
    required this.name,
    required this.host,
    required this.port,
    required this.stack,
    required this.proxy,
    required this.transport,
    required this.outerSecurity,
  });

  final String name;
  final String host;
  final int port;
  final String stack;
  final String proxy;
  final String transport;
  final String outerSecurity;

  factory ProxyEndpointInfo.fromMap(Map<String, Object?> map) {
    return ProxyEndpointInfo(
      name: map['name'] as String? ?? '',
      host: map['host'] as String? ?? '',
      port: (map['port'] as num?)?.toInt() ?? 0,
      stack: map['stack'] as String? ?? '',
      proxy: map['proxy'] as String? ?? '',
      transport: map['transport'] as String? ?? '',
      outerSecurity: map['outer_security'] as String? ?? '',
    );
  }

  factory ProxyEndpointInfo.fromInput(NamedEndpointInput input) {
    final proxy = input.endpoint.proxy['type'] as String? ?? '';
    final transport = input.endpoint.transport['type'] as String? ?? '';
    final outer = input.endpoint.outerSecurity['type'] as String? ?? '';
    return ProxyEndpointInfo(
      name: input.name,
      host: input.host,
      port: input.port,
      stack: '',
      proxy: proxy,
      transport: transport,
      outerSecurity: outer,
    );
  }
}

class ProxyGroupInfo {
  const ProxyGroupInfo({
    required this.name,
    required this.kind,
    required this.members,
    this.selected,
  });

  final String name;
  final ProxyGroupKind kind;
  final List<String> members;
  final String? selected;

  factory ProxyGroupInfo.fromMap(Map<String, Object?> map) {
    return ProxyGroupInfo(
      name: map['name'] as String? ?? '',
      kind: ProxyGroupKind.fromId(map['kind'] as String? ?? 'select'),
      members: [for (final m in (map['members'] as List? ?? const [])) '$m'],
      selected: map['selected'] as String?,
    );
  }
}

class ProxyActiveInfo {
  const ProxyActiveInfo({required this.kind, required this.name});
  final String kind;
  final String name;

  factory ProxyActiveInfo.fromMap(Map<String, Object?> map) {
    return ProxyActiveInfo(
      kind: map['type'] as String? ?? 'endpoint',
      name: map['name'] as String? ?? '',
    );
  }
}

class ProxyGroupsSnapshot {
  const ProxyGroupsSnapshot({
    required this.endpoints,
    required this.groups,
    this.active,
  });

  final List<ProxyEndpointInfo> endpoints;
  final List<ProxyGroupInfo> groups;
  final ProxyActiveInfo? active;

  factory ProxyGroupsSnapshot.fromMap(Map<String, Object?> map) {
    final endpoints = (map['endpoints'] as List? ?? const [])
        .whereType<Map>()
        .map((e) => ProxyEndpointInfo.fromMap(Map<String, Object?>.from(e)))
        .toList(growable: false);
    final groups = (map['groups'] as List? ?? const [])
        .whereType<Map>()
        .map((g) => ProxyGroupInfo.fromMap(Map<String, Object?>.from(g)))
        .toList(growable: false);
    final activeRaw = map['active'];
    final active = activeRaw is Map
        ? ProxyActiveInfo.fromMap(Map<String, Object?>.from(activeRaw))
        : null;
    return ProxyGroupsSnapshot(
      endpoints: endpoints,
      groups: groups,
      active: active,
    );
  }

  factory ProxyGroupsSnapshot.fromConfig(ClientConfigInput config) {
    final active = ProxyActiveInfo(
      kind: config.active.kind,
      name: config.active.name,
    );
    return ProxyGroupsSnapshot(
      endpoints: [
        for (final endpoint in config.endpoints)
          ProxyEndpointInfo.fromInput(endpoint),
      ],
      groups: [
        for (final group in config.groups)
          ProxyGroupInfo(
            name: group.name,
            kind: group.kind,
            members: group.members,
            selected: group.selected,
          ),
      ],
      active: active.name.isEmpty ? null : active,
    );
  }

  static const empty = ProxyGroupsSnapshot(endpoints: [], groups: []);
}

class RouterRule {
  const RouterRule({
    required this.kind,
    required this.action,
    this.value,
    this.cidr,
    this.country,
    this.proxyName,
  });

  final String kind;
  final String action;
  final String? value;
  final String? cidr;
  final String? country;
  final String? proxyName;

  factory RouterRule.fromMap(Map<String, Object?> map) {
    return RouterRule(
      kind: map['kind'] as String? ?? '',
      action: map['action'] as String? ?? '',
      value: map['value'] as String?,
      cidr: map['cidr'] as String?,
      country: map['country'] as String?,
      proxyName: map['name'] as String?,
    );
  }

  Map<String, Object?> toMap() {
    final out = <String, Object?>{'kind': kind, 'action': action};
    if (value != null) out['value'] = value;
    if (cidr != null) out['cidr'] = cidr;
    if (country != null) out['country'] = country;
    if (action == 'proxy' && proxyName != null) out['name'] = proxyName;
    return out;
  }
}

class RouterScript {
  const RouterScript({required this.name, required this.rules});

  final String name;
  final List<RouterRule> rules;

  factory RouterScript.fromMap(Map<String, Object?> map) {
    final rules = (map['rules'] as List? ?? const [])
        .whereType<Map>()
        .map((r) => RouterRule.fromMap(Map<String, Object?>.from(r)))
        .toList(growable: false);
    return RouterScript(name: map['name'] as String? ?? '', rules: rules);
  }

  Map<String, Object?> toMap() => {
    'name': name,
    'rules': rules.map((r) => r.toMap()).toList(),
  };
}

class RouterMode {
  const RouterMode({
    required this.name,
    required this.kind,
    this.proxy,
    this.script,
  });

  final String name;
  final String kind;
  final String? proxy;
  final String? script;

  factory RouterMode.fromMap(Map<String, Object?> map) {
    return RouterMode(
      name: map['name'] as String? ?? '',
      kind: map['kind'] as String? ?? '',
      proxy: map['proxy'] as String?,
      script: map['script'] as String?,
    );
  }

  Map<String, Object?> toMap() => {
    'name': name,
    'kind': kind,
    if (proxy != null) 'proxy': proxy,
    if (script != null) 'script': script,
  };
}

class RouterSnapshot {
  const RouterSnapshot({
    required this.modes,
    required this.scripts,
    this.activeMode,
  });

  final List<RouterMode> modes;
  final List<RouterScript> scripts;
  final String? activeMode;

  factory RouterSnapshot.fromMap(Map<String, Object?> map) {
    final modes = (map['modes'] as List? ?? const [])
        .whereType<Map>()
        .map((m) => RouterMode.fromMap(Map<String, Object?>.from(m)))
        .toList(growable: false);
    final scripts = (map['scripts'] as List? ?? const [])
        .whereType<Map>()
        .map((s) => RouterScript.fromMap(Map<String, Object?>.from(s)))
        .toList(growable: false);
    return RouterSnapshot(
      modes: modes,
      scripts: scripts,
      activeMode: map['active_mode'] as String?,
    );
  }

  factory RouterSnapshot.fromConfig(ClientConfigInput config) {
    return RouterSnapshot(
      modes: config.modes,
      scripts: config.scripts,
      activeMode: config.activeMode,
    );
  }

  static const empty = RouterSnapshot(modes: [], scripts: []);
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

  NativeResponse connectionsList();

  NativeResponse connectionClose(int id);

  NativeResponse connectionsCloseMatching(Map<String, Object?> filter);

  NativeResponse logsSince(int cursor);

  NativeResponse requestsSince(int cursor);

  NativeResponse proxyGroupsJson();

  NativeResponse proxyGroupSelect(String group, String member);

  NativeResponse dnsSettingsJson();

  NativeResponse dnsSettingsSet(Map<String, Object?> settings);

  NativeResponse tunStatusJson();

  NativeResponse tunEnable(Map<String, Object?> config);

  NativeResponse tunDisable();

  NativeResponse routerSnapshotJson();

  NativeResponse routerSetActiveMode(String name);

  NativeResponse routerSetScript(Map<String, Object?> script);

  NativeResponse routerRemoveScript(String name);

  NativeResponse routerUpsertUserMode(Map<String, Object?> mode);

  NativeResponse routerRemoveUserMode(String name);
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

typedef _NativeConnClose = Pointer<Utf8> Function(Uint64);
typedef _DartConnClose = Pointer<Utf8> Function(int);

typedef _NativeLogsSince = Pointer<Utf8> Function(Uint64);
typedef _DartLogsSince = Pointer<Utf8> Function(int);

typedef _NativeRequestsSince = Pointer<Utf8> Function(Uint64);
typedef _DartRequestsSince = Pointer<Utf8> Function(int);

typedef _NativeOneString = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _DartOneString = Pointer<Utf8> Function(Pointer<Utf8>);

typedef _NativeTwoStrings =
    Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef _DartTwoStrings = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

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
    _connectionsList = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_connections_list_json',
    );
    _connectionClose = _library
        .lookupFunction<_NativeConnClose, _DartConnClose>(
          'wrongcl_connection_close',
        );
    _connectionsCloseMatching = _library
        .lookupFunction<_NativeJsonOnly, _DartJsonOnly>(
          'wrongcl_connections_close_matching',
        );
    _logsSince = _library.lookupFunction<_NativeLogsSince, _DartLogsSince>(
      'wrongcl_logs_since',
    );
    _requestsSince = _library
        .lookupFunction<_NativeRequestsSince, _DartRequestsSince>(
          'wrongcl_requests_since',
        );
    _proxyGroupsJson = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_proxy_groups_json',
    );
    _proxyGroupSelect = _library
        .lookupFunction<_NativeTwoStrings, _DartTwoStrings>(
          'wrongcl_proxy_group_select',
        );
    _dnsSettingsJson = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_dns_settings_json',
    );
    _dnsSettingsSet = _library.lookupFunction<_NativeOneString, _DartOneString>(
      'wrongcl_dns_settings_set_json',
    );
    _tunStatusJson = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_tun_status_json',
    );
    _tunEnable = _library.lookupFunction<_NativeOneString, _DartOneString>(
      'wrongcl_tun_enable_json',
    );
    _tunDisable = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_tun_disable',
    );
    _routerSnapshot = _library.lookupFunction<_NativeNoArg, _DartNoArg>(
      'wrongcl_router_snapshot_json',
    );
    _routerSetActiveMode = _library
        .lookupFunction<_NativeOneString, _DartOneString>(
          'wrongcl_router_set_active_mode',
        );
    _routerSetScript = _library
        .lookupFunction<_NativeOneString, _DartOneString>(
          'wrongcl_router_set_script_json',
        );
    _routerRemoveScript = _library
        .lookupFunction<_NativeOneString, _DartOneString>(
          'wrongcl_router_remove_script',
        );
    _routerUpsertUserMode = _library
        .lookupFunction<_NativeOneString, _DartOneString>(
          'wrongcl_router_upsert_user_mode_json',
        );
    _routerRemoveUserMode = _library
        .lookupFunction<_NativeOneString, _DartOneString>(
          'wrongcl_router_remove_user_mode',
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
  late final _DartNoArg _connectionsList;
  late final _DartConnClose _connectionClose;
  late final _DartJsonOnly _connectionsCloseMatching;
  late final _DartLogsSince _logsSince;
  late final _DartRequestsSince _requestsSince;
  late final _DartNoArg _proxyGroupsJson;
  late final _DartTwoStrings _proxyGroupSelect;
  late final _DartNoArg _dnsSettingsJson;
  late final _DartOneString _dnsSettingsSet;
  late final _DartNoArg _tunStatusJson;
  late final _DartOneString _tunEnable;
  late final _DartNoArg _tunDisable;
  late final _DartNoArg _routerSnapshot;
  late final _DartOneString _routerSetActiveMode;
  late final _DartOneString _routerSetScript;
  late final _DartOneString _routerRemoveScript;
  late final _DartOneString _routerUpsertUserMode;
  late final _DartOneString _routerRemoveUserMode;
  late final _DartFree _free;

  static DynamicLibrary _openLibrary() {
    if (Platform.isAndroid) {
      return _openFromCandidates(['libwrongcl_native.so']);
    }
    if (Platform.isIOS) {
      return DynamicLibrary.process();
    }
    if (Platform.isLinux || Platform.operatingSystem == 'freebsd') {
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

  @override
  NativeResponse connectionsList() => _take(_connectionsList());

  @override
  NativeResponse connectionClose(int id) => _take(_connectionClose(id));

  @override
  NativeResponse connectionsCloseMatching(Map<String, Object?> filter) {
    return _withJson(
      jsonEncode(filter),
      (ptr) => _take(_connectionsCloseMatching(ptr)),
    );
  }

  @override
  NativeResponse logsSince(int cursor) => _take(_logsSince(cursor));

  @override
  NativeResponse requestsSince(int cursor) => _take(_requestsSince(cursor));

  @override
  NativeResponse proxyGroupsJson() => _take(_proxyGroupsJson());

  @override
  NativeResponse proxyGroupSelect(String group, String member) {
    final groupPtr = group.toNativeUtf8();
    final memberPtr = member.toNativeUtf8();
    try {
      return _take(_proxyGroupSelect(groupPtr, memberPtr));
    } finally {
      calloc.free(groupPtr);
      calloc.free(memberPtr);
    }
  }

  @override
  NativeResponse dnsSettingsJson() => _take(_dnsSettingsJson());

  @override
  NativeResponse dnsSettingsSet(Map<String, Object?> settings) {
    return _withJson(
      jsonEncode(settings),
      (ptr) => _take(_dnsSettingsSet(ptr)),
    );
  }

  @override
  NativeResponse tunStatusJson() => _take(_tunStatusJson());

  @override
  NativeResponse tunEnable(Map<String, Object?> config) {
    return _withJson(jsonEncode(config), (ptr) => _take(_tunEnable(ptr)));
  }

  @override
  NativeResponse tunDisable() => _take(_tunDisable());

  @override
  NativeResponse routerSnapshotJson() => _take(_routerSnapshot());

  @override
  NativeResponse routerSetActiveMode(String name) {
    final ptr = name.toNativeUtf8();
    try {
      return _take(_routerSetActiveMode(ptr));
    } finally {
      calloc.free(ptr);
    }
  }

  @override
  NativeResponse routerSetScript(Map<String, Object?> script) {
    return _withJson(jsonEncode(script), (ptr) => _take(_routerSetScript(ptr)));
  }

  @override
  NativeResponse routerRemoveScript(String name) {
    final ptr = name.toNativeUtf8();
    try {
      return _take(_routerRemoveScript(ptr));
    } finally {
      calloc.free(ptr);
    }
  }

  @override
  NativeResponse routerUpsertUserMode(Map<String, Object?> mode) {
    return _withJson(
      jsonEncode(mode),
      (ptr) => _take(_routerUpsertUserMode(ptr)),
    );
  }

  @override
  NativeResponse routerRemoveUserMode(String name) {
    final ptr = name.toNativeUtf8();
    try {
      return _take(_routerRemoveUserMode(ptr));
    } finally {
      calloc.free(ptr);
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
