use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;

use bytes::Bytes;
use quinn::{Connection as QuinnConnection, Endpoint};
use uuid::Uuid;

use crate::client::{Tunnel, TunnelReader, TunnelWriter, UdpPacket, UdpSession};
use crate::endpoint::{TlsOptions, TuicOptions};
use crate::error::{ClientError, Result};
use crate::protocol::Target;
use crate::tls;

const TUIC_VERSION: u8 = 0x05;
const TUIC_CMD_AUTHENTICATE: u8 = 0x00;
const TUIC_CMD_CONNECT: u8 = 0x01;
const TUIC_CMD_PACKET: u8 = 0x02;
const TUIC_ADDR_NONE: u8 = 0xff;
const TUIC_MAX_DATAGRAM_PAYLOAD: usize = 1200;

pub fn connect_tuic(
    server_host: &str,
    server_port: u16,
    opts: &TuicOptions,
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
                Ok((_endpoint, conn)) => match conn.open_bi().await {
                    Ok((mut send, mut recv)) => {
                        if let Err(err) =
                            write_tuic_connect_request(&mut send, &target_address).await
                        {
                            let _ = hs_tx.send(Err(err));
                            return;
                        }
                        let _ = hs_tx.send(Ok(()));

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
                        let _ =
                            hs_tx.send(Err(io::Error::other(format!("open TUIC stream: {err}"))));
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
        .map_err(|_| ClientError::Io(io::Error::other("TUIC thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(TuicTunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

pub fn connect_tuic_udp(
    server_host: &str,
    server_port: u16,
    opts: &TuicOptions,
    target: Target,
) -> Result<Box<dyn UdpSession>> {
    let target_address = target_authority(&target.host, target.port);
    let assoc_id = rand::random::<u16>();
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
                Ok((_endpoint, conn)) => {
                    let _ = hs_tx.send(Ok(()));

                    let read_conn = conn.clone();
                    let read_target = target_for_thread.clone();
                    let response_tx_read = response_tx.clone();
                    let read_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                        let mut assemblies: HashMap<(u16, u16), TuicPacketAssembly> =
                            HashMap::new();
                        loop {
                            match read_conn.read_datagram().await {
                                Ok(packet) => match parse_tuic_datagram_command(packet.as_ref()) {
                                    Ok(TuicCommand::Packet(packet))
                                        if packet.assoc_id == assoc_id =>
                                    {
                                        let key = (packet.assoc_id, packet.packet_id);
                                        let payload = if packet.frag_total <= 1 {
                                            Some(packet.payload)
                                        } else {
                                            let assembly =
                                                assemblies.entry(key).or_insert_with(|| {
                                                    TuicPacketAssembly::new(packet.frag_total)
                                                });
                                            if let Err(err) = assembly.insert(
                                                packet.fragment_index,
                                                packet.address.clone(),
                                                packet.payload,
                                            ) {
                                                let _ = response_tx_read.send(Err(
                                                    ClientError::Io(io::Error::new(
                                                        io::ErrorKind::InvalidData,
                                                        err,
                                                    )),
                                                ));
                                                break;
                                            }
                                            if assembly.is_complete() {
                                                match assembly.take_payload() {
                                                    Ok((_address, payload)) => {
                                                        assemblies.remove(&key);
                                                        Some(payload)
                                                    }
                                                    Err(err) => {
                                                        let _ = response_tx_read.send(Err(
                                                            ClientError::Io(io::Error::new(
                                                                io::ErrorKind::InvalidData,
                                                                err,
                                                            )),
                                                        ));
                                                        break;
                                                    }
                                                }
                                            } else {
                                                None
                                            }
                                        };

                                        if let Some(payload) = payload {
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
                                    }
                                    Ok(TuicCommand::Packet(_)) => {}
                                    Err(err) => {
                                        let _ = response_tx_read.send(Err(ClientError::Io(
                                            io::Error::new(io::ErrorKind::InvalidData, err),
                                        )));
                                        break;
                                    }
                                },
                                Err(err) => {
                                    let _ = response_tx_read.send(Err(ClientError::Io(
                                        io::Error::other(format!("TUIC UDP read: {err}")),
                                    )));
                                    break;
                                }
                            }
                        }
                    });

                    let mut packet_id: u16 = 0;
                    while let Some(payload) = tokio_write_rx.recv().await {
                        match fragment_tuic_payload(assoc_id, &target_address, &payload, packet_id)
                        {
                            Ok(packets) => {
                                for packet in packets {
                                    if conn.send_datagram(Bytes::from(packet)).is_err() {
                                        return;
                                    }
                                }
                                packet_id = packet_id.wrapping_add(1);
                            }
                            Err(err) => {
                                let _ = response_tx.send(Err(ClientError::Io(io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    err,
                                ))));
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
        .map_err(|_| ClientError::Io(io::Error::other("TUIC thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(TuicDatagramSession {
        write_tx,
        response_rx,
        _handle: handle,
    }))
}

struct TuicTunnel {
    read_rx: Receiver<Vec<u8>>,
    write_tx: SyncSender<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
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

struct TuicDatagramSession {
    write_tx: SyncSender<Vec<u8>>,
    response_rx: Receiver<std::result::Result<UdpPacket, ClientError>>,
    _handle: JoinHandle<()>,
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

async fn authenticated_connection(
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

async fn write_tuic_connect_request(send: &mut quinn::SendStream, target: &str) -> io::Result<()> {
    let packet = encode_tuic_connect(target)?;
    send.write_all(&packet).await.map_err(io::Error::other)
}

fn encode_tuic_connect(address: &str) -> io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(address.len() + 24);
    out.push(TUIC_VERSION);
    out.push(TUIC_CMD_CONNECT);
    encode_tuic_address(address, &mut out)?;
    Ok(out)
}

fn encode_tuic_address(address: &str, out: &mut Vec<u8>) -> io::Result<()> {
    let (host, port) = split_tuic_host_port(address)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    if let Ok(ip) = host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) => {
                out.push(0x01);
                out.extend_from_slice(&v4.octets());
                out.extend_from_slice(&port.to_be_bytes());
            }
            IpAddr::V6(v6) => {
                out.push(0x02);
                out.extend_from_slice(&v6.octets());
                out.extend_from_slice(&port.to_be_bytes());
            }
        }
    } else {
        out.push(0x00);
        out.push(host.len() as u8);
        out.extend_from_slice(host.as_bytes());
        out.extend_from_slice(&port.to_be_bytes());
    }
    Ok(())
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

fn split_tuic_host_port(s: &str) -> std::result::Result<(&str, u16), &'static str> {
    if let Some(rest) = s.strip_prefix('[') {
        let end = rest.find("]:").ok_or("invalid IPv6 host:port")?;
        let host = &rest[..end];
        let port = rest[end + 2..].parse::<u16>().map_err(|_| "invalid port")?;
        return Ok((host, port));
    }
    let (host, port) = s.rsplit_once(':').ok_or("missing port")?;
    Ok((host, port.parse::<u16>().map_err(|_| "invalid port")?))
}

fn fragment_tuic_payload(
    assoc_id: u16,
    address: &str,
    payload: &[u8],
    packet_id: u16,
) -> std::result::Result<Vec<Vec<u8>>, String> {
    if payload.len() <= TUIC_MAX_DATAGRAM_PAYLOAD {
        return Ok(vec![encode_tuic_packet(
            assoc_id, packet_id, 0, 1, address, payload,
        )?]);
    }
    let mut fragments = Vec::new();
    let chunk_count = payload
        .len()
        .div_ceil(TUIC_MAX_DATAGRAM_PAYLOAD)
        .min(u8::MAX as usize);
    let total = chunk_count as u8;
    for (idx, chunk) in payload.chunks(TUIC_MAX_DATAGRAM_PAYLOAD).enumerate() {
        let addr = if idx == 0 { address } else { "" };
        fragments.push(encode_tuic_packet(
            assoc_id, packet_id, idx as u8, total, addr, chunk,
        )?);
    }
    Ok(fragments)
}

fn encode_tuic_packet(
    assoc_id: u16,
    packet_id: u16,
    fragment_index: u8,
    fragment_total: u8,
    address: &str,
    payload: &[u8],
) -> std::result::Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(16 + address.len() + payload.len());
    out.push(TUIC_VERSION);
    out.push(TUIC_CMD_PACKET);
    out.extend_from_slice(&assoc_id.to_be_bytes());
    out.extend_from_slice(&packet_id.to_be_bytes());
    out.push(fragment_total);
    out.push(fragment_index);
    out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    if fragment_index == 0 {
        encode_tuic_address(address, &mut out).map_err(|err| err.to_string())?;
    } else {
        out.push(TUIC_ADDR_NONE);
    }
    out.extend_from_slice(payload);
    Ok(out)
}

fn parse_tuic_datagram_command(packet: &[u8]) -> std::result::Result<TuicCommand, String> {
    let (cmd, consumed) =
        try_parse_tuic_command(packet)?.ok_or("TUIC datagram too short to contain a command")?;
    if consumed != packet.len() {
        return Err(format!(
            "TUIC datagram contains {} trailing bytes",
            packet.len() - consumed
        ));
    }
    Ok(cmd)
}

fn try_parse_tuic_command(buf: &[u8]) -> std::result::Result<Option<(TuicCommand, usize)>, String> {
    let mut pos = 0usize;
    let Some(version) = buf.get(pos).copied() else {
        return Ok(None);
    };
    pos += 1;
    if version != TUIC_VERSION {
        return Err(format!("unexpected TUIC version: {version:#x}"));
    }
    let Some(cmd) = buf.get(pos).copied() else {
        return Ok(None);
    };
    pos += 1;
    match cmd {
        TUIC_CMD_PACKET => {
            if buf.len() < pos + 8 {
                return Ok(None);
            }
            let assoc_id = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
            pos += 2;
            let packet_id = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
            pos += 2;
            let frag_total = buf[pos];
            pos += 1;
            let fragment_index = buf[pos];
            pos += 1;
            let size = u16::from_be_bytes([buf[pos], buf[pos + 1]]) as usize;
            pos += 2;
            let address = parse_tuic_address(buf, &mut pos)?;
            if buf.len() < pos + size {
                return Ok(None);
            }
            let payload = buf[pos..pos + size].to_vec();
            pos += size;
            Ok(Some((
                TuicCommand::Packet(TuicPacket {
                    assoc_id,
                    packet_id,
                    frag_total,
                    fragment_index,
                    address,
                    payload,
                }),
                pos,
            )))
        }
        _ => Ok(None),
    }
}

enum TuicCommand {
    Packet(TuicPacket),
}

struct TuicPacket {
    assoc_id: u16,
    packet_id: u16,
    frag_total: u8,
    fragment_index: u8,
    address: Option<String>,
    payload: Vec<u8>,
}

struct TuicPacketAssembly {
    fragments: Vec<Option<Vec<u8>>>,
    address: Option<String>,
}

impl TuicPacketAssembly {
    fn new(fragment_total: u8) -> Self {
        Self {
            fragments: vec![None; fragment_total as usize],
            address: None,
        }
    }

    fn insert(
        &mut self,
        fragment_index: u8,
        address: Option<String>,
        payload: Vec<u8>,
    ) -> std::result::Result<(), String> {
        let idx = fragment_index as usize;
        if idx >= self.fragments.len() {
            return Err("invalid TUIC fragment index".into());
        }
        if self.address.is_none() && address.is_some() {
            self.address = address;
        }
        self.fragments[idx] = Some(payload);
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.fragments.iter().all(Option::is_some)
    }

    fn take_payload(&mut self) -> std::result::Result<(Option<String>, Vec<u8>), String> {
        let mut out = Vec::new();
        for fragment in self.fragments.iter_mut() {
            out.extend_from_slice(fragment.take().as_deref().ok_or("missing TUIC fragment")?);
        }
        Ok((self.address.take(), out))
    }
}

fn parse_tuic_address(buf: &[u8], pos: &mut usize) -> std::result::Result<Option<String>, String> {
    let Some(addr_type) = buf.get(*pos).copied() else {
        return Ok(None);
    };
    *pos += 1;
    match addr_type {
        TUIC_ADDR_NONE => Ok(None),
        0x00 => {
            let Some(len) = buf.get(*pos).copied() else {
                return Ok(None);
            };
            *pos += 1;
            let len = len as usize;
            if buf.len() < *pos + len + 2 {
                return Ok(None);
            }
            let host = std::str::from_utf8(&buf[*pos..*pos + len])
                .map_err(|err| err.to_string())?
                .to_string();
            *pos += len;
            let port = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
            *pos += 2;
            Ok(Some(format!("{host}:{port}")))
        }
        0x01 => {
            if buf.len() < *pos + 4 + 2 {
                return Ok(None);
            }
            let raw = [buf[*pos], buf[*pos + 1], buf[*pos + 2], buf[*pos + 3]];
            *pos += 4;
            let port = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
            *pos += 2;
            Ok(Some(format!("{}:{port}", Ipv4Addr::from(raw))))
        }
        0x02 => {
            if buf.len() < *pos + 16 + 2 {
                return Ok(None);
            }
            let mut raw = [0u8; 16];
            raw.copy_from_slice(&buf[*pos..*pos + 16]);
            *pos += 16;
            let port = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
            *pos += 2;
            Ok(Some(format!("[{}]:{port}", Ipv6Addr::from(raw))))
        }
        other => Err(format!("unexpected TUIC address type: {other:#x}")),
    }
}

fn target_authority(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
