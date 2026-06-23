use std::io::{self, Read, Write};
use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};

use crate::client::{Tunnel, UdpSession, WrongsvClient};
use crate::config::ClientConfig;
use crate::error::{ClientError, Result};
use crate::protocol::Target;
use crate::router::Decision;
use crate::source_app;

mod registry;
mod relay;
mod request;
mod requests;
mod udp;

const SOCKS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub use registry::{ConnFilter, ConnInfo, ConnRegistry, ConnState, RegistrySnapshot};
pub use requests::{RequestEntry, RequestLog, RequestRecord, global_request_log};

use relay::{relay, relay_with_initial};
use request::{
    LocalProxyRequest, SocksRequest, detect_local_proxy_request, write_http_connect_ok,
    write_http_error, write_socks5_reply,
};
use udp::relay_udp_associate;

pub struct ProxyHandle {
    local_addr: SocketAddr,
    shared: Arc<ProxyShared>,
    join: Option<JoinHandle<()>>,
}

impl ProxyHandle {
    pub fn start(config: ClientConfig) -> Result<Self> {
        config.validate()?;
        let listener = TcpListener::bind((config.local.host.as_str(), config.local.port))?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;

        let shared = Arc::new(ProxyShared {
            stop: AtomicBool::new(false),
            started_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            registry: Arc::new(ConnRegistry::new()),
            config: RwLock::new(config),
        });
        let accept_shared = Arc::clone(&shared);
        let join = thread::Builder::new()
            .name("wrongcl-socks5".into())
            .spawn(move || accept_loop(listener, accept_shared))
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

    pub fn registry(&self) -> Arc<ConnRegistry> {
        Arc::clone(&self.shared.registry)
    }

    pub fn groups_snapshot(&self) -> Value {
        let config = self.shared.config.read().expect("config rwlock poisoned");
        proxy_groups_json(&config)
    }

    pub fn select_group_member(&self, group: &str, member: &str) -> Result<()> {
        let mut config = self
            .shared
            .config
            .write()
            .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
        config.select_group_member(group, member)
    }

    pub fn dns_settings_json(&self) -> Value {
        let config = self.shared.config.read().expect("config rwlock poisoned");
        json!(config.dns)
    }

    pub fn set_dns_settings(&self, settings: crate::dns::DnsSettings) -> Result<()> {
        settings.validate()?;
        let mut config = self
            .shared
            .config
            .write()
            .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
        config.dns = settings;
        Ok(())
    }

    pub fn router_snapshot(&self) -> Value {
        let config = self.shared.config.read().expect("config rwlock poisoned");
        router_snapshot_json(&config)
    }

    pub fn set_active_mode(&self, name: &str) -> Result<()> {
        let mut config = self
            .shared
            .config
            .write()
            .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
        config.set_active_mode(name)
    }

    pub fn upsert_script(&self, script: crate::router::Script) -> Result<()> {
        let mut config = self
            .shared
            .config
            .write()
            .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
        config.upsert_script(script)
    }

    pub fn remove_script(&self, name: &str) -> Result<()> {
        let mut config = self
            .shared
            .config
            .write()
            .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
        config.remove_script(name)
    }

    pub fn upsert_user_mode(&self, mode: crate::config::Mode) -> Result<()> {
        let mut config = self
            .shared
            .config
            .write()
            .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
        config.upsert_user_mode(mode)
    }

    pub fn remove_user_mode(&self, name: &str) -> Result<()> {
        let mut config = self
            .shared
            .config
            .write()
            .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
        config.remove_user_mode(name)
    }
}

impl Drop for ProxyHandle {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

struct ProxyShared {
    stop: AtomicBool,
    started_at_unix: u64,
    registry: Arc<ConnRegistry>,
    config: RwLock<ClientConfig>,
}

impl ProxyShared {
    fn snapshot(&self, local_addr: SocketAddr, running: bool) -> ProxySnapshot {
        let snap = self.registry.snapshot();
        ProxySnapshot {
            running,
            local_host: local_addr.ip().to_string(),
            local_port: local_addr.port(),
            started_at_unix: Some(self.started_at_unix),
            active_connections: snap.active_connections,
            total_connections: snap.total_connections,
            failed_connections: snap.failed_connections,
            bytes_uploaded: snap.bytes_uploaded,
            bytes_downloaded: snap.bytes_downloaded,
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

fn accept_loop(listener: TcpListener, shared: Arc<ProxyShared>) {
    while !shared.stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, peer)) => {
                let close_clone = stream.try_clone().ok();
                let conn = shared.registry.register(peer, close_clone);
                if let Some((pid, app)) = source_app::lookup(peer) {
                    conn.set_source_pid(pid);
                    conn.set_source_app(app);
                }
                let connection_shared = Arc::clone(&shared);
                let connection_conn = Arc::clone(&conn);
                if thread::Builder::new()
                    .name("wrongcl-socks5-conn".into())
                    .spawn(move || {
                        let failed =
                            handle_socks_client(stream, &connection_shared, &connection_conn)
                                .is_err();
                        connection_shared
                            .registry
                            .retire(connection_conn.id, failed);
                    })
                    .is_err()
                {
                    shared.registry.retire(conn.id, true);
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
    shared: &Arc<ProxyShared>,
    conn: &Arc<ConnInfo>,
) -> Result<()> {
    // The accept loop uses a nonblocking listener for shutdown polling. Some
    // platforms propagate that mode to accepted sockets, which breaks the
    // staged SOCKS/HTTP handshakes below.
    client.set_nonblocking(false)?;
    client.set_read_timeout(Some(SOCKS_HANDSHAKE_TIMEOUT))?;
    client.set_write_timeout(Some(SOCKS_HANDSHAKE_TIMEOUT))?;
    let (allow_socks, allow_http) = {
        let config = shared
            .config
            .read()
            .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
        (config.local.allow_socks, config.local.allow_http)
    };

    let request = match detect_local_proxy_request(&mut client, allow_socks, allow_http) {
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

    match request {
        LocalProxyRequest::Socks(request) => match request {
            SocksRequest::Connect(target) => {
                let authority = format!("{}:{}", target.host, target.port);
                conn.set_target(authority.clone());
                conn.set_url(format!("CONNECT {authority}"));
                requests::global_request_log().record(RequestRecord {
                    conn_id: conn.id,
                    target: authority.clone(),
                    method: "CONNECT".into(),
                    url: None,
                    host: Some(authority),
                    source_pid: conn.source_pid(),
                    source_app: conn.source_app(),
                });
                let routed = resolve_routing(shared, &target.host)?;
                handle_tcp_connect(client, routed, &target, conn, RespondAs::Socks, &[])
            }
            SocksRequest::UdpAssociate => {
                let routed = resolve_routing_mode_only(shared)?;
                let tunnel_client = match routed {
                    RoutedTarget::Direct => {
                        let _ = write_socks5_reply(&mut client, 0x07);
                        return Err(ClientError::UnsupportedProtocol(
                            "Direct mode does not yet support SOCKS5 UDP ASSOCIATE".into(),
                        ));
                    }
                    RoutedTarget::Reject => {
                        let _ = write_socks5_reply(&mut client, 0x02);
                        return Err(ClientError::Config(
                            "routing mode rejects UDP ASSOCIATE".into(),
                        ));
                    }
                    RoutedTarget::Tunnel(t) => *t,
                };
                if !tunnel_client.supports_udp() {
                    let _ = write_socks5_reply(&mut client, 0x07);
                    return Err(ClientError::UnsupportedProtocol(
                        "SOCKS5 UDP ASSOCIATE is not supported for this wrongcl profile".into(),
                    ));
                }
                conn.set_target("udp-associate");
                conn.set_url("UDP ASSOCIATE");
                requests::global_request_log().record(RequestRecord {
                    conn_id: conn.id,
                    target: "udp-associate".into(),
                    method: "UDP-ASSOCIATE".into(),
                    url: None,
                    host: None,
                    source_pid: conn.source_pid(),
                    source_app: conn.source_app(),
                });
                conn.set_state(ConnState::Active);
                relay_udp_associate(client, tunnel_client, &conn.bytes_up, &conn.bytes_down)
            }
        },
        LocalProxyRequest::Http(request) => {
            let authority = format!("{}:{}", request.target.host, request.target.port);
            conn.set_target(authority.clone());
            let url = if request.connect {
                format!("CONNECT {}", request.request_target)
            } else {
                format!("{} {}", request.method, request.request_target)
            };
            conn.set_url(url.clone());
            let logged_url = if request.connect { None } else { Some(url) };
            requests::global_request_log().record(RequestRecord {
                conn_id: conn.id,
                target: authority,
                method: request.method.clone(),
                url: logged_url,
                host: request.host_header.clone(),
                source_pid: conn.source_pid(),
                source_app: conn.source_app(),
            });
            let routed = resolve_routing(shared, &request.target.host)?;
            let respond = if request.connect {
                RespondAs::HttpConnect
            } else {
                RespondAs::HttpInline
            };
            handle_tcp_connect(
                client,
                routed,
                &request.target,
                conn,
                respond,
                &request.initial_bytes,
            )
        }
    }
}

#[derive(Clone, Copy)]
enum RespondAs {
    Socks,
    HttpConnect,
    HttpInline,
}

fn handle_tcp_connect(
    mut client: TcpStream,
    routed: RoutedTarget,
    target: &Target,
    conn: &Arc<ConnInfo>,
    respond: RespondAs,
    initial: &[u8],
) -> Result<()> {
    match routed {
        RoutedTarget::Reject => {
            match respond {
                RespondAs::Socks => {
                    let _ = write_socks5_reply(&mut client, 0x02);
                }
                RespondAs::HttpConnect | RespondAs::HttpInline => {
                    let _ = write_http_error(&mut client, "403 Forbidden");
                }
            }
            Err(ClientError::Config(format!(
                "routing rejected connection to {}:{}",
                target.host, target.port
            )))
        }
        RoutedTarget::Direct => match connect_direct_tcp(target) {
            Ok(upstream) => {
                match respond {
                    RespondAs::Socks => write_socks5_reply(&mut client, 0x00)?,
                    RespondAs::HttpConnect => write_http_connect_ok(&mut client)?,
                    RespondAs::HttpInline => {}
                }
                client.set_read_timeout(None)?;
                client.set_write_timeout(None)?;
                conn.set_state(ConnState::Active);
                let boxed: Box<dyn Tunnel> = Box::new(upstream);
                if initial.is_empty() {
                    relay(client, boxed, &conn.bytes_up, &conn.bytes_down)
                } else {
                    relay_with_initial(client, boxed, &conn.bytes_up, &conn.bytes_down, initial)
                }
            }
            Err(e) => {
                match respond {
                    RespondAs::Socks => {
                        let _ = write_socks5_reply(&mut client, 0x05);
                    }
                    RespondAs::HttpConnect | RespondAs::HttpInline => {
                        let _ = write_http_error(&mut client, "502 Bad Gateway");
                    }
                }
                Err(e)
            }
        },
        RoutedTarget::Tunnel(tunnel_client) => match tunnel_client.connect(target) {
            Ok(upstream) => {
                match respond {
                    RespondAs::Socks => write_socks5_reply(&mut client, 0x00)?,
                    RespondAs::HttpConnect => write_http_connect_ok(&mut client)?,
                    RespondAs::HttpInline => {}
                }
                client.set_read_timeout(None)?;
                client.set_write_timeout(None)?;
                conn.set_state(ConnState::Active);
                if initial.is_empty() {
                    relay(client, upstream, &conn.bytes_up, &conn.bytes_down)
                } else {
                    relay_with_initial(client, upstream, &conn.bytes_up, &conn.bytes_down, initial)
                }
            }
            Err(e) => {
                match respond {
                    RespondAs::Socks => {
                        let _ = write_socks5_reply(&mut client, 0x05);
                    }
                    RespondAs::HttpConnect | RespondAs::HttpInline => {
                        let _ = write_http_error(&mut client, "502 Bad Gateway");
                    }
                }
                Err(e)
            }
        },
    }
}

enum RoutedTarget {
    Direct,
    Tunnel(Box<WrongsvClient>),
    Reject,
}

fn connect_direct_tcp(target: &Target) -> Result<TcpStream> {
    let addr = format!("{}:{}", target.host, target.port);
    TcpStream::connect(addr).map_err(|e| {
        ClientError::Io(io::Error::other(format!(
            "direct connect to {}:{} failed: {e}",
            target.host, target.port
        )))
    })
}

fn resolve_routing(shared: &ProxyShared, target_host: &str) -> Result<RoutedTarget> {
    let config = shared
        .config
        .read()
        .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
    let decision = config.decide(target_host);
    match decision {
        Decision::Direct => Ok(RoutedTarget::Direct),
        Decision::Reject => Ok(RoutedTarget::Reject),
        Decision::Proxy(name) => {
            let server = config.resolve_proxy_target(&name)?.server.clone();
            drop(config);
            let tunnel = WrongsvClient::new(server)?;
            Ok(RoutedTarget::Tunnel(Box::new(tunnel)))
        }
    }
}

fn resolve_routing_mode_only(shared: &ProxyShared) -> Result<RoutedTarget> {
    let config = shared
        .config
        .read()
        .map_err(|_| ClientError::Config("config rwlock poisoned".into()))?;
    let mode = config.modes.iter().find(|m| m.name == config.active_mode);
    let target_name = match mode {
        Some(m) => match m.kind {
            crate::config::ModeKind::Direct => return Ok(RoutedTarget::Direct),
            _ => m
                .proxy
                .clone()
                .unwrap_or_else(|| config.active_selection_name().to_string()),
        },
        None => config.active_selection_name().to_string(),
    };
    let server = config.resolve_proxy_target(&target_name)?.server.clone();
    drop(config);
    let tunnel = WrongsvClient::new(server)?;
    Ok(RoutedTarget::Tunnel(Box::new(tunnel)))
}

#[cfg(test)]
mod tests;

pub(crate) fn proxy_groups_json(config: &ClientConfig) -> Value {
    let active = match &config.active {
        crate::config::ActiveSelection::Endpoint { name } => {
            json!({ "type": "endpoint", "name": name })
        }
        crate::config::ActiveSelection::Group { name } => {
            json!({ "type": "group", "name": name })
        }
    };
    let endpoints: Vec<Value> = config
        .endpoints
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "host": e.server.host,
                "port": e.server.port,
                "stack": e.server.endpoint.stack_summary(),
                "proxy": e.server.endpoint.proxy.id(),
                "transport": e.server.endpoint.transport.id(),
                "outer_security": e.server.endpoint.outer_security.id(),
            })
        })
        .collect();
    let groups: Vec<Value> = config
        .groups
        .iter()
        .map(|g| {
            json!({
                "name": g.name,
                "kind": g.kind.as_str(),
                "members": g.members,
                "selected": g.selected,
            })
        })
        .collect();
    json!({
        "endpoints": endpoints,
        "groups": groups,
        "active": active,
    })
}

pub(crate) fn router_snapshot_json(config: &ClientConfig) -> Value {
    let modes: Vec<Value> = config
        .modes
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "kind": m.kind.as_str(),
                "proxy": m.proxy,
                "script": m.script,
            })
        })
        .collect();
    let scripts: Vec<Value> = config
        .scripts
        .iter()
        .map(|s| {
            json!({
                "name": s.name,
                "rules": s.rules,
            })
        })
        .collect();
    json!({
        "modes": modes,
        "scripts": scripts,
        "active_mode": config.active_mode,
    })
}
