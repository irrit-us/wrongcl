import 'package:flutter/material.dart';

import '../client_home_controller.dart';
import '../home_widgets.dart';
import '../wrongcl_client.dart';

class _EditorSubsection extends StatelessWidget {
  const _EditorSubsection({
    required this.title,
    required this.description,
    required this.child,
  });

  final String title;
  final String description;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: const Color(0xFFF8F6F1),
        borderRadius: BorderRadius.circular(18),
        border: Border.all(color: const Color(0xFFD8D1C5)),
      ),
      child: Material(
        color: Colors.transparent,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 6),
            Text(description, style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 12),
            child,
          ],
        ),
      ),
    );
  }
}

class EditorView extends StatelessWidget {
  const EditorView({super.key, required this.controller});

  final ClientHomeController controller;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 980),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              PanelIntroCard(
                title: 'Configuration Workbench',
                description:
                    'Use this focused mode to build or inspect a full client draft. Runtime controls stay on the main surface; this space is for shaping the stack itself.',
                badges: [
                  InfoBadge(label: 'Proxy', value: controller.proxyKind.label),
                  InfoBadge(
                    label: 'Transport',
                    value: controller.transportKind.label,
                    tone: const Color(0xFF2F4858),
                  ),
                  InfoBadge(
                    label: 'Outer security',
                    value: controller.outerSecurityKind.label,
                    tone: const Color(0xFF0B8A6E),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              SectionCard(
                eyebrow: 'File',
                title: 'Client Config',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    TextField(
                      key: const ValueKey('client-config-path'),
                      controller: controller.clientConfigPath,
                      decoration: const InputDecoration(
                        labelText: 'wrongcl config file path',
                      ),
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 12,
                      runSpacing: 12,
                      children: [
                        OutlinedButton.icon(
                          onPressed: controller.busy
                              ? null
                              : () => controller.runTask(
                                  'load client config',
                                  controller.loadClientConfigFile,
                                ),
                          icon: const Icon(Icons.file_open),
                          label: const Text('Load client config'),
                        ),
                        OutlinedButton.icon(
                          onPressed: controller.busy
                              ? null
                              : () => controller.runTask(
                                  'export current config',
                                  controller.exportCurrentConfigJson,
                                ),
                          icon: const Icon(Icons.download),
                          label: const Text('Export current JSON'),
                        ),
                        OutlinedButton.icon(
                          onPressed: controller.busy
                              ? null
                              : () => controller.runTask(
                                  'export current TOML',
                                  controller.exportCurrentConfigToml,
                                ),
                          icon: const Icon(Icons.description_outlined),
                          label: const Text('Export current TOML'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
              SectionCard(
                eyebrow: 'Endpoint',
                title: 'Endpoint',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const NoticeCard(
                      title: 'Draft structure',
                      message:
                          'Choose the proxy, transport, and outer security first. The editor only exposes combinations that already match current wrongcl logic.',
                      tone: Color(0xFF2F4858),
                    ),
                    const SizedBox(height: 14),
                    Wrap(
                      spacing: 12,
                      runSpacing: 12,
                      children: [
                        _proxyDropdown(),
                        _transportDropdown(),
                        _outerSecurityDropdown(),
                      ],
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 12,
                      runSpacing: 12,
                      children: [
                        _field(
                          controller.serverHost,
                          'Server host',
                          width: 300,
                        ),
                        _field(
                          controller.serverPort,
                          'Server port',
                          width: 150,
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    _EditorSubsection(
                      title: 'Proxy details',
                      description:
                          'Identity and protocol-specific values for the selected upstream proxy.',
                      child: _buildFieldGroup(_proxyFields()),
                    ),
                    const SizedBox(height: 12),
                    _EditorSubsection(
                      title: 'Transport details',
                      description: controller.transportDisabled
                          ? 'Transport selection is constrained by the currently selected proxy or security mode.'
                          : 'Network carrier behavior for the current stack.',
                      child: _buildFieldGroup(_transportFields()),
                    ),
                    const SizedBox(height: 12),
                    _EditorSubsection(
                      title: 'Outer security details',
                      description: controller.outerSecurityDisabled
                          ? 'Outer security selection is constrained by the current proxy or transport.'
                          : 'TLS, REALITY, or related cover-layer details for the current draft.',
                      child: _buildFieldGroup(_outerSecurityFields()),
                    ),
                    const SizedBox(height: 14),
                    OutlinedButton.icon(
                      onPressed: controller.busy
                          ? null
                          : controller.validateCurrentConfig,
                      icon: const Icon(Icons.verified_outlined),
                      label: const Text('Validate current'),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
              SectionCard(
                eyebrow: 'Local',
                title: 'Local Proxy',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'These values define the local entry point that the desktop client exposes after start.',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 12,
                      runSpacing: 12,
                      children: [
                        _field(controller.localHost, 'Listen host', width: 260),
                        _field(controller.localPort, 'Listen port', width: 150),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _field(
    TextEditingController controller,
    String label, {
    double width = 220,
  }) {
    return SizedBox(
      width: width,
      child: TextField(
        controller: controller,
        decoration: InputDecoration(labelText: label),
      ),
    );
  }

  Widget _buildFieldGroup(List<Widget> fields) {
    if (fields.isEmpty) {
      return const NoticeCard(
        title: 'No extra fields required',
        message:
            'The current selection does not need additional values in this section.',
        tone: Color(0xFF2F4858),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: fields,
    );
  }

  Widget _proxyDropdown() {
    return SizedBox(
      width: 230,
      child: DropdownButtonFormField<ProxyKind>(
        initialValue: controller.proxyKind,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Proxy'),
        items: [
          for (final kind in ProxyKind.values)
            DropdownMenuItem(value: kind, child: Text(kind.label)),
        ],
        onChanged: controller.busy
            ? null
            : (value) {
                if (value != null) {
                  controller.setProxyKind(value);
                }
              },
      ),
    );
  }

  Widget _transportDropdown() {
    return SizedBox(
      width: 230,
      child: DropdownButtonFormField<TransportKind>(
        initialValue: controller.transportKind,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Transport'),
        items: [
          for (final kind in TransportKind.values)
            DropdownMenuItem(value: kind, child: Text(kind.label)),
        ],
        onChanged: controller.transportDisabled
            ? null
            : (value) {
                if (value != null) {
                  controller.setTransportKind(value);
                }
              },
      ),
    );
  }

  Widget _outerSecurityDropdown() {
    return SizedBox(
      width: 230,
      child: DropdownButtonFormField<OuterSecurityKind>(
        initialValue: controller.outerSecurityKind,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Outer security'),
        items: [
          for (final kind in OuterSecurityKind.values)
            if ((kind != OuterSecurityKind.reality &&
                    kind != OuterSecurityKind.anytls &&
                    kind != OuterSecurityKind.shadowtls) ||
                controller.proxyKind == ProxyKind.vless)
              DropdownMenuItem(value: kind, child: Text(kind.label)),
        ],
        onChanged: controller.outerSecurityDisabled
            ? null
            : (value) {
                if (value != null) {
                  controller.setOuterSecurityKind(value);
                }
              },
      ),
    );
  }

  List<Widget> _proxyFields() {
    switch (controller.proxyKind) {
      case ProxyKind.vless:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [_field(controller.uuid, 'User UUID', width: 420)],
          ),
          const SizedBox(height: 4),
          CheckboxListTile(
            value: controller.vlessVisionFlow,
            onChanged: (value) {
              controller.vlessVisionFlow = value ?? false;
              controller.refreshStack();
            },
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('XTLS Vision flow (xtls-rprx-vision)'),
          ),
          const NoticeCard(
            title: 'VLESS note',
            message:
                'Enable Vision flow only when the upstream profile is explicitly configured for xtls-rprx-vision.',
            tone: Color(0xFF2F4858),
          ),
          const SizedBox(height: 12),
        ];
      case ProxyKind.naive:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(controller.naiveUsername, 'Naive username', width: 260),
              _field(controller.naivePassword, 'Naive password', width: 320),
            ],
          ),
          const SizedBox(height: 8),
          _field(
            controller.naivePaddingHeaderName,
            'Naive padding header name',
            width: 260,
          ),
          const SizedBox(height: 12),
        ];
      case ProxyKind.hysteria2:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.hysteria2ServerName,
                'Hysteria2 SNI / server name',
                width: 320,
              ),
              _field(
                controller.hysteria2Password,
                'Hysteria2 password',
                width: 320,
              ),
            ],
          ),
          CheckboxListTile(
            value: controller.hysteria2UdpEnabled,
            onChanged: (value) {
              controller.hysteria2UdpEnabled = value ?? true;
              controller.refreshStack();
            },
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('Enable UDP relay'),
          ),
          const NoticeCard(
            title: 'Hysteria2 note',
            message:
                'UDP relay is a real config dimension here, not a decorative toggle. Keep it aligned with the deployed server behavior.',
            tone: Color(0xFF2F4858),
          ),
          const SizedBox(height: 12),
        ];
      case ProxyKind.tuic:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.tuicServerName,
                'TUIC SNI / server name',
                width: 320,
              ),
              _field(controller.tuicUuid, 'TUIC user UUID', width: 420),
            ],
          ),
          const SizedBox(height: 8),
          _field(controller.tuicPassword, 'TUIC password', width: 320),
          const SizedBox(height: 12),
        ];
      case ProxyKind.trojan:
        return [
          _field(controller.trojanPassword, 'Trojan password', width: 360),
          const SizedBox(height: 12),
        ];
      case ProxyKind.mixed:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.mixedUsername,
                'Remote SOCKS username (optional)',
                width: 260,
              ),
              _field(
                controller.mixedPassword,
                'Remote SOCKS password (optional)',
                width: 260,
              ),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case ProxyKind.shadowsocks:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _shadowsocksMethodDropdown(),
              _field(
                controller.shadowsocksPassword,
                'Shadowsocks password',
                width: 320,
              ),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case ProxyKind.wireguard:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.wireguardPrivateKey,
                'WireGuard private-key',
                width: 420,
              ),
              _field(
                controller.wireguardPeerPublicKey,
                'WireGuard peer-public-key',
                width: 420,
              ),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.wireguardPreSharedKey,
                'WireGuard pre-shared-key (optional)',
                width: 420,
              ),
              _field(
                controller.wireguardClientIp,
                'WireGuard client-ip',
                width: 240,
              ),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.wireguardAllowedIps,
                'WireGuard allowed-ips (comma separated)',
                width: 420,
              ),
              _field(controller.wireguardMtu, 'WireGuard MTU', width: 180),
            ],
          ),
          const SizedBox(height: 12),
        ];
    }
  }

  Widget _shadowsocksMethodDropdown() {
    const methods = [
      'chacha20-ietf-poly1305',
      'aes-256-gcm',
      'aes-128-gcm',
      '2022-blake3-aes-128-gcm',
      '2022-blake3-aes-256-gcm',
    ];
    return SizedBox(
      width: 260,
      child: DropdownButtonFormField<String>(
        initialValue: controller.shadowsocksMethod,
        isExpanded: true,
        decoration: const InputDecoration(labelText: 'Method'),
        items: [
          for (final method in methods)
            DropdownMenuItem(value: method, child: Text(method)),
        ],
        onChanged: controller.busy
            ? null
            : (value) {
                if (value != null) {
                  controller.shadowsocksMethod = value;
                  controller.refreshStack();
                }
              },
      ),
    );
  }

  List<Widget> _transportFields() {
    switch (controller.transportKind) {
      case TransportKind.raw:
        return const [];
      case TransportKind.kcp:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(controller.kcpSeed, 'KCP seed (optional)', width: 320),
              _field(controller.kcpMtu, 'KCP MTU', width: 180),
              _field(controller.kcpTti, 'KCP TTI (ms)', width: 180),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.meek:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(controller.meekPath, 'Meek path', width: 220),
              _field(
                controller.meekHost,
                'Meek host header (optional)',
                width: 320,
              ),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.gdocsviewer:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.gdocsviewerPathPrefix,
                'Google Docs Viewer path prefix',
                width: 260,
              ),
              _field(
                controller.gdocsviewerSharedKey,
                'Google Docs Viewer shared key (optional)',
                width: 360,
              ),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.quic:
        return [
          _field(
            controller.quicServerName,
            'QUIC SNI / server name',
            width: 320,
          ),
          CheckboxListTile(
            value: controller.quicUdpEnabled,
            onChanged: (value) {
              controller.quicUdpEnabled = value ?? true;
              controller.refreshStack();
            },
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('Enable UDP relay'),
          ),
          const NoticeCard(
            title: 'QUIC note',
            message:
                'QUIC transport already constrains outer security choices in wrongcl, so this section intentionally stays narrow.',
            tone: Color(0xFF2F4858),
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.webtransport:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.webtransportAuthority,
                'WebTransport authority / SNI',
                width: 320,
              ),
              _field(
                controller.webtransportPath,
                'WebTransport path',
                width: 220,
              ),
            ],
          ),
          CheckboxListTile(
            value: controller.webtransportUdpEnabled,
            onChanged: (value) {
              controller.webtransportUdpEnabled = value ?? true;
              controller.refreshStack();
            },
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('Enable UDP relay'),
          ),
          const NoticeCard(
            title: 'WebTransport note',
            message:
                'Authority and path together define the visible tunnel shape; keep them consistent with the deployed listener.',
            tone: Color(0xFF2F4858),
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.websocket:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(controller.wsPath, 'WebSocket path', width: 220),
              _field(
                controller.wsHost,
                'WebSocket host header (optional)',
                width: 320,
              ),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.httpupgrade:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(controller.huPath, 'HTTPUpgrade path', width: 220),
              _field(
                controller.huHost,
                'HTTPUpgrade host header (optional)',
                width: 320,
              ),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.xhttp:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(controller.xhttpPath, 'XHTTP path', width: 220),
              _field(
                controller.xhttpHost,
                'XHTTP host header (optional)',
                width: 320,
              ),
            ],
          ),
          const SizedBox(height: 12),
        ];
      case TransportKind.grpc:
        return [
          _field(controller.grpcServiceName, 'gRPC service name', width: 260),
          const SizedBox(height: 12),
        ];
    }
  }

  List<Widget> _outerSecurityFields() {
    switch (controller.outerSecurityKind) {
      case OuterSecurityKind.none:
        return const [];
      case OuterSecurityKind.tls:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.tlsServerName,
                'TLS server name / SNI',
                width: 320,
              ),
              _field(controller.tlsAlpn, 'ALPN (comma separated)', width: 260),
            ],
          ),
          Row(
            children: [
              Checkbox(
                value: controller.tlsInsecure,
                onChanged: controller.busy
                    ? null
                    : (value) {
                        controller.tlsInsecure = value ?? false;
                        controller.refreshStack();
                      },
              ),
              const Text('Skip TLS certificate verification (insecure)'),
            ],
          ),
          const NoticeCard(
            title: 'TLS note',
            message:
                'Insecure skip verify should stay off unless the deployment really requires it.',
            tone: Color(0xFF7A5C1E),
          ),
          const SizedBox(height: 12),
        ];
      case OuterSecurityKind.reality:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.realityServerName,
                'REALITY SNI (cover server-name)',
                width: 320,
              ),
              _field(controller.realityShortId, 'short-id (8 hex)', width: 200),
            ],
          ),
          const SizedBox(height: 8),
          _field(
            controller.realityPublicKey,
            'public-key (server X25519, base64-url)',
            width: 520,
          ),
          const SizedBox(height: 8),
          _field(
            controller.realityRawPubkey,
            'raw-pubkey for cert verify (hex, optional)',
            width: 520,
          ),
          const SizedBox(height: 12),
        ];
      case OuterSecurityKind.anytls:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.anytlsServerName,
                'AnyTLS SNI / server name',
                width: 320,
              ),
              _field(controller.anytlsPassword, 'AnyTLS password', width: 320),
            ],
          ),
          Row(
            children: [
              Checkbox(
                value: controller.anytlsInsecureSkipVerify,
                onChanged: controller.busy
                    ? null
                    : (value) {
                        controller.anytlsInsecureSkipVerify = value ?? true;
                        controller.refreshStack();
                      },
              ),
              const Expanded(
                child: Text(
                  'Skip TLS certificate verification (default true - wrongsv anytls uses a self-signed cert)',
                ),
              ),
            ],
          ),
          const NoticeCard(
            title: 'AnyTLS note',
            message:
                'This default follows the current wrongsv anytls expectation for self-signed certificates, not a generic recommendation.',
            tone: Color(0xFF7A5C1E),
          ),
          const SizedBox(height: 12),
        ];
      case OuterSecurityKind.shadowtls:
        return [
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _field(
                controller.shadowTlsServerName,
                'ShadowTLS SNI / server name',
                width: 320,
              ),
              _field(
                controller.shadowTlsPassword,
                'ShadowTLS password',
                width: 320,
              ),
            ],
          ),
          const SizedBox(height: 8),
          const Text(
            'wrongsv shadowtls defaults to a cloudfront-style cover handshake; override the server name when the deployed fallback destination expects a different SNI.',
          ),
          const SizedBox(height: 12),
        ];
    }
  }
}
