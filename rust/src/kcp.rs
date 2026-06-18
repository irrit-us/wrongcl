use std::io::{self, Read, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use aes_gcm::aead::{AeadInPlace, KeyInit};
use rand::RngCore;
use sha2::Digest;

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::endpoint::KcpOptions;
use crate::error::{ClientError, Result};
use crate::protocol::{encode_raw_vless_header, encode_udp_vless_header, Target};

#[path = "../../../wrongsv/crates/server/src/handler/kcp/xray_session.rs"]
mod xray_session;

use xray_session::{
    SessionConfig as XraySessionConfig, SessionState as XraySessionState, XrayKcpSession,
};

const MKCP_ORIGINAL_OVERHEAD: usize = 6;

pub fn connect_kcp(
    server_host: &str,
    server_port: u16,
    opts: &KcpOptions,
    uuid: &str,
    target: &Target,
    flow: &str,
    udp: bool,
) -> Result<Box<dyn Tunnel>> {
    let header = if udp {
        encode_udp_vless_header(uuid, target, flow)?
    } else {
        encode_raw_vless_header(uuid, target, flow)?
    };
    let server_addr = resolve_server_addr(server_host, server_port)?;
    let packet_mask = KcpPacketMask::from_seed(&opts.seed);

    let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>();
    let (write_tx, write_rx) = mpsc::sync_channel::<Vec<u8>>(32);
    let (hs_tx, hs_rx) = mpsc::sync_channel::<std::result::Result<Vec<u8>, io::Error>>(1);
    let opts = opts.clone();

    let handle = thread::spawn(move || {
        let udp = match UdpSocket::bind("0.0.0.0:0") {
            Ok(socket) => socket,
            Err(err) => {
                let _ = hs_tx.send(Err(err));
                return;
            }
        };
        let _ = udp.set_read_timeout(Some(Duration::from_millis(20)));

        let config = XraySessionConfig {
            conv: rand::random::<u16>(),
            mtu: opts.mtu as usize,
            tti: opts.tti,
            uplink_capacity: 5,
            downlink_capacity: 20,
            write_buffer_size: 2 * 1024 * 1024,
            packet_overhead: packet_mask.overhead(),
        };
        let mut session = XrayKcpSession::new(config);
        session.enqueue_application_data(&header);

        let started = Instant::now();
        let deadline = started + Duration::from_secs(5);
        let mut handshake_data = Vec::new();
        loop {
            let current = started.elapsed().as_millis() as u32;
            if let Err(err) =
                flush_session_packets(&mut session, &packet_mask, &udp, server_addr, current)
            {
                let _ = hs_tx.send(Err(err));
                return;
            }
            if let Err(err) =
                drain_udp_socket(&mut session, &packet_mask, &udp, server_addr, current)
            {
                let _ = hs_tx.send(Err(err));
                return;
            }
            while let Some(frame) = session.take_received() {
                handshake_data.extend_from_slice(&frame);
            }
            match consume_vless_response_prefix(&mut handshake_data) {
                Ok(Some(remaining)) => {
                    let _ = hs_tx.send(Ok(remaining));
                    break;
                }
                Ok(None) => {}
                Err(err) => {
                    let _ = hs_tx.send(Err(err));
                    return;
                }
            }
            if Instant::now() >= deadline {
                let _ = hs_tx.send(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "KCP VLESS response timeout",
                )));
                return;
            }
            thread::sleep(Duration::from_millis(opts.tti.max(1) as u64));
        }

        let mut app_write_closed = false;
        loop {
            let current = started.elapsed().as_millis() as u32;

            loop {
                match write_rx.try_recv() {
                    Ok(data) => {
                        if !data.is_empty() {
                            session.enqueue_application_data(&data);
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if !app_write_closed {
                            app_write_closed = true;
                            session.mark_application_write_closed(current);
                        }
                        break;
                    }
                }
            }

            if let Err(err) =
                flush_session_packets(&mut session, &packet_mask, &udp, server_addr, current)
            {
                let _ = read_tx.send(Vec::new());
                let _ = hs_tx.send(Err(err));
                return;
            }
            if let Err(err) =
                drain_udp_socket(&mut session, &packet_mask, &udp, server_addr, current)
            {
                let _ = read_tx.send(Vec::new());
                let _ = hs_tx.send(Err(err));
                return;
            }
            while let Some(frame) = session.take_received() {
                if read_tx.send(frame).is_err() {
                    return;
                }
            }

            if matches!(session.state(), XraySessionState::Terminated) {
                let _ = read_tx.send(Vec::new());
                return;
            }

            thread::sleep(Duration::from_millis(opts.tti.max(1) as u64));
        }
    });

    let initial_pending = hs_rx
        .recv()
        .map_err(|_| ClientError::Io(io::Error::other("KCP thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(KcpTunnel {
        read_rx,
        write_tx,
        read_buf: initial_pending,
        eof: false,
        _handle: handle,
    }))
}

struct KcpTunnel {
    read_rx: Receiver<Vec<u8>>,
    write_tx: SyncSender<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct KcpReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

#[derive(Clone)]
struct KcpWriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for KcpTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for KcpTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "KCP write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for KcpReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for KcpWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "KCP write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for KcpWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for KcpTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "KCP tunnel cannot be cloned (single KCP session)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let KcpTunnel {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(KcpReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(KcpWriteHalf { write_tx }),
        ))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn set_socket_timeouts(
        &self,
        _read: Option<Duration>,
        _write: Option<Duration>,
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
    match read_rx.recv() {
        Ok(data) => {
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
        Err(_) => {
            *eof = true;
            Ok(0)
        }
    }
}

fn flush_session_packets(
    session: &mut XrayKcpSession,
    packet_mask: &KcpPacketMask,
    udp: &UdpSocket,
    server_addr: SocketAddr,
    current: u32,
) -> io::Result<()> {
    for packet in session.flush(current) {
        let wrapped = packet_mask.wrap(&packet)?;
        udp.send_to(&wrapped, server_addr)?;
    }
    Ok(())
}

fn drain_udp_socket(
    session: &mut XrayKcpSession,
    packet_mask: &KcpPacketMask,
    udp: &UdpSocket,
    server_addr: SocketAddr,
    current: u32,
) -> io::Result<()> {
    let mut buf = [0u8; 2048];
    loop {
        match udp.recv_from(&mut buf) {
            Ok((n, src)) if src == server_addr => {
                if let Some(packet) = packet_mask.unwrap(&buf[..n]) {
                    session.input(&packet, current);
                }
            }
            Ok(_) => {}
            Err(ref err)
                if err.kind() == io::ErrorKind::WouldBlock
                    || err.kind() == io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn consume_vless_response_prefix(buffer: &mut Vec<u8>) -> io::Result<Option<Vec<u8>>> {
    if buffer.len() < 2 {
        return Ok(None);
    }
    if buffer[0] != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid VLESS response version: {}", buffer[0]),
        ));
    }
    let addons_len = buffer[1] as usize;
    if buffer.len() < 2 + addons_len {
        return Ok(None);
    }
    let remaining = buffer.split_off(2 + addons_len);
    buffer.clear();
    Ok(Some(remaining))
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

fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn xorfwd(data: &mut [u8]) {
    for i in 4..data.len() {
        data[i] ^= data[i - 4];
    }
}

fn xorbkd(data: &mut [u8]) {
    for i in (4..data.len()).rev() {
        data[i] ^= data[i - 4];
    }
}

enum KcpPacketMask {
    Original,
    Aes128Gcm { key: [u8; 16] },
}

impl KcpPacketMask {
    fn from_seed(seed: &str) -> Self {
        if seed.trim().is_empty() {
            Self::Original
        } else {
            let digest = sha2::Sha256::digest(seed.as_bytes());
            let mut key = [0u8; 16];
            key.copy_from_slice(&digest[..16]);
            Self::Aes128Gcm { key }
        }
    }

    fn overhead(&self) -> usize {
        match self {
            Self::Original => MKCP_ORIGINAL_OVERHEAD,
            Self::Aes128Gcm { .. } => 16,
        }
    }

    fn wrap(&self, plaintext: &[u8]) -> io::Result<Vec<u8>> {
        match self {
            Self::Original => {
                let mut packet = Vec::with_capacity(MKCP_ORIGINAL_OVERHEAD + plaintext.len() + 3);
                packet.extend_from_slice(&[0u8; MKCP_ORIGINAL_OVERHEAD]);
                packet[4..6].copy_from_slice(&(plaintext.len() as u16).to_be_bytes());
                packet.extend_from_slice(plaintext);
                let auth = fnv1a_32(&packet[4..]);
                packet[..4].copy_from_slice(&auth.to_be_bytes());
                let padded_len = if packet.len() % 4 == 0 {
                    packet.len()
                } else {
                    packet.len() + (4 - packet.len() % 4)
                };
                packet.resize(padded_len, 0);
                xorfwd(&mut packet);
                packet.truncate(MKCP_ORIGINAL_OVERHEAD + plaintext.len());
                Ok(packet)
            }
            Self::Aes128Gcm { key } => {
                let cipher = aes_gcm::Aes128Gcm::new_from_slice(key).expect("AES-GCM key length");
                let mut packet = vec![0u8; 12];
                rand::rngs::OsRng.fill_bytes(&mut packet);
                let nonce = aes_gcm::Nonce::from_slice(&packet[..12]);
                let mut ciphertext = plaintext.to_vec();
                let tag = cipher
                    .encrypt_in_place_detached(nonce, b"", &mut ciphertext)
                    .map_err(|e| io::Error::other(format!("mkcp wrap: {e}")))?;
                packet.extend_from_slice(&ciphertext);
                packet.extend_from_slice(tag.as_slice());
                Ok(packet)
            }
        }
    }

    fn unwrap(&self, packet: &[u8]) -> Option<Vec<u8>> {
        match self {
            Self::Original => {
                if packet.len() < MKCP_ORIGINAL_OVERHEAD {
                    return None;
                }
                let mut data = packet.to_vec();
                let padded_len = if data.len().is_multiple_of(4) {
                    data.len()
                } else {
                    data.len() + (4 - data.len() % 4)
                };
                data.resize(padded_len, 0);
                xorbkd(&mut data);
                let auth = u32::from_be_bytes(data[..4].try_into().ok()?);
                if fnv1a_32(&data[4..packet.len()]) != auth {
                    return None;
                }
                let length = u16::from_be_bytes(data[4..6].try_into().ok()?) as usize;
                if packet.len().checked_sub(MKCP_ORIGINAL_OVERHEAD)? != length {
                    return None;
                }
                Some(data[6..6 + length].to_vec())
            }
            Self::Aes128Gcm { key } => {
                if packet.len() < 12 + 16 {
                    return None;
                }
                let cipher = aes_gcm::Aes128Gcm::new_from_slice(key).ok()?;
                let nonce = aes_gcm::Nonce::from_slice(&packet[..12]);
                let split = packet.len() - 16;
                let mut plaintext = packet[12..split].to_vec();
                cipher
                    .decrypt_in_place_detached(
                        nonce,
                        b"",
                        &mut plaintext,
                        aes_gcm::Tag::from_slice(&packet[split..]),
                    )
                    .ok()?;
                Some(plaintext)
            }
        }
    }
}
