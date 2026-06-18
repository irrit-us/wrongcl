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

const SOCKS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

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

enum LocalProxyRequest {
    Socks(SocksRequest),
    Http(HttpRequest),
}

struct HttpRequest {
    target: Target,
    connect: bool,
    initial_bytes: Vec<u8>,
}

fn detect_local_proxy_request(client: &mut TcpStream) -> io::Result<LocalProxyRequest> {
    let mut first = [0u8; 1];
    let n = client.peek(&mut first)?;
    if n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "client closed before proxy handshake",
        ));
    }
    if first[0] == 0x05 {
        return read_socks5_request(client).map(LocalProxyRequest::Socks);
    }
    read_http_proxy_request(client).map(LocalProxyRequest::Http)
}

enum SocksRequest {
    Connect(Target),
    UdpAssociate,
}

fn read_socks5_request(client: &mut TcpStream) -> io::Result<SocksRequest> {
    let mut greeting = [0u8; 2];
    client.read_exact(&mut greeting)?;
    if greeting[0] != 0x05 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported SOCKS version",
        ));
    }

    let method_count = greeting[1] as usize;
    let mut methods = vec![0u8; method_count];
    client.read_exact(&mut methods)?;
    if !methods.contains(&0x00) {
        client.write_all(&[0x05, 0xff])?;
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "SOCKS client did not offer no-auth method",
        ));
    }
    client.write_all(&[0x05, 0x00])?;

    let mut request = [0u8; 4];
    client.read_exact(&mut request)?;
    if request[0] != 0x05 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid SOCKS request version",
        ));
    }

    let host = match request[3] {
        0x01 => {
            let mut octets = [0u8; 4];
            client.read_exact(&mut octets)?;
            Ipv4Addr::from(octets).to_string()
        }
        0x03 => {
            let mut len = [0u8; 1];
            client.read_exact(&mut len)?;
            let mut domain = vec![0u8; len[0] as usize];
            client.read_exact(&mut domain)?;
            String::from_utf8(domain)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid domain name"))?
        }
        0x04 => {
            let mut octets = [0u8; 16];
            client.read_exact(&mut octets)?;
            Ipv6Addr::from(octets).to_string()
        }
        other => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported SOCKS address type: {other}"),
            ));
        }
    };

    let mut port = [0u8; 2];
    client.read_exact(&mut port)?;
    let port = u16::from_be_bytes(port);
    match request[1] {
        0x01 => Target::new(host, port)
            .map(SocksRequest::Connect)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string())),
        0x03 => Ok(SocksRequest::UdpAssociate),
        other => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported SOCKS5 command {other:#04x}"),
        )),
    }
}

fn read_http_proxy_request(client: &mut TcpStream) -> io::Result<HttpRequest> {
    let mut buf = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        client.read_exact(&mut byte)?;
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if buf.len() > 8 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP CONNECT request headers too large",
            ));
        }
    }

    let text = String::from_utf8(buf)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP request"))?;
    let mut lines = text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let request_target = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or("HTTP/1.1");
    if method != "CONNECT" {
        let (target, head) = rewrite_http_forward_request(method, request_target, version, lines)?;
        return Ok(HttpRequest {
            target,
            connect: false,
            initial_bytes: head,
        });
    }
    Ok(HttpRequest {
        target: parse_connect_authority(request_target)?,
        connect: true,
        initial_bytes: Vec::new(),
    })
}

fn rewrite_http_forward_request<'a>(
    method: &str,
    request_target: &str,
    version: &str,
    lines: impl Iterator<Item = &'a str>,
) -> io::Result<(Target, Vec<u8>)> {
    let mut header_lines = Vec::new();
    let mut host_header: Option<String> = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("proxy-connection:") {
            continue;
        }
        if lower.starts_with("host:") {
            host_header = Some(line[5..].trim().to_string());
        }
        header_lines.push(line.to_string());
    }

    let (target, path) =
        if request_target.starts_with("http://") || request_target.starts_with("https://") {
            let uri: http::Uri = request_target.parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid absolute-form URI")
            })?;
            let host = uri
                .host()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "URI missing host"))?;
            let port = uri.port_u16().unwrap_or(80);
            let target = Target::new(host, port)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
            let path = uri
                .path_and_query()
                .map(|value| value.as_str().to_string())
                .unwrap_or_else(|| "/".to_string());
            (target, path)
        } else if request_target.starts_with('/') {
            let host = host_header.clone().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "HTTP request missing Host header",
                )
            })?;
            (parse_host_header(&host)?, request_target.to_string())
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported HTTP proxy request-target",
            ));
        };

    let mut out = Vec::new();
    out.extend_from_slice(format!("{method} {path} {version}\r\n").as_bytes());
    let has_host_header = header_lines
        .iter()
        .any(|line| line.to_ascii_lowercase().starts_with("host:"));
    if !has_host_header {
        out.extend_from_slice(format!("Host: {}\r\n", target.host).as_bytes());
    }
    for line in header_lines {
        out.extend_from_slice(line.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    Ok((target, out))
}

fn parse_connect_authority(authority: &str) -> io::Result<Target> {
    if authority.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing CONNECT authority",
        ));
    }
    let (host, port) = if authority.starts_with('[') {
        let end = authority
            .find("]:")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid IPv6 authority"))?;
        let host = &authority[1..end];
        let port = &authority[end + 2..];
        (host, port)
    } else {
        authority.rsplit_once(':').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "CONNECT authority must include host:port",
            )
        })?
    };
    let port = port
        .parse::<u16>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid CONNECT port"))?;
    Target::new(host, port).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
}

fn parse_host_header(host: &str) -> io::Result<Target> {
    let host = host.trim();
    if host.starts_with('[') {
        let end = host.find(']').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid IPv6 Host header")
        })?;
        let addr = &host[1..end];
        let port = host[end + 1..]
            .strip_prefix(':')
            .map(|value| value.parse::<u16>())
            .transpose()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid Host header port"))?
            .unwrap_or(80);
        return Target::new(addr, port)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()));
    }
    if let Some((host_only, port)) = host.rsplit_once(':') {
        if !host_only.contains(':') {
            let port = port.parse::<u16>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Host header port")
            })?;
            return Target::new(host_only, port)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()));
        }
    }
    Target::new(host, 80).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
}

fn write_socks5_reply(client: &mut TcpStream, reply: u8) -> io::Result<()> {
    client.write_all(&[0x05, reply, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
}

fn write_http_connect_ok(client: &mut TcpStream) -> io::Result<()> {
    client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
}

fn write_http_error(client: &mut TcpStream, status: &str) -> io::Result<()> {
    let response = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    client.write_all(response.as_bytes())
}

fn write_socks5_reply_addr(client: &mut TcpStream, reply: u8, addr: SocketAddr) -> io::Result<()> {
    let mut out = vec![0x05, reply, 0x00];
    match addr.ip() {
        IpAddr::V4(ip) => {
            out.push(0x01);
            out.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            out.push(0x04);
            out.extend_from_slice(&ip.octets());
        }
    }
    out.extend_from_slice(&addr.port().to_be_bytes());
    client.write_all(&out)
}

fn relay_udp_associate(
    mut client: TcpStream,
    tunnel_client: WrongsvClient,
    metrics: &ProxyMetrics,
) -> Result<()> {
    let bind_ip = client.local_addr()?.ip();
    let udp_socket = UdpSocket::bind((bind_ip, 0))?;
    udp_socket.set_read_timeout(Some(Duration::from_millis(20)))?;
    let udp_addr = udp_socket.local_addr()?;
    write_socks5_reply_addr(&mut client, 0x00, udp_addr)?;
    client.set_read_timeout(Some(Duration::from_millis(20)))?;

    let mut client_peer: Option<SocketAddr> = None;
    let mut sessions: std::collections::HashMap<Target, Box<dyn UdpSession>> =
        std::collections::HashMap::new();

    loop {
        if !control_connection_alive(&client)? {
            break;
        }

        let mut did_work = false;
        let mut buf = [0u8; 65535];
        match udp_socket.recv_from(&mut buf) {
            Ok((n, peer)) => {
                client_peer = Some(peer);
                if let Ok((target, payload)) = parse_socks5_udp_datagram(&buf[..n]) {
                    if !sessions.contains_key(&target) {
                        let session = tunnel_client.connect_udp_session(&target)?;
                        sessions.insert(target.clone(), session);
                    }
                    if let Some(session) = sessions.get_mut(&target) {
                        session.send_packet(&payload)?;
                        metrics
                            .bytes_uploaded
                            .fetch_add(payload.len() as u64, Ordering::Relaxed);
                        did_work = true;
                    }
                }
            }
            Err(ref e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(ClientError::Io(e)),
        }

        if let Some(peer) = client_peer {
            for session in sessions.values_mut() {
                while let Some(packet) = session.try_recv_packet()? {
                    let payload = encode_socks5_udp_datagram(&packet.target, &packet.payload)?;
                    udp_socket.send_to(&payload, peer)?;
                    metrics
                        .bytes_downloaded
                        .fetch_add(packet.payload.len() as u64, Ordering::Relaxed);
                    did_work = true;
                }
            }
        }

        if !did_work {
            thread::sleep(Duration::from_millis(10));
        }
    }

    Ok(())
}

fn control_connection_alive(client: &TcpStream) -> io::Result<bool> {
    let mut byte = [0u8; 1];
    match client.peek(&mut byte) {
        Ok(0) => Ok(false),
        Ok(_) => Ok(true),
        Err(ref e)
            if matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) =>
        {
            Ok(true)
        }
        Err(e) => Err(e),
    }
}

fn parse_socks5_udp_datagram(packet: &[u8]) -> io::Result<(Target, Vec<u8>)> {
    if packet.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SOCKS5 UDP packet too short",
        ));
    }
    if packet[0] != 0 || packet[1] != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SOCKS5 UDP reserved bytes must be zero",
        ));
    }
    if packet[2] != 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "SOCKS5 UDP fragmentation is not supported",
        ));
    }

    let (target, header_len) = parse_socks5_target(&packet[3..])?;
    Ok((target, packet[3 + header_len..].to_vec()))
}

fn encode_socks5_udp_datagram(target: &Target, payload: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = vec![0x00, 0x00, 0x00];
    write_socks5_target(&mut out, target)?;
    out.extend_from_slice(payload);
    Ok(out)
}

fn parse_socks5_target(data: &[u8]) -> io::Result<(Target, usize)> {
    let atyp = *data
        .first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing SOCKS5 address type"))?;
    match atyp {
        0x01 => {
            if data.len() < 1 + 4 + 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "short SOCKS5 IPv4 address",
                ));
            }
            let host = Ipv4Addr::from([data[1], data[2], data[3], data[4]]).to_string();
            let port = u16::from_be_bytes([data[5], data[6]]);
            Ok((
                Target::new(host, port)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?,
                7,
            ))
        }
        0x03 => {
            let len = *data.get(1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "short SOCKS5 domain length")
            })? as usize;
            if data.len() < 2 + len + 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "short SOCKS5 domain address",
                ));
            }
            let host = String::from_utf8(data[2..2 + len].to_vec())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid domain name"))?;
            let port = u16::from_be_bytes([data[2 + len], data[3 + len]]);
            Ok((
                Target::new(host, port)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?,
                4 + len,
            ))
        }
        0x04 => {
            if data.len() < 1 + 16 + 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "short SOCKS5 IPv6 address",
                ));
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&data[1..17]);
            let host = Ipv6Addr::from(octets).to_string();
            let port = u16::from_be_bytes([data[17], data[18]]);
            Ok((
                Target::new(host, port)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?,
                19,
            ))
        }
        other => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported SOCKS5 address type {other:#04x}"),
        )),
    }
}

fn write_socks5_target(out: &mut Vec<u8>, target: &Target) -> io::Result<()> {
    let host = target.host.trim();
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        out.push(0x01);
        out.extend_from_slice(&ip.octets());
    } else if let Ok(ip) = host.parse::<Ipv6Addr>() {
        out.push(0x04);
        out.extend_from_slice(&ip.octets());
    } else {
        if host.is_empty() || host.len() > u8::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SOCKS5 domain must be 1..255 bytes",
            ));
        }
        out.push(0x03);
        out.push(host.len() as u8);
        out.extend_from_slice(host.as_bytes());
    }
    out.extend_from_slice(&target.port.to_be_bytes());
    Ok(())
}

fn relay(client: TcpStream, upstream: Box<dyn Tunnel>, metrics: &ProxyMetrics) -> Result<()> {
    relay_with_initial(client, upstream, metrics, &[])
}

fn relay_with_initial(
    mut client: TcpStream,
    upstream: Box<dyn Tunnel>,
    metrics: &ProxyMetrics,
    initial_upload: &[u8],
) -> Result<()> {
    let (mut upstream_reader, mut upstream_writer) = upstream.split_box()?;
    let mut client_writer = client.try_clone()?;
    let download_counter = &metrics.bytes_downloaded;
    let downstream = thread::scope(|scope| {
        let downstream = scope.spawn(move || {
            let _ = copy_counted(&mut upstream_reader, &mut client_writer, download_counter);
            let _ = client_writer.shutdown(Shutdown::Write);
        });

        let upload_result = (|| -> io::Result<u64> {
            if !initial_upload.is_empty() {
                upstream_writer.write_all(initial_upload)?;
                metrics
                    .bytes_uploaded
                    .fetch_add(initial_upload.len() as u64, Ordering::Relaxed);
            }
            copy_counted(&mut client, &mut upstream_writer, &metrics.bytes_uploaded)
        })();
        let _ = upstream_writer.shutdown_write();
        let _ = downstream.join();
        upload_result
    });

    downstream.map(|_| ()).map_err(ClientError::Io)
}

fn copy_counted(
    reader: &mut impl Read,
    writer: &mut impl Write,
    counter: &AtomicU64,
) -> io::Result<u64> {
    let mut buf = [0u8; 16 * 1024];
    let mut total = 0u64;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return Ok(total),
            Ok(n) => {
                writer.write_all(&buf[..n])?;
                total += n as u64;
                counter.fetch_add(n as u64, Ordering::Relaxed);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LocalProxyConfig, ServerConfig};
    use crate::endpoint::{Endpoint, OuterSecurity, ProxyProtocol, ShadowsocksOptions, Transport};
    use base64::Engine as _;
    use std::sync::mpsc;

    const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

    #[test]
    fn socks_proxy_relays_and_tracks_metrics() {
        let server = spawn_fake_vless_server();
        let config =
            ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
        let mut proxy = ProxyHandle::start(config).unwrap();

        let response = run_socks_echo(proxy.snapshot().socket_addr()).unwrap();
        let snapshot = wait_for_inactive(&proxy);
        proxy.stop().unwrap();

        assert_eq!(response, b"hello".to_vec());
        assert_eq!(snapshot.active_connections, 0);
        assert_eq!(snapshot.total_connections, 1);
        assert_eq!(snapshot.failed_connections, 0);
        assert!(snapshot.bytes_uploaded >= 5);
        assert!(snapshot.bytes_downloaded >= 5);
    }

    #[test]
    fn http_connect_proxy_relays_and_tracks_metrics() {
        let server = spawn_fake_vless_server();
        let config =
            ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
        let mut proxy = ProxyHandle::start(config).unwrap();

        let response = run_http_connect_echo(proxy.snapshot().socket_addr()).unwrap();
        let snapshot = wait_for_inactive(&proxy);
        proxy.stop().unwrap();

        assert_eq!(response, b"hello".to_vec());
        assert_eq!(snapshot.active_connections, 0);
        assert_eq!(snapshot.failed_connections, 0);
        assert!(snapshot.bytes_uploaded >= 5);
        assert!(snapshot.bytes_downloaded >= 5);
    }

    #[test]
    fn socks_handshake_survives_nonblocking_client_socket() {
        let server = spawn_fake_vless_server();
        let config =
            ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let shared = ProxyShared {
                stop: AtomicBool::new(false),
                metrics: ProxyMetrics::new(),
            };
            let (stream, _) = listener.accept().unwrap();
            stream.set_nonblocking(true).unwrap();
            handle_socks_client(stream, config, &shared)
        });

        let response = run_socks_echo(local_addr).unwrap();
        let result = worker.join().unwrap();

        assert_eq!(response, b"hello".to_vec());
        result.unwrap();
    }

    #[test]
    fn socks_proxy_relays_through_remote_http_connect_backend() {
        let backend = spawn_fake_http_connect_backend(None, None);
        let config = ClientConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: backend.port,
                endpoint: Endpoint {
                    proxy: ProxyProtocol::Mixed(crate::endpoint::MixedOptions {
                        username: None,
                        password: None,
                    }),
                    transport: Transport::Raw,
                    outer_security: OuterSecurity::None,
                },
            },
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
        };
        let mut proxy = ProxyHandle::start(config).unwrap();

        let response = run_socks_echo(proxy.snapshot().socket_addr()).unwrap();
        proxy.stop().unwrap();

        assert_eq!(response, b"hello".to_vec());
    }

    #[test]
    fn http_proxy_rejects_non_connect_requests() {
        let server = spawn_fake_vless_server();
        let config =
            ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
        let mut proxy = ProxyHandle::start(config).unwrap();

        let response = run_http_get_rejected(proxy.snapshot().socket_addr()).unwrap();
        proxy.stop().unwrap();

        assert!(response.starts_with("HTTP/1.1 405 Method Not Allowed"));
    }

    #[test]
    fn http_proxy_forwards_absolute_form_requests() {
        let server = spawn_fake_vless_server();
        let config =
            ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
        let mut proxy = ProxyHandle::start(config).unwrap();

        let echoed = run_http_absolute_form(proxy.snapshot().socket_addr()).unwrap();
        proxy.stop().unwrap();

        let text = String::from_utf8_lossy(&echoed);
        assert!(text.starts_with("GET /hello?x=1 HTTP/1.1\r\n"), "{text}");
        assert!(text.contains("\r\nHost: example.com\r\n"), "{text}");
        assert!(
            !text.to_ascii_lowercase().contains("proxy-connection"),
            "{text}"
        );
    }

    #[test]
    fn socks_proxy_relays_udp_associate() {
        let server = spawn_fake_vless_udp_server();
        let config =
            ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
        let mut proxy = ProxyHandle::start(config).unwrap();

        let response = run_socks_udp_echo(proxy.snapshot().socket_addr()).unwrap();
        proxy.stop().unwrap();

        assert_eq!(response, b"ping-udp".to_vec());
    }

    #[test]
    fn socks_proxy_relays_shadowsocks_udp() {
        let server =
            spawn_fake_shadowsocks_udp_server("chacha20-ietf-poly1305".into(), "hunter2".into());
        let mut proxy = ProxyHandle::start(shadowsocks_client_config(
            server.port,
            "chacha20-ietf-poly1305",
            "hunter2",
        ))
        .unwrap();

        let response = run_socks_udp_echo(proxy.snapshot().socket_addr()).unwrap();
        proxy.stop().unwrap();

        assert_eq!(response, b"ping-udp".to_vec());
    }

    #[test]
    fn socks_proxy_relays_shadowsocks_aead_2022_udp() {
        let psk_b64 = "AAAAAAAAAAAAAAAAAAAAAA==";
        let server =
            spawn_fake_shadowsocks_udp_server("2022-blake3-aes-128-gcm".into(), psk_b64.into());
        let mut proxy = ProxyHandle::start(shadowsocks_client_config(
            server.port,
            "2022-blake3-aes-128-gcm",
            psk_b64,
        ))
        .unwrap();

        let response = run_socks_udp_echo(proxy.snapshot().socket_addr()).unwrap();
        proxy.stop().unwrap();

        assert_eq!(response, b"ping-udp".to_vec());
    }

    fn wait_for_inactive(proxy: &ProxyHandle) -> ProxySnapshot {
        for _ in 0..40 {
            let snapshot = proxy.snapshot();
            if snapshot.active_connections == 0 {
                return snapshot;
            }
            thread::sleep(Duration::from_millis(25));
        }
        proxy.snapshot()
    }

    impl ProxySnapshot {
        fn socket_addr(&self) -> SocketAddr {
            format!("{}:{}", self.local_host, self.local_port)
                .parse()
                .unwrap()
        }
    }

    struct FakeServer {
        port: u16,
    }

    fn shadowsocks_client_config(port: u16, method: &str, password: &str) -> ClientConfig {
        ClientConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port,
                endpoint: Endpoint {
                    proxy: ProxyProtocol::Shadowsocks(ShadowsocksOptions {
                        method: method.into(),
                        password: password.into(),
                    }),
                    transport: Transport::Raw,
                    outer_security: OuterSecurity::None,
                },
            },
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
        }
    }

    fn spawn_fake_vless_server() -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            ready_tx.send(()).unwrap();
            for stream in listener.incoming().flatten() {
                thread::spawn(move || {
                    let _ = handle_fake_vless(stream);
                });
            }
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        FakeServer { port }
    }

    fn spawn_fake_vless_udp_server() -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            ready_tx.send(()).unwrap();
            for stream in listener.incoming().flatten() {
                thread::spawn(move || {
                    let _ = handle_fake_vless_udp(stream);
                });
            }
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        FakeServer { port }
    }

    fn spawn_fake_shadowsocks_udp_server(method: String, password: String) -> FakeServer {
        use wrongsv_shadowsocks::{
            decrypt_aead_2022_udp_request, decrypt_udp_packet, encrypt_aead_2022_udp_response,
            encrypt_udp_packet, ServerConfig as SsServerConfig,
        };

        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            ready_tx.send(()).unwrap();
            let config = SsServerConfig::new(&method, password).unwrap();
            let mut buf = [0u8; 65535];
            loop {
                let Ok((n, peer)) = socket.recv_from(&mut buf) else {
                    return;
                };
                let response = if config.method.is_aead_2022() {
                    let request = decrypt_aead_2022_udp_request(&buf[..n], &config).unwrap();
                    encrypt_aead_2022_udp_response(
                        &config,
                        [0x11; 8],
                        request.packet_id,
                        request.client_session_id,
                        &request.address,
                        request.port,
                        &request.payload,
                    )
                    .unwrap()
                } else {
                    let plaintext = decrypt_udp_packet(&buf[..n], &config).unwrap();
                    encrypt_udp_packet(&plaintext, &config).unwrap()
                };
                let _ = socket.send_to(&response, peer);
            }
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        FakeServer { port }
    }

    fn spawn_fake_http_connect_backend(
        username: Option<&str>,
        password: Option<&str>,
    ) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let username = username.map(str::to_string);
        let password = password.map(str::to_string);
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            ready_tx.send(()).unwrap();
            for stream in listener.incoming().flatten() {
                let username = username.clone();
                let password = password.clone();
                thread::spawn(move || {
                    let _ = handle_fake_http_connect_backend(
                        stream,
                        username.as_deref(),
                        password.as_deref(),
                    );
                });
            }
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        FakeServer { port }
    }

    fn handle_fake_vless(mut stream: TcpStream) -> io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let mut fixed = [0u8; 19];
        stream.read_exact(&mut fixed)?;
        let addons_len = fixed[17] as usize;
        if addons_len > 0 {
            let mut addons = vec![0u8; addons_len];
            stream.read_exact(&mut addons)?;
        }

        let mut port = [0u8; 2];
        stream.read_exact(&mut port)?;
        let mut atyp = [0u8; 1];
        stream.read_exact(&mut atyp)?;
        match atyp[0] {
            0x01 => {
                let mut addr = [0u8; 4];
                stream.read_exact(&mut addr)?;
            }
            0x02 => {
                let mut len = [0u8; 1];
                stream.read_exact(&mut len)?;
                let mut domain = vec![0u8; len[0] as usize];
                stream.read_exact(&mut domain)?;
            }
            0x03 => {
                let mut addr = [0u8; 16];
                stream.read_exact(&mut addr)?;
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected address type {other}"),
                ));
            }
        }

        stream.write_all(&[0x00, 0x00])?;
        let mut buf = [0u8; 1024];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => return Ok(()),
                Ok(n) => stream.write_all(&buf[..n])?,
                Err(e) => return Err(e),
            }
        }
    }

    fn handle_fake_vless_udp(mut stream: TcpStream) -> io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let mut fixed = [0u8; 19];
        stream.read_exact(&mut fixed)?;
        let addons_len = fixed[17] as usize;
        if addons_len > 0 {
            let mut addons = vec![0u8; addons_len];
            stream.read_exact(&mut addons)?;
        }
        assert_eq!(fixed[18], 0x02, "expected VLESS UDP command");

        let mut port = [0u8; 2];
        stream.read_exact(&mut port)?;
        let mut atyp = [0u8; 1];
        stream.read_exact(&mut atyp)?;
        read_fake_address(&mut stream, atyp[0])?;

        stream.write_all(&[0x00, 0x00])?;
        loop {
            let mut len_buf = [0u8; 2];
            if stream.read_exact(&mut len_buf).is_err() {
                return Ok(());
            }
            let len = u16::from_be_bytes(len_buf) as usize;
            let mut packet = vec![0u8; len];
            if stream.read_exact(&mut packet).is_err() {
                return Ok(());
            }
            stream.write_all(&len_buf)?;
            stream.write_all(&packet)?;
        }
    }

    fn read_fake_address(reader: &mut impl Read, atyp: u8) -> io::Result<()> {
        match atyp {
            0x01 => {
                let mut addr = [0u8; 4];
                reader.read_exact(&mut addr)?;
            }
            0x02 => {
                let mut len = [0u8; 1];
                reader.read_exact(&mut len)?;
                let mut domain = vec![0u8; len[0] as usize];
                reader.read_exact(&mut domain)?;
            }
            0x03 => {
                let mut addr = [0u8; 16];
                reader.read_exact(&mut addr)?;
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected address type {other}"),
                ));
            }
        }
        Ok(())
    }

    fn handle_fake_http_connect_backend(
        mut stream: TcpStream,
        username: Option<&str>,
        password: Option<&str>,
    ) -> io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let request = read_http_headers_from_stream(&mut stream)?;
        let mut lines = request.split("\r\n");
        assert_eq!(
            lines.next().unwrap_or_default(),
            "CONNECT example.com:80 HTTP/1.1"
        );
        let auth_header = lines.find(|line| {
            line.to_ascii_lowercase()
                .starts_with("proxy-authorization:")
        });
        if username.is_some() || password.is_some() {
            let expected = base64::engine::general_purpose::STANDARD.encode(format!(
                "{}:{}",
                username.unwrap_or(""),
                password.unwrap_or("")
            ));
            let expected_line = format!("Proxy-Authorization: Basic {expected}");
            assert_eq!(auth_header, Some(expected_line.as_str()));
        } else {
            assert!(auth_header.is_none());
        }
        stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
        let mut buf = [0u8; 1024];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => return Ok(()),
                Ok(n) => stream.write_all(&buf[..n])?,
                Err(e) => return Err(e),
            }
        }
    }

    fn read_http_headers_from_stream(stream: &mut impl Read) -> io::Result<String> {
        let mut buf = Vec::with_capacity(512);
        let mut byte = [0u8; 1];
        loop {
            stream.read_exact(&mut byte)?;
            buf.push(byte[0]);
            if buf.ends_with(b"\r\n\r\n") {
                return String::from_utf8(buf).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP request")
                });
            }
            if buf.len() > 8 * 1024 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "HTTP request headers too large",
                ));
            }
        }
    }

    fn run_socks_echo(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
        let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.write_all(&[0x05, 0x01, 0x00])?;

        let mut greeting = [0u8; 2];
        stream.read_exact(&mut greeting)?;
        assert_eq!(greeting, [0x05, 0x00]);

        let host = b"example.com";
        let mut request = vec![0x05, 0x01, 0x00, 0x03, host.len() as u8];
        request.extend_from_slice(host);
        request.extend_from_slice(&80u16.to_be_bytes());
        stream.write_all(&request)?;

        let mut reply = [0u8; 10];
        stream.read_exact(&mut reply)?;
        assert_eq!(reply[1], 0x00);

        stream.write_all(b"hello")?;
        let mut response = [0u8; 5];
        stream.read_exact(&mut response)?;
        Ok(response.to_vec())
    }

    fn run_http_connect_echo(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
        let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.write_all(b"CONNECT example.com:80 HTTP/1.1\r\nHost: example.com:80\r\n\r\n")?;

        let mut response = Vec::with_capacity(128);
        let mut byte = [0u8; 1];
        loop {
            stream.read_exact(&mut byte)?;
            response.push(byte[0]);
            if response.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        assert!(std::str::from_utf8(&response)
            .unwrap()
            .starts_with("HTTP/1.1 200 Connection Established"),);

        stream.write_all(b"hello")?;
        let mut echoed = [0u8; 5];
        stream.read_exact(&mut echoed)?;
        Ok(echoed.to_vec())
    }

    fn run_http_get_rejected(local_addr: SocketAddr) -> io::Result<String> {
        let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.write_all(b"GET ftp://example.com/ HTTP/1.1\r\nHost: example.com\r\n\r\n")?;

        let mut response = Vec::with_capacity(128);
        let mut byte = [0u8; 1];
        loop {
            match stream.read_exact(&mut byte) {
                Ok(()) => {
                    response.push(byte[0]);
                    if response.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
        }
        Ok(String::from_utf8_lossy(&response).to_string())
    }

    fn run_http_absolute_form(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
        let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.write_all(
            b"GET http://example.com/hello?x=1 HTTP/1.1\r\nHost: example.com\r\nProxy-Connection: keep-alive\r\n\r\n",
        )?;

        let mut response = vec![0u8; 256];
        let n = stream.read(&mut response)?;
        response.truncate(n);
        Ok(response)
    }

    fn run_socks_udp_echo(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
        let mut control = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
        control.set_read_timeout(Some(Duration::from_secs(3)))?;
        control.write_all(&[0x05, 0x01, 0x00])?;

        let mut greeting = [0u8; 2];
        control.read_exact(&mut greeting)?;
        assert_eq!(greeting, [0x05, 0x00]);

        control.write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])?;
        let mut reply = [0u8; 10];
        control.read_exact(&mut reply)?;
        assert_eq!(reply[1], 0x00);
        let relay_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7])),
            u16::from_be_bytes([reply[8], reply[9]]),
        );

        let udp = UdpSocket::bind("127.0.0.1:0")?;
        udp.set_read_timeout(Some(Duration::from_secs(3)))?;

        let payload = b"ping-udp";
        let mut packet = vec![0x00, 0x00, 0x00, 0x03, 11];
        packet.extend_from_slice(b"example.com");
        packet.extend_from_slice(&53u16.to_be_bytes());
        packet.extend_from_slice(payload);
        udp.send_to(&packet, relay_addr)?;

        let mut buf = [0u8; 1024];
        let (n, _) = udp.recv_from(&mut buf)?;
        assert_eq!(&buf[..3], &[0x00, 0x00, 0x00]);
        let (_, header_len) = parse_socks5_target(&buf[3..])?;
        Ok(buf[3 + header_len..n].to_vec())
    }
}
