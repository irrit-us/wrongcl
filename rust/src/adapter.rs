use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::{ClientConfig, LocalProxyConfig, Protocol, ServerConfig};
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
}

#[derive(Debug, Deserialize)]
struct WrongsvConfig {
    listen: String,
    #[serde(default)]
    users: Vec<WrongsvUser>,
    #[serde(default)]
    flow: Option<String>,
    #[serde(default)]
    tls: Option<toml::Value>,
    #[serde(default)]
    reality: Option<toml::Value>,
    #[serde(default)]
    anytls: Option<toml::Value>,
    #[serde(default)]
    websocket: Option<WrongsvWebSocket>,
    #[serde(default)]
    httpupgrade: Option<WrongsvHttpUpgrade>,
    #[serde(default)]
    grpc: Option<toml::Value>,
    #[serde(default)]
    xhttp: Option<toml::Value>,
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
    shadowsocks: Option<toml::Value>,
    #[serde(default)]
    trojan: Option<toml::Value>,
    #[serde(default)]
    hysteria2: Option<toml::Value>,
    #[serde(default)]
    tuic: Option<toml::Value>,
    #[serde(default)]
    mixed: Option<toml::Value>,
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
struct WrongsvWebSocket {
    #[serde(default = "default_path")]
    path: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    tls: Option<toml::Value>,
}

#[derive(Debug, Deserialize)]
struct WrongsvHttpUpgrade {
    #[serde(default = "default_path")]
    path: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    tls: Option<toml::Value>,
}

fn default_path() -> String {
    "/".into()
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
    let config = if active_profile_supported(&cfg) {
        Some(client_config_for(
            cfg,
            server_host.into(),
            listen_host.into(),
            listen_port,
        )?)
    } else {
        None
    };
    Ok(AdaptedConfig { report, config })
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
                let implemented = matches!(*profile, "raw" | "websocket" | "httpupgrade");
                let profile_active = *profile == active;
                ProfileSupport {
                    profile: (*profile).to_string(),
                    display_name: (*display_name).to_string(),
                    implemented,
                    active: profile_active,
                    reason: support_reason(cfg, profile, implemented),
                }
            })
            .collect(),
    }
}

fn support_reason(cfg: &WrongsvConfig, profile: &str, implemented: bool) -> String {
    if !implemented {
        return "recognized wrongsv server capability; client transport not implemented yet".into();
    }
    if profile == "websocket" && cfg.websocket.as_ref().is_some_and(|ws| ws.tls.is_some()) {
        return "websocket carrier is implemented; websocket+tls is not implemented yet".into();
    }
    if profile == "httpupgrade" && cfg.httpupgrade.as_ref().is_some_and(|h| h.tls.is_some()) {
        return "httpupgrade carrier is implemented; httpupgrade+tls is not implemented yet".into();
    }
    "implemented for TCP SOCKS5 proxy and direct probe".into()
}

fn active_profile_supported(cfg: &WrongsvConfig) -> bool {
    match active_profile(cfg) {
        "raw" => true,
        "websocket" => cfg.websocket.as_ref().is_some_and(|ws| ws.tls.is_none()),
        "httpupgrade" => cfg.httpupgrade.as_ref().is_some_and(|h| h.tls.is_none()),
        _ => false,
    }
}

fn client_config_for(
    cfg: WrongsvConfig,
    server_host: String,
    listen_host: String,
    listen_port: u16,
) -> Result<ClientConfig> {
    let profile = active_profile(&cfg);
    let uuid = cfg
        .users
        .first()
        .map(|user| user.id.clone())
        .ok_or_else(|| ClientError::Config("wrongsv config has no [[users]] entry".into()))?;
    let flow = cfg
        .users
        .first()
        .map(|user| user.flow.clone())
        .filter(|value| !value.is_empty())
        .or(cfg.flow.clone())
        .unwrap_or_default();
    let port = parse_listen_port(&cfg.listen)
        .ok_or_else(|| ClientError::Config(format!("invalid wrongsv listen: {}", cfg.listen)))?;

    let (protocol, path, host_header) = match profile {
        "raw" => (Protocol::RawVlessTcp, None, None),
        "websocket" => {
            let ws = cfg.websocket.ok_or_else(|| {
                ClientError::Config("active websocket profile has no [websocket] config".into())
            })?;
            if ws.tls.is_some() {
                return Err(ClientError::UnsupportedProtocol(
                    "websocket+tls is not implemented yet".into(),
                ));
            }
            (Protocol::VlessWebsocket, Some(ws.path), ws.host)
        }
        "httpupgrade" => {
            let http = cfg.httpupgrade.ok_or_else(|| {
                ClientError::Config("active httpupgrade profile has no [httpupgrade] config".into())
            })?;
            if http.tls.is_some() {
                return Err(ClientError::UnsupportedProtocol(
                    "httpupgrade+tls is not implemented yet".into(),
                ));
            }
            (Protocol::VlessHttpupgrade, Some(http.path), http.host)
        }
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
            uuid,
            protocol,
            path,
            host_header,
            flow,
        },
        local: LocalProxyConfig {
            host: listen_host,
            port: listen_port,
        },
    };
    config.validate()?;
    Ok(config)
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
email = "user@example.com"

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
        assert_eq!(config.server.protocol, Protocol::VlessWebsocket);
        assert_eq!(config.server.path.as_deref(), Some("/ws"));
    }

    #[test]
    fn reports_unsupported_tls_websocket() {
        let cfg: WrongsvConfig = toml::from_str(
            r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[websocket]
path = "/ws"

[websocket.tls]
"#,
        )
        .unwrap();

        assert_eq!(active_profile(&cfg), "websocket");
        assert!(!active_profile_supported(&cfg));
    }
}
