use std::io::{self, Read, Write};
use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket,
};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::client::{Tunnel, UdpSession, WrongsvClient};
use crate::config::ClientConfig;
use crate::error::{ClientError, Result};
use crate::protocol::Target;

mod relay;
mod request;
mod udp;

const SOCKS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

use relay::{relay, relay_with_initial};
use request::{
    detect_local_proxy_request, write_http_connect_ok, write_http_error, write_socks5_reply,
    LocalProxyRequest, SocksRequest,
};
use udp::relay_udp_associate;

pub struct ProxyHandle {
    local_addr: SocketAddr,
    shared: Arc<ProxyShared>,
    join: Option<JoinHandle<()>>,
}

impl ProxyHandle {
    pub fn start(config: ClientConfig) -> Result<Self> {
        let listener = TcpListener::bind((config.local.host.as_str(), config.local.port))?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;

        let shared = Arc::new(ProxyShared {
            stop: AtomicBool::new(false),
            metrics: ProxyMetrics::new(),
        });
        let accept_shared = Arc::clone(&shared);
        let join = thread::Builder::new()
            .name("wrongcl-socks5".into())
            .spawn(move || accept_loop(listener, config, accept_shared))
            .map_err(|e| ClientError::Io(io::Error::other(format!("start proxy thread: {e}"))))?;

        Ok(Self {
            local_addr,
            shared,
            join: Some(join),
        })
    }

    pub fn stop(&mut self) -> Result<()> {
        self.shared.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&self.local_addr, Duration::from_millis(250));
        if let Some(join) = self.join.take() {
            join.join().map_err(|_| {
                ClientError::Io(io::Error::other("proxy thread panicked while stopping"))
            })?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> ProxySnapshot {
        self.shared.snapshot(self.local_addr, true)
    }
}

impl Drop for ProxyHandle {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

struct ProxyShared {
    stop: AtomicBool,
    metrics: ProxyMetrics,
}

impl ProxyShared {
    fn snapshot(&self, local_addr: SocketAddr, running: bool) -> ProxySnapshot {
        self.metrics.snapshot(local_addr, running)
    }
}

struct ProxyMetrics {
    started_at_unix: u64,
    active_connections: AtomicUsize,
    total_connections: AtomicU64,
    failed_connections: AtomicU64,
    bytes_uploaded: AtomicU64,
    bytes_downloaded: AtomicU64,
}

impl ProxyMetrics {
    fn new() -> Self {
        Self {
            started_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            active_connections: AtomicUsize::new(0),
            total_connections: AtomicU64::new(0),
            failed_connections: AtomicU64::new(0),
            bytes_uploaded: AtomicU64::new(0),
            bytes_downloaded: AtomicU64::new(0),
        }
    }

    fn snapshot(&self, local_addr: SocketAddr, running: bool) -> ProxySnapshot {
        ProxySnapshot {
            running,
            local_host: local_addr.ip().to_string(),
            local_port: local_addr.port(),
            started_at_unix: Some(self.started_at_unix),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_connections: self.total_connections.load(Ordering::Relaxed),
            failed_connections: self.failed_connections.load(Ordering::Relaxed),
            bytes_uploaded: self.bytes_uploaded.load(Ordering::Relaxed),
            bytes_downloaded: self.bytes_downloaded.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProxySnapshot {
    pub running: bool,
    pub local_host: String,
    pub local_port: u16,
    pub started_at_unix: Option<u64>,
    pub active_connections: usize,
    pub total_connections: u64,
    pub failed_connections: u64,
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
}

impl ProxySnapshot {
    pub fn stopped() -> Self {
        Self {
            running: false,
            local_host: String::new(),
            local_port: 0,
            started_at_unix: None,
            active_connections: 0,
            total_connections: 0,
            failed_connections: 0,
            bytes_uploaded: 0,
            bytes_downloaded: 0,
        }
    }
}

fn accept_loop(listener: TcpListener, config: ClientConfig, shared: Arc<ProxyShared>) {
    while !shared.stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _peer)) => {
                shared
                    .metrics
                    .total_connections
                    .fetch_add(1, Ordering::Relaxed);
                shared
                    .metrics
                    .active_connections
                    .fetch_add(1, Ordering::Relaxed);
                let connection_config = config.clone();
                let connection_shared = Arc::clone(&shared);
                if thread::Builder::new()
                    .name("wrongcl-socks5-conn".into())
                    .spawn(move || {
                        if handle_socks_client(stream, connection_config, &connection_shared)
                            .is_err()
                        {
                            connection_shared
                                .metrics
                                .failed_connections
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        connection_shared
                            .metrics
                            .active_connections
                            .fetch_sub(1, Ordering::Relaxed);
                    })
                    .is_err()
                {
                    shared
                        .metrics
                        .failed_connections
                        .fetch_add(1, Ordering::Relaxed);
                    shared
                        .metrics
                        .active_connections
                        .fetch_sub(1, Ordering::Relaxed);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn handle_socks_client(
    mut client: TcpStream,
    config: ClientConfig,
    shared: &ProxyShared,
) -> Result<()> {
    // The accept loop uses a nonblocking listener for shutdown polling. Some
    // platforms propagate that mode to accepted sockets, which breaks the
    // staged SOCKS/HTTP handshakes below.
    client.set_nonblocking(false)?;
    client.set_read_timeout(Some(SOCKS_HANDSHAKE_TIMEOUT))?;
    client.set_write_timeout(Some(SOCKS_HANDSHAKE_TIMEOUT))?;

    let request = match detect_local_proxy_request(&mut client) {
        Ok(request) => request,
        Err(e) if e.kind() == io::ErrorKind::Unsupported => {
            let _ = write_http_error(&mut client, "405 Method Not Allowed");
            return Err(ClientError::Io(e));
        }
        Err(e) => {
            if client.peek(&mut [0u8; 1]).ok() == Some(1) {
                let _ = write_http_error(&mut client, "400 Bad Request");
            } else {
                let _ = write_socks5_reply(&mut client, 0x01);
            }
            return Err(ClientError::Io(e));
        }
    };

    let tunnel_client = WrongsvClient::new(config.server.clone())?;
    match request {
        LocalProxyRequest::Socks(request) => match request {
            SocksRequest::Connect(target) => match tunnel_client.connect(&target) {
                Ok(upstream) => {
                    write_socks5_reply(&mut client, 0x00)?;
                    client.set_read_timeout(None)?;
                    client.set_write_timeout(None)?;
                    relay(client, upstream, &shared.metrics)
                }
                Err(e) => {
                    let _ = write_socks5_reply(&mut client, 0x05);
                    Err(e)
                }
            },
            SocksRequest::UdpAssociate => {
                if !tunnel_client.supports_udp() {
                    let _ = write_socks5_reply(&mut client, 0x07);
                    return Err(ClientError::UnsupportedProtocol(
                        "SOCKS5 UDP ASSOCIATE is not supported for this wrongcl profile".into(),
                    ));
                }
                relay_udp_associate(client, tunnel_client, &shared.metrics)
            }
        },
        LocalProxyRequest::Http(request) => match tunnel_client.connect(&request.target) {
            Ok(upstream) => {
                if request.connect {
                    write_http_connect_ok(&mut client)?;
                }
                client.set_read_timeout(None)?;
                client.set_write_timeout(None)?;
                relay_with_initial(client, upstream, &shared.metrics, &request.initial_bytes)
            }
            Err(e) => {
                let _ = write_http_error(&mut client, "502 Bad Gateway");
                Err(e)
            }
        },
    }
}

#[cfg(test)]
mod tests;
