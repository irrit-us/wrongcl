use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bytes::Bytes;

use crate::client::{split_cloneable_tunnel, Tunnel, UdpPacket, UdpSession};
use crate::error::Result;
use crate::protocol::Target;
use crate::wireguard_runtime::{
    TcpSession, TcpSessionReader, TcpSessionWriter, UdpSession as RuntimeUdpSession,
    WireGuardRuntime,
};

pub(super) fn runtime_tunnel(
    session: TcpSession,
    runtime: Arc<WireGuardRuntime>,
) -> Result<Box<dyn Tunnel>> {
    let (writer, reader) = session.split();
    let (client, bridge) = local_tcp_pair()?;
    let bridge_reader = bridge.try_clone()?;
    let bridge_writer = bridge;

    spawn_local_to_wireguard(writer, bridge_reader);
    spawn_wireguard_to_local(reader, bridge_writer);

    Ok(Box::new(RuntimeTunnel {
        inner: client,
        runtime,
    }))
}

pub(super) fn runtime_udp_session(
    session: RuntimeUdpSession,
    runtime: Arc<WireGuardRuntime>,
    target: Target,
) -> Box<dyn UdpSession> {
    Box::new(RuntimeUdpAdapter {
        inner: session,
        runtime,
        target,
    })
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

struct RuntimeTunnel {
    inner: TcpStream,
    runtime: Arc<WireGuardRuntime>,
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

struct RuntimeUdpAdapter {
    inner: RuntimeUdpSession,
    runtime: Arc<WireGuardRuntime>,
    target: Target,
}

impl UdpSession for RuntimeUdpAdapter {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        let _runtime = &self.runtime;
        self.inner
            .send(Bytes::copy_from_slice(payload))
            .map_err(super::map_runtime_error)
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        let _runtime = &self.runtime;
        Ok(self
            .inner
            .try_recv()
            .map_err(super::map_runtime_error)?
            .map(|payload| UdpPacket {
                target: self.target.clone(),
                payload: payload.to_vec(),
            }))
    }
}
