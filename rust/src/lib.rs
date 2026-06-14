pub mod adapter;
pub mod client;
pub mod config;
pub mod error;
pub mod ffi;
pub mod manager;
pub mod protocol;
pub mod proxy;

pub use adapter::{adapt_wrongsv_config, inspect_wrongsv_config, AdaptedConfig, CapabilityReport};
pub use client::{ProbeResult, RawVlessTcpClient, Tunnel, WrongsvClient};
pub use config::{ClientConfig, LocalProxyConfig, Protocol, ServerConfig};
pub use error::{ClientError, Result};
pub use manager::{global_manager, ConnectionManager};
pub use protocol::{Target, VlessAddress};
pub use proxy::ProxySnapshot;
