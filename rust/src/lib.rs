pub mod adapter;
pub mod anytls;
pub mod client;
pub mod config;
pub mod endpoint;
pub mod error;
pub mod ffi;
pub mod gdocsviewer;
pub mod grpc;
pub mod hysteria2;
pub mod kcp;
pub mod manager;
pub mod meek;
pub mod naive;
pub mod protocol;
pub mod proxy;
pub mod quic;
pub mod reality;
pub mod shadowsocks;
pub mod shadowtls;
pub mod tls;
pub mod trojan;
pub mod tuic;
pub mod vision;
pub mod webtransport;
pub mod wireguard;
mod wireguard_runtime;
pub mod xhttp;

pub use adapter::{adapt_wrongsv_config, inspect_wrongsv_config, AdaptedConfig, CapabilityReport};
pub use client::{ProbeResult, Tunnel, WrongsvClient};
pub use config::{ClientConfig, LocalProxyConfig, ServerConfig};
pub use endpoint::{
    AnyTlsOptions, Endpoint, GdocsViewerOptions, GrpcOptions, HuOptions, Hysteria2Options,
    KcpOptions, MeekOptions, MixedOptions, NaiveOptions, OuterSecurity, ProxyProtocol, QuicOptions,
    RealityOptions, ShadowsocksOptions, TlsOptions, Transport, TrojanOptions, TuicOptions,
    VlessOptions, WebTransportOptions, WireGuardOptions, WsOptions, XhttpOptions,
};
pub use error::{ClientError, Result};
pub use manager::{global_manager, ConnectionManager};
pub use protocol::{Target, VlessAddress};
pub use proxy::ProxySnapshot;
