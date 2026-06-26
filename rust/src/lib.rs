#![allow(clippy::collapsible_if)]

pub mod adapter;
pub mod anytls;
pub mod client;
pub mod config;
pub mod dns;
pub mod endpoint;
pub mod error;
pub mod ffi;
mod fragment;
pub mod gdocsviewer;
pub mod grpc;
pub mod hysteria2;
pub mod kcp;
pub mod logs;
pub mod manager;
pub mod meek;
pub mod naive;
pub mod protocol;
pub mod proxy;
pub mod quic;
mod quic_obfs;
pub mod reality;
pub mod router;
pub mod shadowsocks;
pub mod shadowtls;
pub mod snell;
pub mod source_app;
pub mod tls;
pub mod trojan;
pub mod tuic;
pub mod tun;
pub mod vision;
pub mod webtransport;
pub mod wireguard;
mod wireguard_runtime;
pub mod xhttp;

pub use adapter::{AdaptedConfig, CapabilityReport, adapt_wrongsv_config, inspect_wrongsv_config};
pub use client::{ProbeResult, Tunnel, WrongsvClient};
pub use config::{
    ActiveSelection, ClientConfig, LocalProxyConfig, NamedEndpoint, ProxyGroup, ProxyGroupKind,
    ServerConfig,
};
pub use endpoint::{
    AnyTlsOptions, Endpoint, FragmentOptions, GdocsViewerOptions, GrpcOptions, HuOptions,
    Hysteria2Options, KcpOptions, MeekOptions, MixedOptions, NaiveOptions, OuterSecurity,
    ProxyProtocol, QuicOptions, RealityOptions, ShadowsocksOptions, SnellOptions, TlsOptions,
    Transport, TrojanOptions, TuicOptions, VlessOptions, WebTransportOptions, WireGuardOptions,
    WsOptions, XhttpOptions,
};
pub use error::{ClientError, Result};
pub use manager::{ConnectionManager, global_manager};
pub use protocol::{Target, VlessAddress};
pub use proxy::{
    ConnFilter, ConnInfo, ConnRegistry, ConnState, ProxySnapshot, RegistrySnapshot, RequestEntry,
    RequestLog, global_request_log,
};
pub use tun::{TunStatus, current_status as current_tun_status};
