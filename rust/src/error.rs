use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("unsupported protocol: {0}")]
    UnsupportedProtocol(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, ClientError>;
