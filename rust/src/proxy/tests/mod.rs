use super::*;
use crate::config::{LocalProxyConfig, ServerConfig};
use crate::endpoint::{Endpoint, OuterSecurity, ProxyProtocol, ShadowsocksOptions, Transport};
use base64::Engine as _;
use std::sync::mpsc;

mod cases;
mod helpers;

use helpers::*;

impl ProxySnapshot {
    fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.local_host, self.local_port)
            .parse()
            .unwrap()
    }
}
