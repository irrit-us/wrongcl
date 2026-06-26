use super::*;
use crate::config::{ClientConfig, LocalProxyConfig};
use crate::endpoint::{
    Endpoint, HuOptions, KcpOptions, MixedOptions, OuterSecurity, ProxyProtocol, QuicOptions,
    ShadowsocksOptions, SnellOptions, Transport, VlessOptions, WebTransportOptions, WsOptions,
};
use crate::proxy::{ProxyHandle, ProxySnapshot};
use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc;
use std::thread;

mod cases;
mod helpers;

use helpers::*;

trait SnapshotAddr {
    fn socket_addr(&self) -> SocketAddr;
}

impl SnapshotAddr for ProxySnapshot {
    fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.local_host, self.local_port)
            .parse()
            .unwrap()
    }
}
