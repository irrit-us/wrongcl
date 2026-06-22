use std::io::{self, Read, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;

use bytes::Bytes;
use h3_quinn::Connection as H3QuinnConnection;
use quinn::{Connection as QuinnConnection, Endpoint};

use crate::client::{Tunnel, TunnelReader, TunnelWriter, UdpPacket, UdpSession};
use crate::endpoint::{Hysteria2Options, TlsOptions};
use crate::error::{ClientError, Result};
use crate::quic_obfs::{
    wrap_async_udp_socket_gecko, wrap_async_udp_socket_salamander, GECKO_DEFAULT_MAX_PACKET_SIZE,
    GECKO_DEFAULT_MIN_PACKET_SIZE,
};
use crate::tls;

const HYSTERIA2_AUTH_PATH: &str = "/auth";
const HYSTERIA2_TCP_REQUEST_ID: u64 = 0x401;

pub(super) struct Hysteria2Tunnel {
    pub(super) read_rx: Receiver<Vec<u8>>,
    pub(super) write_tx: SyncSender<Vec<u8>>,
    pub(super) read_buf: Vec<u8>,
    pub(super) eof: bool,
    pub(super) _handle: JoinHandle<()>,
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

pub(super) struct Hysteria2DatagramSession {
    pub(super) write_tx: SyncSender<Vec<u8>>,
    pub(super) response_rx: Receiver<std::result::Result<UdpPacket, ClientError>>,
    pub(super) _handle: JoinHandle<()>,
}

pub(super) struct Hysteria2PacketAssembly {
    fragments: Vec<Option<Vec<u8>>>,
    address: Option<String>,
}

impl Hysteria2PacketAssembly {
    pub(super) fn new(fragment_count: u8) -> io::Result<Self> {
        if fragment_count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Hysteria2 UDP fragment count must be at least 1",
            ));
        }
        Ok(Self {
            fragments: vec![None; fragment_count as usize],
            address: None,
        })
    }

    pub(super) fn insert(
        &mut self,
        fragment_id: u8,
        address: String,
        payload: Vec<u8>,
    ) -> io::Result<()> {
        let idx = fragment_id as usize;
        if idx >= self.fragments.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Hysteria2 UDP fragment index out of range",
            ));
        }
        if let Some(existing) = &self.address {
            if existing != &address {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Hysteria2 UDP fragments disagreed on target address",
                ));
            }
        } else {
            self.address = Some(address);
        }
        self.fragments[idx] = Some(payload);
        Ok(())
    }

    pub(super) fn is_complete(&self) -> bool {
        self.fragments.iter().all(Option::is_some)
    }

    pub(super) fn take_payload(&mut self) -> io::Result<(String, Vec<u8>)> {
        let address = self.address.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Hysteria2 UDP packet assembly missing target address",
            )
        })?;
        let mut payload = Vec::new();
        for fragment in self.fragments.iter_mut() {
            payload.extend_from_slice(fragment.take().as_deref().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Hysteria2 UDP packet assembly missing a fragment",
                )
            })?);
        }
        Ok((address, payload))
    }
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

pub(super) async fn authenticated_connection(
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

    let runtime: Arc<dyn quinn::Runtime> = Arc::new(quinn::TokioRuntime);
    let bind_addr = if server_addr.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let socket = UdpSocket::bind(bind_addr)?;
    let abstract_socket = runtime.wrap_udp_socket(socket)?;
    let abstract_socket = match (opts.obfs_type.as_deref(), opts.obfs_password.as_deref()) {
        (Some("salamander"), Some(password)) => {
            wrap_async_udp_socket_salamander(abstract_socket, password.as_bytes())
                .map_err(io::Error::other)?
        }
        (Some("gecko"), Some(password)) => wrap_async_udp_socket_gecko(
            abstract_socket,
            password.as_bytes(),
            opts.obfs_min_packet_size
                .unwrap_or(GECKO_DEFAULT_MIN_PACKET_SIZE),
            opts.obfs_max_packet_size
                .unwrap_or(GECKO_DEFAULT_MAX_PACKET_SIZE),
        )
        .map_err(io::Error::other)?,
        _ => abstract_socket,
    };
    let mut endpoint = Endpoint::new_with_abstract_socket(
        quinn::EndpointConfig::default(),
        None,
        abstract_socket,
        runtime,
    )
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

pub(super) async fn write_hysteria2_tcp_request(
    send: &mut quinn::SendStream,
    target: &str,
) -> io::Result<()> {
    let mut req = Vec::with_capacity(target.len() + 24);
    req.extend_from_slice(&encode_quic_varint(HYSTERIA2_TCP_REQUEST_ID)?);
    req.extend_from_slice(&encode_quic_varint(target.len() as u64)?);
    req.extend_from_slice(target.as_bytes());
    req.extend_from_slice(&encode_quic_varint(0)?);
    send.write_all(&req).await.map_err(io::Error::other)
}

pub(super) async fn read_hysteria2_tcp_response(
    recv: &mut quinn::RecvStream,
) -> io::Result<(bool, String)> {
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

pub(super) fn encode_hysteria2_udp_message(
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

pub(super) fn parse_hysteria2_udp_message(
    packet: &[u8],
) -> io::Result<(u32, u16, u8, u8, String, Vec<u8>)> {
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

pub(super) fn target_authority(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::Hysteria2PacketAssembly;

    #[test]
    fn hysteria2_packet_assembly_reassembles_out_of_order_fragments() {
        let mut assembly = Hysteria2PacketAssembly::new(3).unwrap();
        assembly
            .insert(2, "example.com:53".into(), b"third".to_vec())
            .unwrap();
        assembly
            .insert(0, "example.com:53".into(), b"first".to_vec())
            .unwrap();
        assert!(!assembly.is_complete());
        assembly
            .insert(1, "example.com:53".into(), b"second".to_vec())
            .unwrap();
        assert!(assembly.is_complete());

        let (address, payload) = assembly.take_payload().unwrap();
        assert_eq!(address, "example.com:53");
        assert_eq!(payload, b"firstsecondthird");
    }

    #[test]
    fn hysteria2_packet_assembly_rejects_invalid_fragment_index() {
        let mut assembly = Hysteria2PacketAssembly::new(2).unwrap();
        let err = assembly
            .insert(2, "example.com:53".into(), b"payload".to_vec())
            .unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn hysteria2_packet_assembly_rejects_mismatched_addresses() {
        let mut assembly = Hysteria2PacketAssembly::new(2).unwrap();
        assembly
            .insert(0, "example.com:53".into(), b"first".to_vec())
            .unwrap();
        let err = assembly
            .insert(1, "other.example:53".into(), b"second".to_vec())
            .unwrap_err();
        assert!(err.to_string().contains("disagreed on target address"));
    }
}
