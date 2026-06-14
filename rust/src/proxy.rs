use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::client::{Tunnel, WrongsvClient};
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
            .map_err(|e| {
                ClientError::Io(io::Error::new(
                    io::ErrorKind::Other,
                    format!("start proxy thread: {e}"),
                ))
            })?;

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
                ClientError::Io(io::Error::new(
                    io::ErrorKind::Other,
                    "proxy thread panicked while stopping",
                ))
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
    client.set_read_timeout(Some(SOCKS_HANDSHAKE_TIMEOUT))?;
    client.set_write_timeout(Some(SOCKS_HANDSHAKE_TIMEOUT))?;

    let target = match read_socks5_connect(&mut client) {
        Ok(target) => target,
        Err(e) => {
            let _ = write_socks5_reply(&mut client, 0x01);
            return Err(ClientError::Io(e));
        }
    };

    let tunnel_client = WrongsvClient::new(config.server.clone())?;
    match tunnel_client.connect(&target) {
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
    }
}

fn read_socks5_connect(client: &mut TcpStream) -> io::Result<Target> {
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
    if request[1] != 0x01 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "only SOCKS5 CONNECT is supported",
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
    Target::new(host, u16::from_be_bytes(port))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
}

fn write_socks5_reply(client: &mut TcpStream, reply: u8) -> io::Result<()> {
    client.write_all(&[0x05, reply, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
}

fn relay(
    mut client: TcpStream,
    mut upstream: Box<dyn Tunnel>,
    metrics: &ProxyMetrics,
) -> Result<()> {
    let mut upstream_reader = upstream.try_clone_box()?;
    let mut client_writer = client.try_clone()?;
    let download_counter = &metrics.bytes_downloaded;
    let downstream = thread::scope(|scope| {
        let downstream = scope.spawn(move || {
            let _ = copy_counted(&mut upstream_reader, &mut client_writer, download_counter);
            let _ = client_writer.shutdown(Shutdown::Write);
        });

        let upload_result = copy_counted(&mut client, &mut upstream, &metrics.bytes_uploaded);
        let _ = upstream.shutdown_write();
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
    use std::sync::mpsc;

    const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

    #[test]
    fn socks_proxy_relays_and_tracks_metrics() {
        let server = spawn_fake_vless_server();
        let config =
            ClientConfig::new("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
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
}
