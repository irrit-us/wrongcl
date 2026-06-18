use base64::Engine as _;
use http::uri::Authority;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ClientError, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Endpoint {
    pub proxy: ProxyProtocol,
    #[serde(default)]
    pub transport: Transport,
    #[serde(default, rename = "outer-security")]
    pub outer_security: OuterSecurity,
}

impl Default for Endpoint {
    fn default() -> Self {
        Self {
            proxy: ProxyProtocol::Vless(VlessOptions::default()),
            transport: Transport::default(),
            outer_security: OuterSecurity::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProxyProtocol {
    Vless(VlessOptions),
    Hysteria2(Hysteria2Options),
    Tuic(TuicOptions),
    Trojan(TrojanOptions),
    Mixed(MixedOptions),
    Shadowsocks(ShadowsocksOptions),
    Wireguard(WireGuardOptions),
}

impl ProxyProtocol {
    pub fn id(&self) -> &'static str {
        match self {
            ProxyProtocol::Vless(_) => "vless",
            ProxyProtocol::Hysteria2(_) => "hysteria2",
            ProxyProtocol::Tuic(_) => "tuic",
            ProxyProtocol::Trojan(_) => "trojan",
            ProxyProtocol::Mixed(_) => "mixed",
            ProxyProtocol::Shadowsocks(_) => "shadowsocks",
            ProxyProtocol::Wireguard(_) => "wireguard",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProxyProtocol::Vless(_) => "VLESS",
            ProxyProtocol::Hysteria2(_) => "Hysteria2",
            ProxyProtocol::Tuic(_) => "TUIC",
            ProxyProtocol::Trojan(_) => "Trojan",
            ProxyProtocol::Mixed(_) => "Mixed remote SOCKS/HTTP",
            ProxyProtocol::Shadowsocks(_) => "Shadowsocks",
            ProxyProtocol::Wireguard(_) => "WireGuard",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VlessOptions {
    pub uuid: String,
    #[serde(default)]
    pub flow: String,
}

impl Default for VlessOptions {
    fn default() -> Self {
        Self {
            uuid: "12345678-1234-1234-1234-123456789abc".into(),
            flow: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrojanOptions {
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hysteria2Options {
    #[serde(rename = "server-name")]
    pub server_name: String,
    pub password: String,
    #[serde(default = "default_udp_enabled", rename = "udp-enabled")]
    pub udp_enabled: bool,
}

fn default_udp_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TuicOptions {
    #[serde(rename = "server-name")]
    pub server_name: String,
    pub uuid: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MixedOptions {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShadowsocksOptions {
    pub method: String,
    pub password: String,
}

impl Default for ShadowsocksOptions {
    fn default() -> Self {
        Self {
            method: "chacha20-ietf-poly1305".into(),
            password: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WireGuardOptions {
    #[serde(rename = "private-key")]
    pub private_key: String,
    #[serde(rename = "peer-public-key")]
    pub peer_public_key: String,
    #[serde(default, rename = "pre-shared-key")]
    pub pre_shared_key: Option<String>,
    #[serde(rename = "client-ip")]
    pub client_ip: String,
    #[serde(rename = "allowed-ips")]
    pub allowed_ips: Vec<String>,
    #[serde(default = "default_wireguard_mtu")]
    pub mtu: u32,
}

fn default_wireguard_mtu() -> u32 {
    1400
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Transport {
    #[default]
    Raw,
    Kcp(KcpOptions),
    Meek(MeekOptions),
    Gdocsviewer(GdocsViewerOptions),
    Webtransport(WebTransportOptions),
    Websocket(WsOptions),
    Httpupgrade(HuOptions),
    Xhttp(XhttpOptions),
    Grpc(GrpcOptions),
    Quic(QuicOptions),
}

impl Transport {
    pub fn id(&self) -> &'static str {
        match self {
            Transport::Raw => "raw",
            Transport::Kcp(_) => "kcp",
            Transport::Meek(_) => "meek",
            Transport::Gdocsviewer(_) => "gdocsviewer",
            Transport::Webtransport(_) => "webtransport",
            Transport::Websocket(_) => "websocket",
            Transport::Httpupgrade(_) => "httpupgrade",
            Transport::Xhttp(_) => "xhttp",
            Transport::Grpc(_) => "grpc",
            Transport::Quic(_) => "quic",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Transport::Raw => "raw",
            Transport::Kcp(_) => "KCP",
            Transport::Meek(_) => "Meek",
            Transport::Gdocsviewer(_) => "Google Docs Viewer",
            Transport::Webtransport(_) => "WebTransport",
            Transport::Websocket(_) => "WebSocket",
            Transport::Httpupgrade(_) => "HTTPUpgrade",
            Transport::Xhttp(_) => "XHTTP",
            Transport::Grpc(_) => "gRPC",
            Transport::Quic(_) => "QUIC",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsOptions {
    #[serde(default = "default_ws_path")]
    pub path: String,
    #[serde(default)]
    pub host: Option<String>,
}

impl Default for WsOptions {
    fn default() -> Self {
        Self {
            path: default_ws_path(),
            host: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HuOptions {
    #[serde(default = "default_hu_path")]
    pub path: String,
    #[serde(default)]
    pub host: Option<String>,
}

impl Default for HuOptions {
    fn default() -> Self {
        Self {
            path: default_hu_path(),
            host: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XhttpOptions {
    #[serde(default = "default_xhttp_path")]
    pub path: String,
    #[serde(default)]
    pub host: Option<String>,
}

impl Default for XhttpOptions {
    fn default() -> Self {
        Self {
            path: default_xhttp_path(),
            host: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrpcOptions {
    #[serde(default = "default_grpc_service_name", rename = "service-name")]
    pub service_name: String,
}

impl Default for GrpcOptions {
    fn default() -> Self {
        Self {
            service_name: default_grpc_service_name(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuicOptions {
    #[serde(rename = "server-name")]
    pub server_name: String,
    #[serde(default = "default_udp_enabled", rename = "udp-enabled")]
    pub udp_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcpOptions {
    #[serde(default)]
    pub seed: String,
    pub mtu: u16,
    pub tti: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeekOptions {
    #[serde(default = "default_meek_path")]
    pub path: String,
    #[serde(default)]
    pub host: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GdocsViewerOptions {
    #[serde(default = "default_gdocsviewer_path", rename = "path-prefix")]
    pub path_prefix: String,
    #[serde(default, rename = "shared-key")]
    pub shared_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebTransportOptions {
    pub authority: String,
    #[serde(default = "default_wt_path")]
    pub path: String,
    #[serde(default = "default_udp_enabled", rename = "udp-enabled")]
    pub udp_enabled: bool,
}

fn default_ws_path() -> String {
    "/ws".into()
}

fn default_hu_path() -> String {
    "/up".into()
}

fn default_xhttp_path() -> String {
    "/xhttp".into()
}

fn default_meek_path() -> String {
    "/".into()
}

fn default_gdocsviewer_path() -> String {
    "/gdocsviewer".into()
}

fn default_wt_path() -> String {
    "/wt".into()
}

fn default_grpc_service_name() -> String {
    "GunService".into()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum OuterSecurity {
    #[default]
    None,
    Tls(TlsOptions),
    Reality(RealityOptions),
    AnyTls(AnyTlsOptions),
    #[serde(rename = "shadowtls")]
    ShadowTls(ShadowTlsOptions),
}

impl OuterSecurity {
    pub fn id(&self) -> &'static str {
        match self {
            OuterSecurity::None => "none",
            OuterSecurity::Tls(_) => "tls",
            OuterSecurity::Reality(_) => "reality",
            OuterSecurity::AnyTls(_) => "anytls",
            OuterSecurity::ShadowTls(_) => "shadowtls",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            OuterSecurity::None => "none",
            OuterSecurity::Tls(_) => "TLS",
            OuterSecurity::Reality(_) => "REALITY",
            OuterSecurity::AnyTls(_) => "AnyTLS",
            OuterSecurity::ShadowTls(_) => "ShadowTLS",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TlsOptions {
    #[serde(rename = "server-name")]
    pub server_name: String,
    #[serde(default, rename = "insecure-skip-verify")]
    pub insecure_skip_verify: bool,
    #[serde(default)]
    pub alpn: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealityOptions {
    #[serde(rename = "server-name")]
    pub server_name: String,
    #[serde(rename = "public-key")]
    pub public_key: String,
    #[serde(rename = "short-id")]
    pub short_id: String,
    #[serde(default, rename = "raw-pubkey")]
    pub raw_pubkey: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnyTlsOptions {
    #[serde(rename = "server-name")]
    pub server_name: String,
    pub password: String,
    #[serde(default, rename = "insecure-skip-verify")]
    pub insecure_skip_verify: bool,
    #[serde(default)]
    pub alpn: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShadowTlsOptions {
    #[serde(rename = "server-name")]
    pub server_name: String,
    pub password: String,
}

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

    pub fn stack_summary(&self) -> String {
        if matches!(self.proxy, ProxyProtocol::Hysteria2(_)) {
            return "Hysteria2 → QUIC → TLS → TCP".into();
        }
        if matches!(self.proxy, ProxyProtocol::Tuic(_)) {
            return "TUIC → QUIC → TLS → TCP".into();
        }
        if matches!(self.proxy, ProxyProtocol::Wireguard(_)) {
            return "Payload IP → WireGuard → UDP".into();
        }
        if matches!(self.transport, Transport::Quic(_)) {
            return "VLESS → QUIC → TLS → TCP".into();
        }
        if matches!(self.transport, Transport::Kcp(_)) {
            return "VLESS → KCP → TCP".into();
        }
        if matches!(self.transport, Transport::Webtransport(_)) {
            return "VLESS → WebTransport → QUIC → TLS → TCP".into();
        }
        let mut parts: Vec<&str> = Vec::new();
        parts.push(self.proxy.display_name());
        match self.transport {
            Transport::Raw => parts.push("raw"),
            Transport::Kcp(_) => parts.push("KCP"),
            Transport::Meek(_) => parts.push("Meek"),
            Transport::Gdocsviewer(_) => parts.push("Google Docs Viewer"),
            Transport::Webtransport(_) => parts.push("WebTransport"),
            Transport::Websocket(_) => parts.push("WebSocket"),
            Transport::Httpupgrade(_) => parts.push("HTTPUpgrade"),
            Transport::Xhttp(_) => parts.push("XHTTP"),
            Transport::Grpc(_) => parts.push("gRPC"),
            Transport::Quic(_) => parts.push("QUIC"),
        }
        match self.outer_security {
            OuterSecurity::Tls(_) => parts.push("TLS"),
            OuterSecurity::Reality(_) => parts.push("REALITY"),
            OuterSecurity::AnyTls(_) => parts.push("AnyTLS"),
            OuterSecurity::ShadowTls(_) => parts.push("ShadowTLS"),
            OuterSecurity::None => {}
        }
        parts.push("TCP");
        parts.join(" → ")
    }
}

fn decode_32_byte_key(value: &str) -> Option<[u8; 32]> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value)
        .ok()?;
    decoded.try_into().ok()
}
