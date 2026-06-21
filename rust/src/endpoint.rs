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
    Naive(NaiveOptions),
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
            ProxyProtocol::Naive(_) => "naive",
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
            ProxyProtocol::Naive(_) => "Naive",
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
pub struct NaiveOptions {
    pub username: String,
    pub password: String,
    #[serde(
        default = "default_naive_padding_header",
        rename = "padding-header-name"
    )]
    pub padding_header_name: String,
}

fn default_naive_padding_header() -> String {
    "Padding".into()
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

mod summary;
mod validate;
