pub mod client;
pub mod config;
pub mod error;
pub mod ffi;
pub mod manager;
pub mod protocol;
pub mod proxy;

pub use client::{ProbeResult, RawVlessTcpClient};
pub use config::{ClientConfig, LocalProxyConfig, Protocol, ServerConfig};
pub use error::{ClientError, Result};
pub use manager::{global_manager, ConnectionManager};
pub use protocol::{Target, VlessAddress};
pub use proxy::ProxySnapshot;
