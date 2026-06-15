use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::endpoint::{Endpoint, OuterSecurity, ProxyProtocol, Transport, VlessOptions};
use crate::error::{ClientError, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(flatten)]
    pub endpoint: Endpoint,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalProxyConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub server: ServerConfig,
    pub local: LocalProxyConfig,
}

impl ClientConfig {
    pub fn raw_vless(
        server_host: impl Into<String>,
        server_port: u16,
        uuid: impl Into<String>,
        local_host: impl Into<String>,
        local_port: u16,
    ) -> Result<Self> {
        let config = Self {
            server: ServerConfig {
                host: server_host.into(),
                port: server_port,
                endpoint: Endpoint {
                    proxy: ProxyProtocol::Vless(VlessOptions {
                        uuid: uuid.into(),
                        flow: String::new(),
                    }),
                    transport: Transport::Raw,
                    outer_security: OuterSecurity::None,
                },
            },
            local: LocalProxyConfig {
                host: local_host.into(),
                port: local_port,
            },
        };
        config.validate()?;
        Ok(config)
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let config: Self = serde_json::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        validate_host(&self.server.host, "server host")?;
        validate_port(self.server.port, "server port")?;
        validate_host(&self.local.host, "local listen host")?;
        self.server.endpoint.validate()?;
        Ok(())
    }

    pub fn with_local_port(mut self, port: u16) -> Result<Self> {
        self.local.port = port;
        self.validate()?;
        Ok(self)
    }
}

pub fn default_config() -> ClientConfig {
    ClientConfig {
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port: 443,
            endpoint: Endpoint::default(),
        },
        local: LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 1080,
        },
    }
}

pub fn config_example() -> String {
    r#"[server]
host = "127.0.0.1"
port = 443

[server.proxy]
type = "vless"
uuid = "12345678-1234-1234-1234-123456789abc"
flow = ""

[server.transport]
type = "raw"

[server.outer-security]
type = "none"

# Sample alternates — uncomment one.
#
# [server.proxy]
# type = "trojan"
# password = "change-this-password"
#
# [server.proxy]
# type = "mixed"
# username = "admin"
# password = "change-this-password"
#
# [server.transport]
# type = "websocket"
# path = "/ws"
# host = "example.com"
#
# [server.transport]
# type = "httpupgrade"
# path = "/up"
# host = "example.com"
#
# [server.outer-security]
# type = "tls"
# server-name = "example.com"
# insecure-skip-verify = false
# alpn = ["h2", "http/1.1"]

[local]
host = "127.0.0.1"
port = 1080
"#
    .to_string()
}

pub(crate) fn validate_host(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ClientError::Config(format!("{name} is required")));
    }
    Ok(())
}

pub(crate) fn validate_port(value: u16, name: &str) -> Result<()> {
    if value == 0 {
        return Err(ClientError::Config(format!(
            "{name} must be greater than zero"
        )));
    }
    Ok(())
}
