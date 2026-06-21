use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use bytes::Bytes;
use serde::Serialize;

use crate::client::{split_cloneable_tunnel, Tunnel, UdpPacket, UdpSession};
use crate::endpoint::WireGuardOptions;
use crate::error::{ClientError, Result};
use crate::protocol::Target;
use crate::wireguard_runtime::{
    TcpSessionReader, TcpSessionWriter, WireGuardRuntime as DirectWireGuardRuntime,
    WireGuardRuntimeConfig,
};

static RUNTIME_CACHE: OnceLock<Mutex<HashMap<String, Arc<DirectWireGuardRuntime>>>> =
    OnceLock::new();

pub fn connect_wireguard(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
    target: &Target,
) -> Result<Box<dyn Tunnel>> {
    let runtime = acquire_runtime(server_host, server_port, opts)?;
    let route = runtime.resolve_target(target).map_err(map_runtime_error)?;
    let session = runtime.open_tcp(route).map_err(map_runtime_error)?;
    let stream = bridge_tcp_session(session)?;
    Ok(Box::new(RuntimeTunnel {
        inner: stream,
        runtime,
    }))
}

pub fn connect_wireguard_udp(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
    target: &Target,
) -> Result<Box<dyn UdpSession>> {
    let runtime = acquire_runtime(server_host, server_port, opts)?;
    let route = runtime.resolve_target(target).map_err(map_runtime_error)?;
    let session = runtime.open_udp(route).map_err(map_runtime_error)?;
    Ok(Box::new(RuntimeUdpSession {
        inner: session,
        runtime,
        target: target.clone(),
    }))
}

fn acquire_runtime(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
) -> Result<Arc<DirectWireGuardRuntime>> {
    let key = runtime_key(server_host, server_port, opts)?;
    let cache = RUNTIME_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache
        .lock()
        .map_err(|_| ClientError::Config("wireguard runtime cache lock is poisoned".into()))?;

    if let Some(runtime) = guard.get(&key) {
        return Ok(Arc::clone(runtime));
    }

    let config = WireGuardRuntimeConfig::from_options(server_host, server_port, opts)?;
    let runtime = Arc::new(DirectWireGuardRuntime::start(config).map_err(map_runtime_error)?);
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

fn bridge_tcp_session(session: crate::wireguard_runtime::TcpSession) -> Result<TcpStream> {
    let (writer, reader) = session.split();
    let (client, bridge) = local_tcp_pair()?;
    let bridge_reader = bridge.try_clone()?;
    let bridge_writer = bridge;

    spawn_local_to_wireguard(writer, bridge_reader);
    spawn_wireguard_to_local(reader, bridge_writer);

    Ok(client)
}

fn spawn_local_to_wireguard(writer: TcpSessionWriter, mut local: TcpStream) {
    thread::spawn(move || {
        let mut buf = [0u8; 64 * 1024];
        loop {
            match local.read(&mut buf) {
                Ok(0) => {
                    writer.shutdown();
                    return;
                }
                Ok(size) => {
                    if writer.send(Bytes::copy_from_slice(&buf[..size])).is_err() {
                        return;
                    }
                }
                Err(_) => {
                    writer.shutdown();
                    return;
                }
            }
        }
    });
}

fn spawn_wireguard_to_local(mut reader: TcpSessionReader, mut local: TcpStream) {
    thread::spawn(move || {
        while let Some(chunk) = reader.blocking_recv() {
            if local.write_all(&chunk).is_err() {
                return;
            }
        }
        let _ = local.shutdown(Shutdown::Write);
    });
}

fn local_tcp_pair() -> io::Result<(TcpStream, TcpStream)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let addr = listener.local_addr()?;
    let client = TcpStream::connect(addr)?;
    let (server, _) = listener.accept()?;
    Ok((client, server))
}

fn map_runtime_error(error: anyhow::Error) -> ClientError {
    if let Some(io_error) = error.downcast_ref::<io::Error>() {
        return ClientError::Io(io::Error::new(io_error.kind(), io_error.to_string()));
    }
    ClientError::Config(error.to_string())
}

struct RuntimeTunnel {
    inner: TcpStream,
    runtime: Arc<DirectWireGuardRuntime>,
}

impl Read for RuntimeTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let _runtime = &self.runtime;
        self.inner.read(buf)
    }
}

impl Write for RuntimeTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _runtime = &self.runtime;
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Tunnel for RuntimeTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(Self {
            inner: self.inner.try_clone()?,
            runtime: Arc::clone(&self.runtime),
        }))
    }

    fn split_box(
        self: Box<Self>,
    ) -> io::Result<(
        Box<dyn crate::client::TunnelReader>,
        Box<dyn crate::client::TunnelWriter>,
    )> {
        split_cloneable_tunnel(self)
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        self.inner.shutdown(Shutdown::Write)
    }

    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        self.inner.set_read_timeout(read)?;
        self.inner.set_write_timeout(write)?;
        Ok(())
    }
}

struct RuntimeUdpSession {
    inner: crate::wireguard_runtime::UdpSession,
    runtime: Arc<DirectWireGuardRuntime>,
    target: Target,
}

impl UdpSession for RuntimeUdpSession {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        let _runtime = &self.runtime;
        self.inner
            .send(Bytes::copy_from_slice(payload))
            .map_err(map_runtime_error)
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        let _runtime = &self.runtime;
        Ok(self
            .inner
            .try_recv()
            .map_err(map_runtime_error)?
            .map(|payload| UdpPacket {
                target: self.target.clone(),
                payload: payload.to_vec(),
            }))
    }
}
