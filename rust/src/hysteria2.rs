use std::io::{self, Read, Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;

use bytes::Bytes;
use h3_quinn::Connection as H3QuinnConnection;
use quinn::{Connection as QuinnConnection, Endpoint};

use crate::client::{Tunnel, TunnelReader, TunnelWriter, UdpPacket, UdpSession};
use crate::endpoint::{Hysteria2Options, TlsOptions};
use crate::error::{ClientError, Result};
use crate::protocol::Target;
use crate::tls;

const HYSTERIA2_AUTH_PATH: &str = "/auth";
const HYSTERIA2_TCP_REQUEST_ID: u64 = 0x401;

pub fn connect_hysteria2(
    server_host: &str,
    server_port: u16,
    opts: &Hysteria2Options,
    target: Target,
) -> Result<Box<dyn Tunnel>> {
    let target_address = target_authority(&target.host, target.port);
    let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>();
    let (write_tx, write_rx) = mpsc::sync_channel::<Vec<u8>>(32);
    let (hs_tx, hs_rx) = mpsc::sync_channel::<std::result::Result<(), io::Error>>(1);
    let (tokio_write_tx, mut tokio_write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
    let server_host = server_host.to_string();
    let opts = opts.clone();

    let handle = std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                let _ = hs_tx.send(Err(io::Error::other(format!("tokio runtime: {err}"))));
                return;
            }
        };

        let bridge_tx = tokio_write_tx;
        std::thread::spawn(move || {
            while let Ok(data) = write_rx.recv() {
                if bridge_tx.blocking_send(data).is_err() {
                    break;
                }
            }
        });

        rt.block_on(async move {
            match authenticated_connection(&server_host, server_port, &opts).await {
                Ok((_endpoint, conn, _udp_enabled)) => match conn.open_bi().await {
                    Ok((mut send, mut recv)) => {
                        if let Err(err) =
                            write_hysteria2_tcp_request(&mut send, &target_address).await
                        {
                            let _ = hs_tx.send(Err(err));
                            return;
                        }
                        match read_hysteria2_tcp_response(&mut recv).await {
                            Ok((true, _message)) => {
                                let _ = hs_tx.send(Ok(()));
                            }
                            Ok((false, message)) => {
                                let _ = hs_tx.send(Err(io::Error::new(
                                    io::ErrorKind::PermissionDenied,
                                    format!("Hysteria2 TCP request failed: {message}"),
                                )));
                                return;
                            }
                            Err(err) => {
                                let _ = hs_tx.send(Err(err));
                                return;
                            }
                        }

                        let read_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                            let mut buf = vec![0u8; 65536];
                            loop {
                                match recv.read(&mut buf).await {
                                    Ok(Some(n)) => {
                                        if n == 0 {
                                            let _ = read_tx.send(Vec::new());
                                            break;
                                        }
                                        if read_tx.send(buf[..n].to_vec()).is_err() {
                                            break;
                                        }
                                    }
                                    Ok(None) => {
                                        let _ = read_tx.send(Vec::new());
                                        break;
                                    }
                                    Err(_) => {
                                        let _ = read_tx.send(Vec::new());
                                        break;
                                    }
                                }
                            }
                        });

                        while let Some(data) = tokio_write_rx.recv().await {
                            if send
                                .write_all(&data)
                                .await
                                .map_err(io::Error::other)
                                .is_err()
                            {
                                break;
                            }
                        }

                        let _ = send.finish();
                        read_task.abort();
                    }
                    Err(err) => {
                        let _ = hs_tx.send(Err(io::Error::other(format!(
                            "open Hysteria2 stream: {err}"
                        ))));
                    }
                },
                Err(err) => {
                    let _ = hs_tx.send(Err(err));
                }
            }
        });
    });

    hs_rx
        .recv()
        .map_err(|_| ClientError::Io(io::Error::other("Hysteria2 thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(Hysteria2Tunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

pub fn connect_hysteria2_udp(
    server_host: &str,
    server_port: u16,
    opts: &Hysteria2Options,
    target: Target,
) -> Result<Box<dyn UdpSession>> {
    let target_address = target_authority(&target.host, target.port);
    let session_id = rand::random::<u32>();
    let (response_tx, response_rx) = mpsc::channel::<std::result::Result<UdpPacket, ClientError>>();
    let (write_tx, write_rx) = mpsc::sync_channel::<Vec<u8>>(64);
    let (hs_tx, hs_rx) = mpsc::sync_channel::<std::result::Result<(), io::Error>>(1);
    let (tokio_write_tx, mut tokio_write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
    let server_host = server_host.to_string();
    let opts = opts.clone();
    let target_for_thread = target.clone();

    let handle = std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                let _ = hs_tx.send(Err(io::Error::other(format!("tokio runtime: {err}"))));
                return;
            }
        };

        let bridge_tx = tokio_write_tx;
        std::thread::spawn(move || {
            while let Ok(data) = write_rx.recv() {
                if bridge_tx.blocking_send(data).is_err() {
                    break;
                }
            }
        });

        rt.block_on(async move {
            match authenticated_connection(&server_host, server_port, &opts).await {
                Ok((_endpoint, conn, udp_enabled)) => {
                    if !udp_enabled {
                        let _ = hs_tx.send(Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "Hysteria2 server disabled UDP relay",
                        )));
                        return;
                    }
                    let _ = hs_tx.send(Ok(()));

                    let read_conn = conn.clone();
                    let read_target = target_for_thread.clone();
                    let response_tx_read = response_tx.clone();
                    let read_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                        loop {
                            match read_conn.read_datagram().await {
                                Ok(packet) => match parse_hysteria2_udp_message(packet.as_ref()) {
                                    Ok((incoming_session_id, _packet_id, _fragment_id, fragment_count, _address, payload))
                                        if incoming_session_id == session_id && fragment_count == 1 =>
                                    {
                                        if response_tx_read
                                            .send(Ok(UdpPacket {
                                                target: read_target.clone(),
                                                payload,
                                            }))
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                    Ok((_incoming_session_id, _packet_id, _fragment_id, fragment_count, _address, _payload))
                                        if fragment_count != 1 =>
                                    {
                                        let _ = response_tx_read.send(Err(ClientError::UnsupportedProtocol(
                                            "Hysteria2 fragmented UDP responses are not implemented in wrongcl yet"
                                                .into(),
                                        )));
                                        break;
                                    }
                                    Ok(_) => {}
                                    Err(err) => {
                                        let _ = response_tx_read.send(Err(ClientError::Io(err)));
                                        break;
                                    }
                                },
                                Err(err) => {
                                    let _ = response_tx_read.send(Err(ClientError::Io(io::Error::other(
                                        format!("Hysteria2 UDP read: {err}"),
                                    ))));
                                    break;
                                }
                            }
                        }
                    });

                    let mut packet_id: u16 = 0;
                    while let Some(payload) = tokio_write_rx.recv().await {
                        match encode_hysteria2_udp_message(
                            session_id,
                            packet_id,
                            0,
                            1,
                            &target_address,
                            &payload,
                        ) {
                            Ok(packet) => {
                                if conn.send_datagram(Bytes::from(packet)).is_err() {
                                    break;
                                }
                                packet_id = packet_id.wrapping_add(1);
                            }
                            Err(err) => {
                                let _ = response_tx.send(Err(ClientError::Io(err)));
                                break;
                            }
                        }
                    }

                    read_task.abort();
                }
                Err(err) => {
                    let _ = hs_tx.send(Err(err));
                }
            }
        });
    });

    hs_rx
        .recv()
        .map_err(|_| ClientError::Io(io::Error::other("Hysteria2 thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(Hysteria2DatagramSession {
        write_tx,
        response_rx,
        _handle: handle,
    }))
}

struct Hysteria2Tunnel {
    read_rx: Receiver<Vec<u8>>,
    write_tx: SyncSender<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct Hysteria2ReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

#[derive(Clone)]
struct Hysteria2WriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for Hysteria2Tunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for Hysteria2Tunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx.send(buf.to_vec()).map_err(|_| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Hysteria2 write channel closed")
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Hysteria2ReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for Hysteria2WriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx.send(buf.to_vec()).map_err(|_| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Hysteria2 write channel closed")
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for Hysteria2WriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for Hysteria2Tunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Hysteria2 tunnel cannot be cloned (single QUIC stream)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let Hysteria2Tunnel {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(Hysteria2ReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(Hysteria2WriteHalf { write_tx }),
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

struct Hysteria2DatagramSession {
    write_tx: SyncSender<Vec<u8>>,
    response_rx: Receiver<std::result::Result<UdpPacket, ClientError>>,
    _handle: JoinHandle<()>,
}

impl UdpSession for Hysteria2DatagramSession {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        self.write_tx.send(payload.to_vec()).map_err(|_| {
            ClientError::Io(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Hysteria2 UDP write channel closed",
            ))
        })?;
        Ok(())
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        match self.response_rx.try_recv() {
            Ok(result) => result.map(Some),
            Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => Ok(None),
        }
    }
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

async fn authenticated_connection(
    server_host: &str,
    server_port: u16,
    opts: &Hysteria2Options,
) -> io::Result<(Endpoint, QuinnConnection, bool)> {
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
        .map_err(|err| io::Error::other(format!("hysteria2 endpoint: {err}")))?;
    endpoint.set_default_client_config(client_config);
    let conn = endpoint
        .connect(server_addr, &opts.server_name)
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("hysteria2 connect: {err}"),
            )
        })?
        .await
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("hysteria2 connect: {err}"),
            )
        })?;

    let h3_conn = H3QuinnConnection::new(conn.clone());
    let mut builder = h3::client::builder();
    builder.enable_datagram(true);
    let (_driver, mut send_request) = builder
        .build::<_, _, Bytes>(h3_conn)
        .await
        .map_err(io::Error::other)?;
    let request = http::Request::builder()
        .method(http::Method::POST)
        .uri(format!("https://{}{HYSTERIA2_AUTH_PATH}", opts.server_name))
        .header("Hysteria-Auth", &opts.password)
        .header("Hysteria-CC-RX", "1000")
        .body(())
        .map_err(io::Error::other)?;
    let mut req_stream = send_request
        .send_request(request)
        .await
        .map_err(io::Error::other)?;
    req_stream.finish().await.map_err(io::Error::other)?;
    let response = req_stream.recv_response().await.map_err(io::Error::other)?;
    if response.status() != 233 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Hysteria2 auth failed with status {}", response.status()),
        ));
    }
    let udp_enabled = response
        .headers()
        .get("Hysteria-UDP")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("false")
        == "true";
    Ok((endpoint, conn, udp_enabled))
}

async fn write_hysteria2_tcp_request(send: &mut quinn::SendStream, target: &str) -> io::Result<()> {
    let mut req = Vec::with_capacity(target.len() + 24);
    req.extend_from_slice(&encode_quic_varint(HYSTERIA2_TCP_REQUEST_ID)?);
    req.extend_from_slice(&encode_quic_varint(target.len() as u64)?);
    req.extend_from_slice(target.as_bytes());
    req.extend_from_slice(&encode_quic_varint(0)?);
    send.write_all(&req).await.map_err(io::Error::other)
}

async fn read_hysteria2_tcp_response(recv: &mut quinn::RecvStream) -> io::Result<(bool, String)> {
    let mut status = [0u8; 1];
    recv.read_exact(&mut status)
        .await
        .map_err(io::Error::other)?;
    let msg_len = read_quic_varint_from_stream(recv).await? as usize;
    let mut msg = vec![0u8; msg_len];
    if msg_len > 0 {
        recv.read_exact(&mut msg).await.map_err(io::Error::other)?;
    }
    let pad_len = read_quic_varint_from_stream(recv).await? as usize;
    if pad_len > 0 {
        let mut padding = vec![0u8; pad_len];
        recv.read_exact(&mut padding)
            .await
            .map_err(io::Error::other)?;
    }
    Ok((status[0] == 0, String::from_utf8_lossy(&msg).to_string()))
}

async fn read_quic_varint_from_stream(recv: &mut quinn::RecvStream) -> io::Result<u64> {
    let mut first = [0u8; 1];
    recv.read_exact(&mut first)
        .await
        .map_err(io::Error::other)?;
    let len = 1usize << (first[0] >> 6);
    let mut buf = vec![0u8; len];
    buf[0] = first[0];
    if len > 1 {
        recv.read_exact(&mut buf[1..])
            .await
            .map_err(io::Error::other)?;
    }
    let mut pos = 0usize;
    read_quic_varint(&buf, &mut pos)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "missing varint"))
}

fn read_quic_varint(buf: &[u8], pos: &mut usize) -> io::Result<Option<u64>> {
    if *pos >= buf.len() {
        return Ok(None);
    }
    let first = buf[*pos];
    let len = 1usize << (first >> 6);
    if buf.len() < *pos + len {
        return Ok(None);
    }
    let value = match len {
        1 => (first & 0x3f) as u64,
        2 => u16::from_be_bytes([buf[*pos], buf[*pos + 1]]) as u64 & 0x3fff,
        4 => {
            u32::from_be_bytes([buf[*pos], buf[*pos + 1], buf[*pos + 2], buf[*pos + 3]]) as u64
                & 0x3fff_ffff
        }
        8 => {
            u64::from_be_bytes([
                buf[*pos],
                buf[*pos + 1],
                buf[*pos + 2],
                buf[*pos + 3],
                buf[*pos + 4],
                buf[*pos + 5],
                buf[*pos + 6],
                buf[*pos + 7],
            ]) & 0x3fff_ffff_ffff_ffff
        }
        _ => unreachable!(),
    };
    *pos += len;
    Ok(Some(value))
}

fn encode_quic_varint(value: u64) -> io::Result<Vec<u8>> {
    if value < (1 << 6) {
        Ok(vec![value as u8])
    } else if value < (1 << 14) {
        Ok((0x4000 | value as u16).to_be_bytes().to_vec())
    } else if value < (1 << 30) {
        Ok((0x8000_0000 | value as u32).to_be_bytes().to_vec())
    } else if value < (1 << 62) {
        Ok((0xC000_0000_0000_0000 | value).to_be_bytes().to_vec())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "quic varint too large",
        ))
    }
}

fn encode_hysteria2_udp_message(
    session_id: u32,
    packet_id: u16,
    fragment_id: u8,
    fragment_count: u8,
    address: &str,
    payload: &[u8],
) -> io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(8 + address.len() + payload.len() + 16);
    out.extend_from_slice(&session_id.to_be_bytes());
    out.extend_from_slice(&packet_id.to_be_bytes());
    out.push(fragment_id);
    out.push(fragment_count);
    out.extend_from_slice(&encode_quic_varint(address.len() as u64)?);
    out.extend_from_slice(address.as_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

fn parse_hysteria2_udp_message(packet: &[u8]) -> io::Result<(u32, u16, u8, u8, String, Vec<u8>)> {
    if packet.len() < 8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Hysteria2 UDP packet too short",
        ));
    }
    let session_id = u32::from_be_bytes([packet[0], packet[1], packet[2], packet[3]]);
    let packet_id = u16::from_be_bytes([packet[4], packet[5]]);
    let fragment_id = packet[6];
    let fragment_count = packet[7];
    let mut pos = 8usize;
    let Some(addr_len) = read_quic_varint(packet, &mut pos)? else {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Hysteria2 UDP packet missing address length",
        ));
    };
    let addr_len = addr_len as usize;
    if packet.len() < pos + addr_len {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Hysteria2 UDP packet truncated address",
        ));
    }
    let address = std::str::from_utf8(&packet[pos..pos + addr_len])
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?
        .to_string();
    pos += addr_len;
    Ok((
        session_id,
        packet_id,
        fragment_id,
        fragment_count,
        address,
        packet[pos..].to_vec(),
    ))
}

fn target_authority(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
