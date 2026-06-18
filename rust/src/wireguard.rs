use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::client::{Tunnel, TunnelReader, TunnelWriter, UdpPacket, UdpSession, WrongsvClient};
use crate::config::ServerConfig;
use crate::endpoint::{
    Endpoint, MixedOptions, OuterSecurity, ProxyProtocol, Transport, WireGuardOptions,
};
use crate::error::{ClientError, Result};
use crate::protocol::Target;

const HELPER_START_TIMEOUT: Duration = Duration::from_secs(15);

static RUNTIME_CACHE: OnceLock<Mutex<HashMap<String, Arc<WireGuardRuntime>>>> = OnceLock::new();

pub fn connect_wireguard(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
    target: &Target,
) -> Result<Box<dyn Tunnel>> {
    let runtime = acquire_runtime(server_host, server_port, opts)?;
    let helper_client = runtime.helper_client()?;
    let inner = helper_client.connect(target)?;
    Ok(Box::new(HelperTunnel { inner, runtime }))
}

pub fn connect_wireguard_udp(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
    target: &Target,
) -> Result<Box<dyn UdpSession>> {
    let runtime = acquire_runtime(server_host, server_port, opts)?;
    let helper_client = runtime.helper_client()?;
    let inner = helper_client.connect_udp_session(target)?;
    Ok(Box::new(HelperUdpSession { inner, runtime }))
}

fn acquire_runtime(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
) -> Result<Arc<WireGuardRuntime>> {
    let key = runtime_key(server_host, server_port, opts)?;
    let cache = RUNTIME_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache
        .lock()
        .map_err(|_| ClientError::Config("wireguard runtime cache lock is poisoned".into()))?;

    if let Some(runtime) = guard.get(&key) {
        if runtime.is_alive()? {
            return Ok(Arc::clone(runtime));
        }
        guard.remove(&key);
    }

    let runtime = Arc::new(WireGuardRuntime::spawn(server_host, server_port, opts)?);
    guard.insert(key, Arc::clone(&runtime));
    Ok(runtime)
}

fn runtime_key(server_host: &str, server_port: u16, opts: &WireGuardOptions) -> Result<String> {
    #[derive(Serialize)]
    struct RuntimeKey<'a> {
        server_host: &'a str,
        server_port: u16,
        options: &'a WireGuardOptions,
    }

    serde_json::to_string(&RuntimeKey {
        server_host,
        server_port,
        options: opts,
    })
    .map_err(ClientError::Json)
}

struct WireGuardRuntime {
    local_addr: SocketAddr,
    child: Mutex<Child>,
    config_path: PathBuf,
}

impl WireGuardRuntime {
    fn spawn(server_host: &str, server_port: u16, opts: &WireGuardOptions) -> Result<Self> {
        let helper_binary = build_helper_binary()?;
        let local_addr = reserve_loopback_port()?;
        let config_path = write_runtime_config(local_addr, server_host, server_port, opts)?;
        let mut child = spawn_helper(&helper_binary, &config_path)?;
        wait_for_helper_start(local_addr, &mut child)?;
        Ok(Self {
            local_addr,
            child: Mutex::new(child),
            config_path,
        })
    }

    fn is_alive(&self) -> Result<bool> {
        let mut child = self
            .child
            .lock()
            .map_err(|_| ClientError::Config("wireguard helper lock is poisoned".into()))?;
        Ok(child.try_wait()?.is_none())
    }

    fn helper_client(&self) -> Result<WrongsvClient> {
        WrongsvClient::new(ServerConfig {
            host: self.local_addr.ip().to_string(),
            port: self.local_addr.port(),
            endpoint: Endpoint {
                proxy: ProxyProtocol::Mixed(MixedOptions::default()),
                transport: Transport::Raw,
                outer_security: OuterSecurity::None,
            },
        })
    }
}

impl Drop for WireGuardRuntime {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.stdin.take();
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        let _ = std::fs::remove_file(&self.config_path);
    }
}

#[derive(Serialize)]
struct HelperConfig<'a> {
    listen: String,
    server_endpoint: String,
    private_key: &'a str,
    peer_public_key: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pre_shared_key: Option<&'a str>,
    client_addresses: Vec<&'a str>,
    allowed_ips: &'a [String],
    mtu: u32,
    keep_alive: i64,
}

fn reserve_loopback_port() -> Result<SocketAddr> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    listener.local_addr().map_err(ClientError::Io)
}

fn write_runtime_config(
    local_addr: SocketAddr,
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!(
        "wrongcl-wireguard-{}.json",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let config = HelperConfig {
        listen: local_addr.to_string(),
        server_endpoint: format!("{server_host}:{server_port}"),
        private_key: &opts.private_key,
        peer_public_key: &opts.peer_public_key,
        pre_shared_key: opts.pre_shared_key.as_deref(),
        client_addresses: vec![opts.client_ip.as_str()],
        allowed_ips: &opts.allowed_ips,
        mtu: opts.mtu,
        keep_alive: 25,
    };
    let payload = serde_json::to_vec_pretty(&config)?;
    std::fs::write(&path, payload)?;
    Ok(path)
}

fn helper_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../helpers/wireguard-client-bridge")
}

fn helper_binary_path() -> PathBuf {
    let file = if cfg!(windows) {
        "wireguard-client-bridge.exe"
    } else {
        "wireguard-client-bridge"
    };
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join(file)
}

fn build_helper_binary() -> Result<PathBuf> {
    let helper_dir = helper_directory();
    let output = helper_binary_path();
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let status = Command::new("go")
        .env("GOTOOLCHAIN", "auto")
        .arg("build")
        .arg("-o")
        .arg(&output)
        .arg(".")
        .current_dir(&helper_dir)
        .status()?;
    if !status.success() {
        return Err(ClientError::Io(io::Error::other(format!(
            "go build failed for {}",
            helper_dir.display()
        ))));
    }
    Ok(output)
}

fn spawn_helper(binary: &Path, config_path: &Path) -> Result<Child> {
    Command::new(binary)
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(ClientError::Io)
}

fn wait_for_helper_start(local_addr: SocketAddr, child: &mut Child) -> Result<()> {
    let deadline = Instant::now() + HELPER_START_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(ClientError::Config(format!(
                "wireguard helper exited during startup: {status}"
            )));
        }
        if std::net::TcpStream::connect_timeout(&local_addr, Duration::from_millis(100)).is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(ClientError::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                "wireguard helper did not start listening in time",
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

struct HelperTunnel {
    inner: Box<dyn Tunnel>,
    runtime: Arc<WireGuardRuntime>,
}

struct HelperReader {
    inner: Box<dyn TunnelReader>,
    runtime: Arc<WireGuardRuntime>,
}

struct HelperWriter {
    inner: Box<dyn TunnelWriter>,
    runtime: Arc<WireGuardRuntime>,
}

impl Read for HelperTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let _runtime = &self.runtime;
        self.inner.read(buf)
    }
}

impl Write for HelperTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _runtime = &self.runtime;
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Tunnel for HelperTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(Self {
            inner: self.inner.try_clone_box()?,
            runtime: Arc::clone(&self.runtime),
        }))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let Self { inner, runtime } = *self;
        let (reader, writer) = inner.split_box()?;
        Ok((
            Box::new(HelperReader {
                inner: reader,
                runtime: Arc::clone(&runtime),
            }),
            Box::new(HelperWriter {
                inner: writer,
                runtime,
            }),
        ))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        self.inner.shutdown_write()
    }

    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        self.inner.set_socket_timeouts(read, write)
    }
}

impl Read for HelperReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let _runtime = &self.runtime;
        self.inner.read(buf)
    }
}

impl Write for HelperWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _runtime = &self.runtime;
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl TunnelWriter for HelperWriter {
    fn shutdown_write(&mut self) -> io::Result<()> {
        self.inner.shutdown_write()
    }
}

struct HelperUdpSession {
    inner: Box<dyn UdpSession>,
    runtime: Arc<WireGuardRuntime>,
}

impl UdpSession for HelperUdpSession {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        let _runtime = &self.runtime;
        self.inner.send_packet(payload)
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        let _runtime = &self.runtime;
        self.inner.try_recv_packet()
    }
}
