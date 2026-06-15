use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use crate::endpoint::{
    AnyTlsOptions, Endpoint, HuOptions, MixedOptions, OuterSecurity, ProxyProtocol, RealityOptions,
    ShadowsocksOptions, TlsOptions, Transport, TrojanOptions, VlessOptions, WsOptions,
};
use crate::error::{ClientError, Result};

const KNOWN_PROFILES: &[(&str, &str)] = &[
    ("raw", "VLESS raw TCP"),
    ("tls", "VLESS raw TCP over TLS"),
    ("reality", "VLESS REALITY"),
    ("anytls", "VLESS AnyTLS"),
    ("websocket", "VLESS WebSocket"),
    ("httpupgrade", "VLESS HTTPUpgrade"),
    ("grpc", "VLESS gRPC"),
    ("xhttp", "VLESS XHTTP"),
    ("meek", "VLESS Meek"),
    ("gdocsviewer", "VLESS Google Docs Viewer"),
    ("quic", "VLESS QUIC"),
    ("kcp", "VLESS mKCP"),
    ("webtransport", "VLESS WebTransport"),
    ("shadowtls", "VLESS ShadowTLS"),
    ("vmess", "VMess AEAD"),
    ("shadowsocks", "Shadowsocks AEAD/2022"),
    ("trojan", "Trojan TLS"),
    ("hysteria2", "Hysteria2"),
    ("tuic", "TUIC"),
    ("mixed", "Mixed SOCKS/HTTP proxy inbound"),
    ("wireguard", "WireGuard"),
    ("naive", "Naive"),
];

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityReport {
    pub active_profile: String,
    pub listen: String,
    pub listen_port: u16,
    pub profiles: Vec<ProfileSupport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileSupport {
    pub profile: String,
    pub display_name: String,
    pub implemented: bool,
    pub active: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdaptedConfig {
    pub report: CapabilityReport,
    pub config: Option<ClientConfig>,
    pub stack_summary: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WrongsvConfig {
    listen: String,
    #[serde(default)]
    users: Vec<WrongsvUser>,
    #[serde(default)]
    flow: Option<String>,
    #[serde(default)]
    tls: Option<WrongsvTls>,
    #[serde(default)]
    reality: Option<WrongsvReality>,
    #[serde(default)]
    anytls: Option<WrongsvAnyTls>,
    #[serde(default)]
    websocket: Option<WrongsvWebSocket>,
    #[serde(default)]
    httpupgrade: Option<WrongsvHttpUpgrade>,
    #[serde(default)]
    grpc: Option<WrongsvGrpc>,
    #[serde(default)]
    xhttp: Option<WrongsvXhttp>,
    #[serde(default)]
    meek: Option<toml::Value>,
    #[serde(default)]
    gdocsviewer: Option<toml::Value>,
    #[serde(default)]
    quic: Option<toml::Value>,
    #[serde(default)]
    kcp: Option<toml::Value>,
    #[serde(default)]
    webtransport: Option<toml::Value>,
    #[serde(default)]
    shadowtls: Option<toml::Value>,
    #[serde(default)]
    vmess: Option<toml::Value>,
    #[serde(default)]
    shadowsocks: Option<WrongsvShadowsocks>,
    #[serde(default)]
    trojan: Option<WrongsvTrojan>,
    #[serde(default)]
    hysteria2: Option<toml::Value>,
    #[serde(default)]
    tuic: Option<toml::Value>,
    #[serde(default)]
    mixed: Option<WrongsvMixed>,
    #[serde(default)]
    wireguard: Option<toml::Value>,
    #[serde(default)]
    naive: Option<toml::Value>,
}

#[derive(Debug, Deserialize)]
struct WrongsvUser {
    id: String,
    #[serde(default)]
    flow: String,
}

#[derive(Debug, Deserialize)]
struct WrongsvTls {
    #[serde(default, alias = "server_name", alias = "server-name", alias = "sni")]
    server_name: Option<String>,
    #[serde(default)]
    alpn: Option<Vec<String>>,
    #[serde(
        default,
        alias = "insecure",
        alias = "insecure_skip_verify",
        alias = "insecure-skip-verify"
    )]
    insecure: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct WrongsvReality {
    #[serde(default, alias = "server_name", alias = "server-name", alias = "sni")]
    server_name: Option<String>,
    #[serde(default)]
    dest: Option<String>,
    #[serde(
        default,
        alias = "public_key",
        alias = "public-key",
        alias = "publickey"
    )]
    public_key: Option<String>,
    #[serde(default, alias = "short_id", alias = "short-id", alias = "shortid")]
    short_id: Option<String>,
    #[serde(default, alias = "short_ids", alias = "short-ids")]
    short_ids: Option<Vec<String>>,
    #[serde(
        default,
        alias = "raw_pubkey",
        alias = "raw-pubkey",
        alias = "server_pubkey",
        alias = "server-pubkey"
    )]
    raw_pubkey: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WrongsvAnyTls {
    password: String,
    #[serde(default, alias = "server_name", alias = "server-name", alias = "sni")]
    server_name: Option<String>,
    #[serde(default)]
    alpn: Option<Vec<String>>,
    #[serde(
        default,
        alias = "insecure",
        alias = "insecure_skip_verify",
        alias = "insecure-skip-verify"
    )]
    insecure: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct WrongsvWebSocket {
    #[serde(default = "default_ws_path")]
    path: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    tls: Option<WrongsvTls>,
}

#[derive(Debug, Deserialize)]
struct WrongsvHttpUpgrade {
    #[serde(default = "default_hu_path")]
    path: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    tls: Option<WrongsvTls>,
}

#[derive(Debug, Deserialize)]
struct WrongsvXhttp {
    #[serde(default = "default_xhttp_path")]
    path: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    tls: Option<WrongsvTls>,
}

#[derive(Debug, Deserialize)]
struct WrongsvGrpc {
    #[serde(default, rename = "service_name", alias = "service-name")]
    service_name: Option<String>,
    #[serde(default)]
    tls: Option<WrongsvTls>,
}

#[derive(Debug, Deserialize)]
struct WrongsvMixed {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WrongsvShadowsocks {
    method: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct WrongsvTrojan {
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    users: Vec<WrongsvTrojanUser>,
    #[serde(default)]
    tls: Option<WrongsvTls>,
}

#[derive(Debug, Deserialize)]
struct WrongsvTrojanUser {
    #[serde(default)]
    password: String,
}

fn default_ws_path() -> String {
    "/".into()
}

fn default_hu_path() -> String {
    "/".into()
}

fn default_xhttp_path() -> String {
    "/xhttp".into()
}

pub fn inspect_wrongsv_config(path: impl AsRef<Path>) -> Result<CapabilityReport> {
    let cfg = read_wrongsv_config(path)?;
    Ok(report_for(&cfg))
}

pub fn adapt_wrongsv_config(
    path: impl AsRef<Path>,
    server_host: impl Into<String>,
    listen_host: impl Into<String>,
    listen_port: u16,
) -> Result<AdaptedConfig> {
    let cfg = read_wrongsv_config(path)?;
    let report = report_for(&cfg);
    let server_host = server_host.into();
    let listen_host = listen_host.into();
    let config = if active_profile_supported(&cfg) {
        Some(client_config_for(
            cfg,
            server_host,
            listen_host,
            listen_port,
        )?)
    } else {
        None
    };
    let stack_summary = config
        .as_ref()
        .map(|cfg| cfg.server.endpoint.stack_summary());
    Ok(AdaptedConfig {
        report,
        config,
        stack_summary,
    })
}

fn read_wrongsv_config(path: impl AsRef<Path>) -> Result<WrongsvConfig> {
    let content = fs::read_to_string(path)?;
    let cfg: WrongsvConfig = toml::from_str(&content)?;
    Ok(cfg)
}

fn report_for(cfg: &WrongsvConfig) -> CapabilityReport {
    let active = active_profile(cfg).to_string();
    CapabilityReport {
        active_profile: active.clone(),
        listen: cfg.listen.clone(),
        listen_port: parse_listen_port(&cfg.listen).unwrap_or(0),
        profiles: KNOWN_PROFILES
            .iter()
            .map(|(profile, display_name)| {
                let implemented = matches!(
                    *profile,
                    "raw"
                        | "tls"
                        | "reality"
                        | "anytls"
                        | "websocket"
                        | "httpupgrade"
                        | "xhttp"
                        | "grpc"
                        | "trojan"
                        | "mixed"
                        | "shadowsocks"
                );
                let profile_active = *profile == active;
                ProfileSupport {
                    profile: (*profile).to_string(),
                    display_name: (*display_name).to_string(),
                    implemented,
                    active: profile_active,
                    reason: support_reason(profile, implemented),
                }
            })
            .collect(),
    }
}

fn support_reason(profile: &str, implemented: bool) -> String {
    if !implemented {
        return "recognized wrongsv server capability; client transport not implemented yet".into();
    }
    match profile {
        "raw" => "VLESS over raw TCP".into(),
        "tls" => "VLESS over raw TCP wrapped by TLS".into(),
        "reality" => "VLESS over raw TCP wrapped by REALITY (TLS 1.3 cover handshake)".into(),
        "anytls" => "VLESS over raw TCP wrapped by TLS + SHA256(password) auth frame".into(),
        "websocket" => "VLESS over WebSocket, optionally wrapped by TLS".into(),
        "httpupgrade" => "VLESS over HTTPUpgrade, optionally wrapped by TLS".into(),
        "xhttp" => "VLESS over XHTTP (HTTP/2 + raw DATA frames), optionally wrapped by TLS".into(),
        "grpc" => {
            "VLESS over gRPC (HTTP/2 + Hunk frames, V2Ray-compatible), optionally wrapped by TLS"
                .into()
        }
        "trojan" => "Trojan over TLS (no transport carrier)".into(),
        "mixed" => "remote SOCKS5/HTTP inbound (raw TCP)".into(),
        "shadowsocks" => {
            "Shadowsocks classic AEAD (aes-128-gcm/aes-256-gcm/chacha20-ietf-poly1305)".into()
        }
        _ => "implemented for SOCKS5 proxy and direct probe".into(),
    }
}

fn active_profile_supported(cfg: &WrongsvConfig) -> bool {
    matches!(
        active_profile(cfg),
        "raw"
            | "tls"
            | "reality"
            | "anytls"
            | "websocket"
            | "httpupgrade"
            | "xhttp"
            | "grpc"
            | "trojan"
            | "mixed"
            | "shadowsocks"
    )
}

fn client_config_for(
    cfg: WrongsvConfig,
    server_host: String,
    listen_host: String,
    listen_port: u16,
) -> Result<ClientConfig> {
    let profile = active_profile(&cfg);
    let port = parse_listen_port(&cfg.listen)
        .ok_or_else(|| ClientError::Config(format!("invalid wrongsv listen: {}", cfg.listen)))?;

    let endpoint = match profile {
        "raw" => endpoint_for_vless(&cfg, Transport::Raw, OuterSecurity::None)?,
        "tls" => {
            let outer = tls_options(cfg.tls.as_ref(), &server_host)?;
            endpoint_for_vless(&cfg, Transport::Raw, OuterSecurity::Tls(outer))?
        }
        "reality" => {
            let outer = reality_options(cfg.reality.as_ref(), &server_host)?;
            endpoint_for_vless(&cfg, Transport::Raw, OuterSecurity::Reality(outer))?
        }
        "anytls" => {
            let outer = anytls_options(cfg.anytls.as_ref(), &server_host)?;
            endpoint_for_vless(&cfg, Transport::Raw, OuterSecurity::AnyTls(outer))?
        }
        "websocket" => {
            let ws = cfg
                .websocket
                .as_ref()
                .ok_or_else(|| ClientError::Config("missing [websocket] table".into()))?;
            let outer = match ws.tls.as_ref() {
                Some(tls) => OuterSecurity::Tls(tls_options(Some(tls), &server_host)?),
                None => OuterSecurity::None,
            };
            endpoint_for_vless(
                &cfg,
                Transport::Websocket(WsOptions {
                    path: ws.path.clone(),
                    host: ws.host.clone(),
                }),
                outer,
            )?
        }
        "httpupgrade" => {
            let hu = cfg
                .httpupgrade
                .as_ref()
                .ok_or_else(|| ClientError::Config("missing [httpupgrade] table".into()))?;
            let outer = match hu.tls.as_ref() {
                Some(tls) => OuterSecurity::Tls(tls_options(Some(tls), &server_host)?),
                None => OuterSecurity::None,
            };
            endpoint_for_vless(
                &cfg,
                Transport::Httpupgrade(HuOptions {
                    path: hu.path.clone(),
                    host: hu.host.clone(),
                }),
                outer,
            )?
        }
        "xhttp" => {
            let xh = cfg
                .xhttp
                .as_ref()
                .ok_or_else(|| ClientError::Config("missing [xhttp] table".into()))?;
            let outer = match xh.tls.as_ref() {
                Some(tls) => OuterSecurity::Tls(tls_options(Some(tls), &server_host)?),
                None => OuterSecurity::None,
            };
            endpoint_for_vless(
                &cfg,
                Transport::Xhttp(crate::endpoint::XhttpOptions {
                    path: xh.path.clone(),
                    host: xh.host.clone(),
                }),
                outer,
            )?
        }
        "grpc" => {
            let gr = cfg
                .grpc
                .as_ref()
                .ok_or_else(|| ClientError::Config("missing [grpc] table".into()))?;
            let outer = match gr.tls.as_ref() {
                Some(tls) => OuterSecurity::Tls(tls_options(Some(tls), &server_host)?),
                None => OuterSecurity::None,
            };
            let service_name = gr
                .service_name
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "GunService".into());
            endpoint_for_vless(
                &cfg,
                Transport::Grpc(crate::endpoint::GrpcOptions { service_name }),
                outer,
            )?
        }
        "trojan" => endpoint_for_trojan(&cfg, &server_host)?,
        "mixed" => endpoint_for_mixed(&cfg)?,
        "shadowsocks" => endpoint_for_shadowsocks(&cfg)?,
        other => {
            return Err(ClientError::UnsupportedProtocol(format!(
                "wrongsv profile '{other}' is recognized but not implemented in wrongcl yet"
            )));
        }
    };

    let config = ClientConfig {
        server: ServerConfig {
            host: server_host,
            port,
            endpoint,
        },
        local: LocalProxyConfig {
            host: listen_host,
            port: listen_port,
        },
    };
    config.validate()?;
    Ok(config)
}

fn endpoint_for_vless(
    cfg: &WrongsvConfig,
    transport: Transport,
    outer_security: OuterSecurity,
) -> Result<Endpoint> {
    let user = cfg
        .users
        .first()
        .ok_or_else(|| ClientError::Config("wrongsv config has no [[users]] entry".into()))?;
    let flow = if user.flow.is_empty() {
        cfg.flow.clone().unwrap_or_default()
    } else {
        user.flow.clone()
    };
    Ok(Endpoint {
        proxy: ProxyProtocol::Vless(VlessOptions {
            uuid: user.id.clone(),
            flow,
        }),
        transport,
        outer_security,
    })
}

fn endpoint_for_trojan(cfg: &WrongsvConfig, server_host: &str) -> Result<Endpoint> {
    let trojan = cfg
        .trojan
        .as_ref()
        .ok_or_else(|| ClientError::Config("missing [trojan] table".into()))?;
    let password = trojan
        .password
        .clone()
        .or_else(|| trojan.users.first().map(|u| u.password.clone()))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ClientError::Config("Trojan requires a password".into()))?;
    let tls = tls_options(trojan.tls.as_ref(), server_host)?;
    Ok(Endpoint {
        proxy: ProxyProtocol::Trojan(TrojanOptions { password }),
        transport: Transport::Raw,
        outer_security: OuterSecurity::Tls(tls),
    })
}

fn endpoint_for_mixed(cfg: &WrongsvConfig) -> Result<Endpoint> {
    let mixed = cfg
        .mixed
        .as_ref()
        .ok_or_else(|| ClientError::Config("missing [mixed] table".into()))?;
    Ok(Endpoint {
        proxy: ProxyProtocol::Mixed(MixedOptions {
            username: mixed.username.clone(),
            password: mixed.password.clone(),
        }),
        transport: Transport::Raw,
        outer_security: OuterSecurity::None,
    })
}

fn endpoint_for_shadowsocks(cfg: &WrongsvConfig) -> Result<Endpoint> {
    let ss = cfg
        .shadowsocks
        .as_ref()
        .ok_or_else(|| ClientError::Config("missing [shadowsocks] table".into()))?;
    Ok(Endpoint {
        proxy: ProxyProtocol::Shadowsocks(ShadowsocksOptions {
            method: ss.method.clone(),
            password: ss.password.clone(),
        }),
        transport: Transport::Raw,
        outer_security: OuterSecurity::None,
    })
}

fn tls_options(tls: Option<&WrongsvTls>, server_host: &str) -> Result<TlsOptions> {
    let server_name = tls
        .and_then(|t| t.server_name.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| server_host.to_string());
    let alpn = tls.and_then(|t| t.alpn.clone()).unwrap_or_default();
    let insecure_skip_verify = tls.and_then(|t| t.insecure).unwrap_or(false);
    Ok(TlsOptions {
        server_name,
        insecure_skip_verify,
        alpn,
    })
}

fn reality_options(reality: Option<&WrongsvReality>, server_host: &str) -> Result<RealityOptions> {
    let reality = reality.ok_or_else(|| ClientError::Config("missing [reality] table".into()))?;
    let server_name = reality
        .server_name
        .clone()
        .or_else(|| reality.dest.clone().map(|d| host_from_dest(&d)))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| server_host.to_string());
    let public_key = reality
        .public_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ClientError::Config("REALITY [reality].public-key is required (server config holds private_key; client needs the matching public_key)".into()))?;
    let short_id = reality
        .short_id
        .clone()
        .or_else(|| {
            reality
                .short_ids
                .as_ref()
                .and_then(|list| list.first().cloned())
        })
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ClientError::Config("REALITY [reality].short-id is required".into()))?;
    let raw_pubkey = reality.raw_pubkey.clone().unwrap_or_default();
    Ok(RealityOptions {
        server_name,
        public_key,
        short_id,
        raw_pubkey,
    })
}

fn host_from_dest(dest: &str) -> String {
    dest.rsplit_once(':')
        .map(|(host, _)| host.to_string())
        .unwrap_or_else(|| dest.to_string())
}

fn anytls_options(anytls: Option<&WrongsvAnyTls>, server_host: &str) -> Result<AnyTlsOptions> {
    let anytls = anytls.ok_or_else(|| ClientError::Config("missing [anytls] table".into()))?;
    let password = anytls.password.clone();
    if password.trim().is_empty() {
        return Err(ClientError::Config(
            "AnyTLS [anytls].password is required".into(),
        ));
    }
    let server_name = anytls
        .server_name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| server_host.to_string());
    let alpn = anytls.alpn.clone().unwrap_or_default();
    let insecure_skip_verify = anytls.insecure.unwrap_or(true);
    Ok(AnyTlsOptions {
        server_name,
        password,
        insecure_skip_verify,
        alpn,
    })
}

fn active_profile(cfg: &WrongsvConfig) -> &'static str {
    if cfg.reality.is_some() {
        "reality"
    } else if cfg.anytls.is_some() {
        "anytls"
    } else if cfg.websocket.is_some() {
        "websocket"
    } else if cfg.httpupgrade.is_some() {
        "httpupgrade"
    } else if cfg.grpc.is_some() {
        "grpc"
    } else if cfg.xhttp.is_some() {
        "xhttp"
    } else if cfg.meek.is_some() {
        "meek"
    } else if cfg.gdocsviewer.is_some() {
        "gdocsviewer"
    } else if cfg.quic.is_some() {
        "quic"
    } else if cfg.kcp.is_some() {
        "kcp"
    } else if cfg.webtransport.is_some() {
        "webtransport"
    } else if cfg.shadowtls.is_some() {
        "shadowtls"
    } else if cfg.vmess.is_some() {
        "vmess"
    } else if cfg.shadowsocks.is_some() {
        "shadowsocks"
    } else if cfg.trojan.is_some() {
        "trojan"
    } else if cfg.hysteria2.is_some() {
        "hysteria2"
    } else if cfg.tuic.is_some() {
        "tuic"
    } else if cfg.mixed.is_some() {
        "mixed"
    } else if cfg.wireguard.is_some() {
        "wireguard"
    } else if cfg.naive.is_some() {
        "naive"
    } else if cfg.tls.is_some() {
        "tls"
    } else {
        "raw"
    }
}

fn parse_listen_port(listen: &str) -> Option<u16> {
    listen.rsplit_once(':')?.1.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_websocket_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[websocket]
path = "/ws"
"#,
        )
        .unwrap();

        let report = report_for(&cfg);
        assert_eq!(report.active_profile, "websocket");
        assert!(active_profile_supported(&cfg));

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        assert!(matches!(
            config.server.endpoint.proxy,
            ProxyProtocol::Vless(_)
        ));
        match &config.server.endpoint.transport {
            Transport::Websocket(ws) => assert_eq!(ws.path, "/ws"),
            other => panic!("unexpected transport {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
    }

    #[test]
    fn adapts_websocket_over_tls_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[websocket]
path = "/ws"

[websocket.tls]
server_name = "example.com"
alpn = ["h2", "http/1.1"]
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "websocket");
        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
            OuterSecurity::Tls(tls) => {
                assert_eq!(tls.server_name, "example.com");
                assert_eq!(tls.alpn, vec!["h2".to_string(), "http/1.1".to_string()]);
            }
            other => panic!("expected TLS, got {other:?}"),
        }
    }

    #[test]
    fn adapts_xhttp_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[xhttp]
path = "/xhttp"
host = "xhttp.example"
"#,
        )
        .unwrap();

        let report = report_for(&cfg);
        assert_eq!(report.active_profile, "xhttp");
        assert!(active_profile_supported(&cfg));

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Xhttp(xh) => {
                assert_eq!(xh.path, "/xhttp");
                assert_eq!(xh.host.as_deref(), Some("xhttp.example"));
            }
            other => panic!("unexpected transport {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → XHTTP → TCP"
        );
    }

    #[test]
    fn adapts_xhttp_over_tls_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[xhttp]
path = "/x"

[xhttp.tls]
server_name = "xhttp.example"
"#,
        )
        .unwrap();

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
            OuterSecurity::Tls(tls) => assert_eq!(tls.server_name, "xhttp.example"),
            other => panic!("expected TLS, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → XHTTP → TLS → TCP"
        );
    }

    #[test]
    fn adapts_grpc_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[grpc]
service_name = "MyGun"
"#,
        )
        .unwrap();

        let report = report_for(&cfg);
        assert_eq!(report.active_profile, "grpc");
        assert!(active_profile_supported(&cfg));

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Grpc(gr) => assert_eq!(gr.service_name, "MyGun"),
            other => panic!("unexpected transport {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → gRPC → TCP"
        );
    }

    #[test]
    fn adapts_grpc_over_tls_config_defaults_service_name() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[grpc]

[grpc.tls]
server_name = "grpc.example"
"#,
        )
        .unwrap();

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Grpc(gr) => assert_eq!(gr.service_name, "GunService"),
            other => panic!("unexpected transport {other:?}"),
        }
        match &config.server.endpoint.outer_security {
            OuterSecurity::Tls(tls) => assert_eq!(tls.server_name, "grpc.example"),
            other => panic!("expected TLS, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → gRPC → TLS → TCP"
        );
    }

    #[test]
    fn adapts_trojan_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[trojan]
password = "hunter2"

[trojan.tls]
server_name = "example.com"
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "trojan");
        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Trojan(opts) => assert_eq!(opts.password, "hunter2"),
            other => panic!("unexpected proxy {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::Tls(_)
        ));
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "Trojan → raw → TLS → TCP"
        );
    }

    #[test]
    fn adapts_mixed_config_with_credentials() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "127.0.0.1:1080"

[mixed]
username = "admin"
password = "secret"
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "mixed");
        assert!(active_profile_supported(&cfg));

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1081).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Mixed(opts) => {
                assert_eq!(opts.username.as_deref(), Some("admin"));
                assert_eq!(opts.password.as_deref(), Some("secret"));
            }
            other => panic!("unexpected proxy {other:?}"),
        }
    }

    #[test]
    fn adapts_shadowsocks_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:8388"

[shadowsocks]
method = "chacha20-ietf-poly1305"
password = "hunter2"
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "shadowsocks");
        assert!(active_profile_supported(&cfg));

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Shadowsocks(opts) => {
                assert_eq!(opts.method, "chacha20-ietf-poly1305");
                assert_eq!(opts.password, "hunter2");
            }
            other => panic!("unexpected proxy {other:?}"),
        }
        assert!(matches!(config.server.endpoint.transport, Transport::Raw));
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
    }

    #[test]
    fn adapts_reality_config_with_vision_flow() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
flow = "xtls-rprx-vision"

[reality]
public_key = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "reality");
        assert!(active_profile_supported(&cfg));

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Vless(opts) => assert_eq!(opts.flow, "xtls-rprx-vision"),
            other => panic!("expected VLESS, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_vless_flow() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
flow = "xtls-rprx-direct"

[reality]
public_key = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
        )
        .unwrap();

        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
        match err {
            ClientError::UnsupportedProtocol(msg) => {
                assert!(msg.contains("xtls-rprx-direct"));
            }
            other => panic!("expected UnsupportedProtocol, got {other:?}"),
        }
    }

    #[test]
    fn adapts_reality_config_without_vision_flow() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[reality]
public_key = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "reality");
        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
            OuterSecurity::Reality(opts) => {
                assert_eq!(opts.server_name, "www.microsoft.com");
                assert_eq!(opts.short_id, "aaaaaaaa");
                assert_eq!(
                    opts.public_key,
                    "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
                );
            }
            other => panic!("expected REALITY, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → raw → REALITY → TCP"
        );
    }

    #[test]
    fn reality_config_missing_public_key_rejected() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[reality]
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
        )
        .unwrap();

        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
        match err {
            ClientError::Config(msg) => assert!(msg.contains("public-key")),
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn adapts_anytls_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[anytls]
password = "hunter2"
server_name = "cloudfront.net"
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "anytls");
        assert!(active_profile_supported(&cfg));

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
            OuterSecurity::AnyTls(opts) => {
                assert_eq!(opts.password, "hunter2");
                assert_eq!(opts.server_name, "cloudfront.net");
                assert!(opts.insecure_skip_verify);
            }
            other => panic!("expected AnyTLS, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → raw → AnyTLS → TCP"
        );
    }

    #[test]
    fn anytls_config_missing_password_rejected() {
        let err = toml::from_str::<WrongsvConfig>(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[anytls]
"#,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("password"),
            "expected missing field error, got {err}"
        );
    }

    #[test]
    fn adapts_raw_tls_config() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[tls]
server_name = "example.com"
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "tls");
        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        assert!(matches!(config.server.endpoint.transport, Transport::Raw));
        match &config.server.endpoint.outer_security {
            OuterSecurity::Tls(tls) => assert_eq!(tls.server_name, "example.com"),
            other => panic!("expected TLS, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unparseable_listen_string() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "not-a-host-port-string"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
"#,
        )
        .unwrap();
        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
        match err {
            ClientError::Config(msg) => assert!(msg.contains("invalid wrongsv listen")),
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unimplemented_profile() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[vmess]
alter_id = 0
"#,
        )
        .unwrap();
        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
        match err {
            ClientError::UnsupportedProtocol(msg) => {
                assert!(msg.contains("vmess"), "msg: {msg}");
                assert!(msg.contains("not implemented"), "msg: {msg}");
            }
            other => panic!("expected UnsupportedProtocol, got {other:?}"),
        }
    }

    #[test]
    fn rejects_trojan_with_empty_password() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[trojan]
password = ""
"#,
        )
        .unwrap();
        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
        match err {
            ClientError::Config(msg) => {
                assert!(msg.to_lowercase().contains("trojan"), "msg: {msg}");
                assert!(msg.to_lowercase().contains("password"), "msg: {msg}");
            }
            other => panic!("expected Config error, got {other:?}"),
        }
    }
}
