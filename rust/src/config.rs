use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ClientError, Result};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Protocol {
    #[default]
    RawVlessTcp,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::RawVlessTcp => "raw-vless-tcp",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub uuid: String,
    #[serde(default)]
    pub protocol: Protocol,
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
    pub fn new(
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
                uuid: uuid.into(),
                protocol: Protocol::RawVlessTcp,
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

    pub fn validate(&self) -> Result<()> {
        validate_host(&self.server.host, "server host")?;
        validate_port(self.server.port, "server port")?;
        validate_host(&self.local.host, "local listen host")?;
        Uuid::parse_str(self.server.uuid.trim())
            .map_err(|e| ClientError::Config(format!("invalid UUID: {e}")))?;
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
            uuid: "12345678-1234-1234-1234-123456789abc".into(),
            protocol: Protocol::RawVlessTcp,
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
uuid = "12345678-1234-1234-1234-123456789abc"
protocol = "raw-vless-tcp"

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
