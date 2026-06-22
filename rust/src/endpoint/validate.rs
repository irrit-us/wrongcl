use super::*;

impl Endpoint {
    pub fn validate(&self) -> Result<()> {
        match &self.proxy {
            ProxyProtocol::Vless(opts) => {
                Uuid::parse_str(opts.uuid.trim())
                    .map_err(|e| ClientError::Config(format!("invalid VLESS UUID: {e}")))?;
                let flow = opts.flow.trim();
                if !flow.is_empty() && flow != "xtls-rprx-vision" {
                    return Err(ClientError::UnsupportedProtocol(format!(
                        "VLESS flow '{}' is not implemented in wrongcl (only 'xtls-rprx-vision' is supported)",
                        opts.flow
                    )));
                }
            }
            ProxyProtocol::Naive(opts) => {
                if !matches!(self.transport, Transport::Raw) {
                    return Err(ClientError::Config(
                        "Naive owns its HTTP/2 CONNECT transport and must use raw transport in wrongcl"
                            .into(),
                    ));
                }
                if !matches!(self.outer_security, OuterSecurity::Tls(_)) {
                    return Err(ClientError::Config(
                        "Naive requires TLS as outer security".into(),
                    ));
                }
                if opts.username.trim().is_empty() {
                    return Err(ClientError::Config(
                        "Naive requires a non-empty username".into(),
                    ));
                }
                if opts.password.trim().is_empty() {
                    return Err(ClientError::Config(
                        "Naive requires a non-empty password".into(),
                    ));
                }
                if opts.padding_header_name.trim().is_empty() {
                    return Err(ClientError::Config(
                        "Naive requires a non-empty padding-header-name".into(),
                    ));
                }
                http::HeaderName::from_bytes(opts.padding_header_name.as_bytes()).map_err(|e| {
                    ClientError::Config(format!(
                        "invalid Naive padding-header-name '{}': {e}",
                        opts.padding_header_name
                    ))
                })?;
            }
            ProxyProtocol::Hysteria2(opts) => {
                if !matches!(self.transport, Transport::Raw) {
                    return Err(ClientError::Config(
                        "Hysteria2 owns its QUIC transport and must use raw transport in wrongcl"
                            .into(),
                    ));
                }
                if !matches!(self.outer_security, OuterSecurity::None) {
                    return Err(ClientError::Config(
                        "Hysteria2 owns its TLS layer and does not wrap wrongcl outer security"
                            .into(),
                    ));
                }
                if opts.server_name.trim().is_empty() {
                    return Err(ClientError::Config(
                        "Hysteria2 requires server-name (SNI for the QUIC handshake)".into(),
                    ));
                }
                if opts.password.trim().is_empty() {
                    return Err(ClientError::Config(
                        "Hysteria2 requires a non-empty password".into(),
                    ));
                }
                match (
                    opts.obfs_type.as_deref(),
                    opts.obfs_password.as_deref(),
                    opts.obfs_min_packet_size,
                    opts.obfs_max_packet_size,
                ) {
                    (None, None, None, None) => {}
                    (Some(kind), Some(password), min, max) => {
                        if password.trim().len() < 4 {
                            return Err(ClientError::Config(
                                "Hysteria2 obfs-password must be at least 4 bytes".into(),
                            ));
                        }
                        match kind {
                            "salamander" => {
                                if min.is_some() || max.is_some() {
                                    return Err(ClientError::Config(
                                        "Hysteria2 salamander obfs does not use min/max packet size"
                                            .into(),
                                    ));
                                }
                            }
                            "gecko" => {
                                let min = min.unwrap_or(512);
                                let max = max.unwrap_or(1200);
                                if min == 0 || max == 0 || min > max {
                                    return Err(ClientError::Config(
                                        "Hysteria2 gecko obfs packet-size range is invalid".into(),
                                    ));
                                }
                            }
                            other => {
                                return Err(ClientError::Config(format!(
                                    "Hysteria2 obfs-type '{other}' is unsupported"
                                )));
                            }
                        }
                    }
                    _ => {
                        return Err(ClientError::Config(
                            "Hysteria2 obfs-type and obfs-password must be supplied together"
                                .into(),
                        ));
                    }
                }
            }
            ProxyProtocol::Tuic(opts) => {
                if !matches!(self.transport, Transport::Raw) {
                    return Err(ClientError::Config(
                        "TUIC owns its QUIC transport and must use raw transport in wrongcl".into(),
                    ));
                }
                if !matches!(self.outer_security, OuterSecurity::None) {
                    return Err(ClientError::Config(
                        "TUIC owns its TLS layer and does not wrap wrongcl outer security".into(),
                    ));
                }
                if opts.server_name.trim().is_empty() {
                    return Err(ClientError::Config(
                        "TUIC requires server-name (SNI for the QUIC handshake)".into(),
                    ));
                }
                Uuid::parse_str(opts.uuid.trim())
                    .map_err(|e| ClientError::Config(format!("invalid TUIC UUID: {e}")))?;
                if opts.password.trim().is_empty() {
                    return Err(ClientError::Config(
                        "TUIC requires a non-empty password".into(),
                    ));
                }
            }
            ProxyProtocol::Trojan(opts) => {
                if opts.password.trim().is_empty() {
                    return Err(ClientError::Config(
                        "Trojan requires a non-empty password".into(),
                    ));
                }
                if !matches!(self.outer_security, OuterSecurity::Tls(_)) {
                    return Err(ClientError::Config(
                        "Trojan requires TLS as outer security".into(),
                    ));
                }
            }
            ProxyProtocol::Mixed(_) => {
                if !matches!(self.transport, Transport::Raw) {
                    return Err(ClientError::Config(
                        "Mixed remote SOCKS5 only supports raw transport".into(),
                    ));
                }
                if !matches!(self.outer_security, OuterSecurity::None) {
                    return Err(ClientError::Config(
                        "Mixed remote SOCKS5 does not wrap an outer security layer".into(),
                    ));
                }
            }
            ProxyProtocol::Shadowsocks(opts) => {
                if opts.password.is_empty() {
                    return Err(ClientError::Config(
                        "Shadowsocks requires a non-empty password".into(),
                    ));
                }
                match opts.method.trim().to_ascii_lowercase().as_str() {
                    "aes-128-gcm"
                    | "aes-256-gcm"
                    | "chacha20-ietf-poly1305"
                    | "2022-blake3-aes-128-gcm"
                    | "2022-blake3-aes-256-gcm" => {}
                    other => {
                        return Err(ClientError::Config(format!(
                            "Shadowsocks method '{other}' is not recognized"
                        )));
                    }
                }
                if !matches!(self.transport, Transport::Raw) {
                    return Err(ClientError::Config(
                        "Shadowsocks only supports raw transport".into(),
                    ));
                }
                if !matches!(self.outer_security, OuterSecurity::None) {
                    return Err(ClientError::Config(
                        "Shadowsocks does not wrap an outer security layer".into(),
                    ));
                }
            }
            ProxyProtocol::Wireguard(opts) => {
                if !matches!(self.transport, Transport::Raw) {
                    return Err(ClientError::Config(
                        "WireGuard owns its UDP tunnel and must use raw transport in wrongcl"
                            .into(),
                    ));
                }
                if !matches!(self.outer_security, OuterSecurity::None) {
                    return Err(ClientError::Config(
                        "WireGuard does not wrap wrongcl outer security".into(),
                    ));
                }
                if opts.private_key.trim().is_empty() {
                    return Err(ClientError::Config("WireGuard requires private-key".into()));
                }
                if decode_32_byte_key(&opts.private_key).is_none() {
                    return Err(ClientError::Config(
                        "WireGuard private-key must be base64 for 32 bytes".into(),
                    ));
                }
                if decode_32_byte_key(&opts.peer_public_key).is_none() {
                    return Err(ClientError::Config(
                        "WireGuard peer-public-key must be base64 for 32 bytes".into(),
                    ));
                }
                if let Some(pre_shared_key) = &opts.pre_shared_key {
                    if decode_32_byte_key(pre_shared_key).is_none() {
                        return Err(ClientError::Config(
                            "WireGuard pre-shared-key must be base64 for 32 bytes".into(),
                        ));
                    }
                }
                if opts.client_ip.trim().is_empty() {
                    return Err(ClientError::Config("WireGuard requires client-ip".into()));
                }
                if opts.allowed_ips.is_empty() {
                    return Err(ClientError::Config(
                        "WireGuard requires at least one allowed-ips entry".into(),
                    ));
                }
                if opts.mtu < 576 {
                    return Err(ClientError::Config(
                        "WireGuard mtu must be at least 576".into(),
                    ));
                }
            }
        }

        if let OuterSecurity::Tls(opts) = &self.outer_security {
            if opts.server_name.trim().is_empty() {
                return Err(ClientError::Config(
                    "TLS outer security requires server-name".into(),
                ));
            }
        }
        if let OuterSecurity::Reality(opts) = &self.outer_security {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "REALITY outer security only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(self.transport, Transport::Raw) {
                return Err(ClientError::Config(
                    "REALITY outer security only supports raw transport".into(),
                ));
            }
            if opts.server_name.trim().is_empty() {
                return Err(ClientError::Config(
                    "REALITY requires server-name (SNI used for the cover handshake)".into(),
                ));
            }
            if opts.public_key.trim().is_empty() {
                return Err(ClientError::Config(
                    "REALITY requires public-key (base64-url server X25519 pubkey)".into(),
                ));
            }
            if opts.short_id.trim().is_empty() {
                return Err(ClientError::Config(
                    "REALITY requires short-id (8 hex chars)".into(),
                ));
            }
        }
        if let OuterSecurity::AnyTls(opts) = &self.outer_security {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "AnyTLS outer security only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(self.transport, Transport::Raw) {
                return Err(ClientError::Config(
                    "AnyTLS outer security only supports raw transport".into(),
                ));
            }
            if opts.server_name.trim().is_empty() {
                return Err(ClientError::Config(
                    "AnyTLS requires server-name (SNI for the outer TLS handshake)".into(),
                ));
            }
            if opts.password.trim().is_empty() {
                return Err(ClientError::Config(
                    "AnyTLS requires a non-empty password".into(),
                ));
            }
        }
        if let OuterSecurity::ShadowTls(opts) = &self.outer_security {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "ShadowTLS outer security only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(self.transport, Transport::Raw) {
                return Err(ClientError::Config(
                    "ShadowTLS outer security only supports raw transport".into(),
                ));
            }
            if opts.server_name.trim().is_empty() {
                return Err(ClientError::Config(
                    "ShadowTLS requires server-name (SNI for the cover ClientHello)".into(),
                ));
            }
            if opts.password.trim().is_empty() {
                return Err(ClientError::Config(
                    "ShadowTLS requires a non-empty password".into(),
                ));
            }
        }
        if let Transport::Xhttp(opts) = &self.transport {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "XHTTP transport only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(
                self.outer_security,
                OuterSecurity::None | OuterSecurity::Tls(_)
            ) {
                return Err(ClientError::Config(
                    "XHTTP transport only supports 'none' or 'tls' outer security (it owns the TLS+h2 stack)".into(),
                ));
            }
            if !opts.path.starts_with('/') {
                return Err(ClientError::Config("XHTTP path must start with '/'".into()));
            }
        }
        if let Transport::Grpc(opts) = &self.transport {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "gRPC transport only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(
                self.outer_security,
                OuterSecurity::None | OuterSecurity::Tls(_)
            ) {
                return Err(ClientError::Config(
                    "gRPC transport only supports 'none' or 'tls' outer security (it owns the TLS+h2 stack)".into(),
                ));
            }
            if opts.service_name.trim().is_empty() {
                return Err(ClientError::Config(
                    "gRPC requires a non-empty service-name".into(),
                ));
            }
        }
        if let Transport::Kcp(opts) = &self.transport {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "KCP transport only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(self.outer_security, OuterSecurity::None) {
                return Err(ClientError::Config(
                    "KCP transport does not wrap wrongcl outer security".into(),
                ));
            }
            if !(576..=1460).contains(&opts.mtu) {
                return Err(ClientError::Config("KCP mtu must be in 576..=1460".into()));
            }
            if !(10..=100).contains(&opts.tti) {
                return Err(ClientError::Config("KCP tti must be in 10..=100".into()));
            }
        }
        if let Transport::Meek(opts) = &self.transport {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "Meek transport only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(
                self.outer_security,
                OuterSecurity::None | OuterSecurity::Tls(_)
            ) {
                return Err(ClientError::Config(
                    "Meek transport only supports 'none' or 'tls' outer security".into(),
                ));
            }
            if !opts.path.starts_with('/') {
                return Err(ClientError::Config("Meek path must start with '/'".into()));
            }
        }
        if let Transport::Gdocsviewer(opts) = &self.transport {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "Google Docs Viewer transport only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(
                self.outer_security,
                OuterSecurity::None | OuterSecurity::Tls(_)
            ) {
                return Err(ClientError::Config(
                    "Google Docs Viewer transport only supports 'none' or 'tls' outer security"
                        .into(),
                ));
            }
            if !opts.path_prefix.starts_with('/') {
                return Err(ClientError::Config(
                    "Google Docs Viewer path-prefix must start with '/'".into(),
                ));
            }
            if let Some(shared_key) = &opts.shared_key {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(shared_key)
                    .map_err(|_| {
                        ClientError::Config("Google Docs Viewer shared-key must be base64".into())
                    })?;
                if decoded.len() != 32 {
                    return Err(ClientError::Config(
                        "Google Docs Viewer shared-key must decode to 32 bytes".into(),
                    ));
                }
            }
        }
        if let Transport::Webtransport(opts) = &self.transport {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "WebTransport transport only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(self.outer_security, OuterSecurity::None) {
                return Err(ClientError::Config(
                    "WebTransport transport owns its TLS layer and only supports 'none' outer security"
                        .into(),
                ));
            }
            if opts.authority.trim().is_empty() {
                return Err(ClientError::Config(
                    "WebTransport transport requires authority (used for SNI and :authority)"
                        .into(),
                ));
            }
            opts.authority
                .parse::<Authority>()
                .map_err(|e| ClientError::Config(format!("invalid WebTransport authority: {e}")))?;
            if !opts.path.starts_with('/') {
                return Err(ClientError::Config(
                    "WebTransport path must start with '/'".into(),
                ));
            }
        }
        if let Transport::Quic(opts) = &self.transport {
            if !matches!(self.proxy, ProxyProtocol::Vless(_)) {
                return Err(ClientError::Config(
                    "QUIC transport only wraps the VLESS proxy".into(),
                ));
            }
            if !matches!(self.outer_security, OuterSecurity::None) {
                return Err(ClientError::Config(
                    "QUIC transport owns its TLS layer and only supports 'none' outer security"
                        .into(),
                ));
            }
            if opts.server_name.trim().is_empty() {
                return Err(ClientError::Config(
                    "QUIC transport requires server-name (SNI for the QUIC handshake)".into(),
                ));
            }
        }
        Ok(())
    }
}

fn decode_32_byte_key(value: &str) -> Option<[u8; 32]> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value)
        .ok()?;
    decoded.try_into().ok()
}
