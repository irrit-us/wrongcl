use std::io::{self, Read, Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;

use quinn::{Connection as QuinnConnection, Endpoint};
use uuid::Uuid;

use crate::client::{Tunnel, TunnelReader, TunnelWriter, UdpPacket, UdpSession};
use crate::endpoint::{TlsOptions, TuicOptions};
use crate::error::{ClientError, Result};
use crate::tls;

use super::codec::encode_connect_request;

const TUIC_VERSION: u8 = 0x05;
const TUIC_CMD_AUTHENTICATE: u8 = 0x00;

pub(super) struct TuicTunnel {
    pub(super) read_rx: Receiver<Vec<u8>>,
    pub(super) write_tx: SyncSender<Vec<u8>>,
    pub(super) read_buf: Vec<u8>,
    pub(super) eof: bool,
    pub(super) _handle: JoinHandle<()>,
}

struct TuicReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

#[derive(Clone)]
struct TuicWriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for TuicTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for TuicTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "TUIC write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for TuicReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for TuicWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "TUIC write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for TuicWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for TuicTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "TUIC tunnel cannot be cloned (single QUIC stream)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let TuicTunnel {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(TuicReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(TuicWriteHalf { write_tx }),
        ))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn set_socket_timeouts(
        &self,
        _read: Option<std::time::Duration>,
        _write: Option<std::time::Duration>,
    ) -> io::Result<()> {
        Ok(())
    }
}

fn read_channel(
    read_rx: &Receiver<Vec<u8>>,
    read_buf: &mut Vec<u8>,
    eof: &mut bool,
    buf: &mut [u8],
) -> io::Result<usize> {
    if !read_buf.is_empty() {
        let n = read_buf.len().min(buf.len());
        buf[..n].copy_from_slice(&read_buf[..n]);
        read_buf.drain(..n);
        return Ok(n);
    }
    if *eof {
        return Ok(0);
    }
    let data = match read_rx.recv() {
        Ok(d) => d,
        Err(_) => {
            *eof = true;
            return Ok(0);
        }
    };
    if data.is_empty() {
        *eof = true;
        return Ok(0);
    }
    let n = data.len().min(buf.len());
    buf[..n].copy_from_slice(&data[..n]);
    if n < data.len() {
        read_buf.extend_from_slice(&data[n..]);
    }
    Ok(n)
}

pub(super) struct TuicDatagramSession {
    pub(super) write_tx: SyncSender<Vec<u8>>,
    pub(super) response_rx: Receiver<std::result::Result<UdpPacket, ClientError>>,
    pub(super) _handle: JoinHandle<()>,
}

impl UdpSession for TuicDatagramSession {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        self.write_tx
            .send(payload.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "TUIC UDP write channel closed"))
            .map_err(ClientError::Io)?;
        Ok(())
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        match self.response_rx.try_recv() {
            Ok(result) => result.map(Some),
            Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => Ok(None),
        }
    }
}

pub(super) async fn authenticated_connection(
    server_host: &str,
    server_port: u16,
    opts: &TuicOptions,
) -> io::Result<(Endpoint, QuinnConnection)> {
    let server_addr = resolve_server_addr(server_host, server_port)?;
    let tls_opts = TlsOptions {
        server_name: opts.server_name.clone(),
        insecure_skip_verify: true,
        alpn: vec!["h3".into()],
    };
    let client_crypto =
        tls::build_client_config(&tls_opts).map_err(|err| io::Error::other(err.to_string()))?;
    let client_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
        .map_err(io::Error::other)?;
    let client_config = quinn::ClientConfig::new(Arc::new(client_crypto));

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
        .map_err(|err| io::Error::other(format!("tuic endpoint: {err}")))?;
    endpoint.set_default_client_config(client_config);
    let conn = endpoint
        .connect(server_addr, &opts.server_name)
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("tuic connect: {err}"),
            )
        })?
        .await
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("tuic connect: {err}"),
            )
        })?;

    let uuid = Uuid::parse_str(opts.uuid.trim())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err.to_string()))?;
    let token = derive_tuic_token(&conn, &uuid, &opts.password)
        .map_err(|err| io::Error::new(io::ErrorKind::PermissionDenied, err))?;
    let mut auth = Vec::with_capacity(50);
    auth.push(TUIC_VERSION);
    auth.push(TUIC_CMD_AUTHENTICATE);
    auth.extend_from_slice(uuid.as_bytes());
    auth.extend_from_slice(&token);
    let mut send = conn.open_uni().await.map_err(io::Error::other)?;
    send.write_all(&auth).await.map_err(io::Error::other)?;
    send.finish().map_err(io::Error::other)?;

    Ok((endpoint, conn))
}

fn derive_tuic_token(
    conn: &QuinnConnection,
    uuid: &Uuid,
    password: &str,
) -> std::result::Result<[u8; 32], String> {
    let mut token = [0u8; 32];
    conn.export_keying_material(&mut token, uuid.as_bytes(), password.as_bytes())
        .map_err(|e| format!("tuic token derivation failed: {e:?}"))?;
    Ok(token)
}

pub(super) async fn write_tuic_connect_request(
    send: &mut quinn::SendStream,
    target: &str,
) -> io::Result<()> {
    let packet = encode_connect_request(target)?;
    send.write_all(&packet).await.map_err(io::Error::other)
}

fn resolve_server_addr(host: &str, port: u16) -> io::Result<SocketAddr> {
    ToSocketAddrs::to_socket_addrs(&(host, port))?
        .next()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "no server addresses resolved",
            )
        })
}
