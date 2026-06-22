use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hmac::{Hmac, Mac};
use rustls::ClientConnection;
use rustls::pki_types::ServerName;
use sha1::Sha1;

use crate::client::{Tunnel, split_cloneable_tunnel};
use crate::endpoint::{ShadowTlsOptions, TlsOptions};
use crate::error::{ClientError, Result};
use crate::tls;

type HmacSha1 = Hmac<Sha1>;

const TLS_HEADER_SIZE: usize = 5;
const TLS_RANDOM_SIZE: usize = 32;
const TLS_SESSION_ID_SIZE: usize = 32;
const SHADOWTLS_RECORD_CHUNK: usize = 0x3fff;
const HANDSHAKE: u8 = 22;
const ALERT: u8 = 21;
const APPLICATION_DATA: u8 = 23;
const CLIENT_HELLO: u8 = 1;
const SERVER_HELLO: u8 = 2;
const HMAC_SIZE: usize = 4;
const TLS_HMAC_HEADER_SIZE: usize = TLS_HEADER_SIZE + HMAC_SIZE;
const SERVER_RANDOM_INDEX: usize = TLS_HEADER_SIZE + 1 + 3 + 2;
const SESSION_ID_LENGTH_INDEX: usize = TLS_HEADER_SIZE + 1 + 3 + 2 + TLS_RANDOM_SIZE;

struct HandshakeRelayState {
    hmac: HmacSha1,
}

struct ShadowTlsReadState {
    post_auth: HmacSha1,
    handshake_relay: Option<HandshakeRelayState>,
}

struct ShadowTlsWriteState {
    post_auth: HmacSha1,
}

pub fn wrap(mut socket: TcpStream, opts: &ShadowTlsOptions) -> Result<Box<dyn Tunnel>> {
    let client_hello = build_shadowtls_client_hello(opts)?;
    socket.write_all(&client_hello)?;
    socket.flush()?;

    let server_hello = extract_tls_frame(&mut socket)?;
    let server_random = extract_server_random(&server_hello).ok_or_else(|| {
        ClientError::Config("ShadowTLS server random missing from ServerHello".into())
    })?;

    let read_state = ShadowTlsReadState {
        post_auth: seed_shadowtls_hmac(&opts.password, &server_random, b"S")?,
        handshake_relay: Some(HandshakeRelayState {
            hmac: seed_shadowtls_hmac(&opts.password, &server_random, b"")?,
        }),
    };
    let write_state = ShadowTlsWriteState {
        post_auth: seed_shadowtls_hmac(&opts.password, &server_random, b"C")?,
    };

    Ok(Box::new(ShadowTlsTunnel {
        read_state: Arc::new(Mutex::new(read_state)),
        write_state: Arc::new(Mutex::new(write_state)),
        write_lock: Arc::new(Mutex::new(())),
        socket,
        read_buf: Vec::new(),
    }))
}

fn build_shadowtls_client_hello(opts: &ShadowTlsOptions) -> Result<Vec<u8>> {
    let tls_opts = TlsOptions {
        server_name: opts.server_name.clone(),
        insecure_skip_verify: true,
        alpn: Vec::new(),
    };
    let config = Arc::new(tls::build_client_config(&tls_opts)?);
    let server_name = ServerName::try_from(opts.server_name.clone())
        .map_err(|e| ClientError::Config(format!("invalid ShadowTLS server-name: {e}")))?;
    let mut conn = ClientConnection::new(config, server_name)
        .map_err(|e| ClientError::Config(format!("ShadowTLS TLS init: {e}")))?;
    let mut client_hello = drain_client_hello(&mut conn)?;
    patch_client_hello_hmac(&mut client_hello, &opts.password)?;
    Ok(client_hello)
}

fn drain_client_hello(conn: &mut ClientConnection) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    while conn.wants_write() {
        let before = out.len();
        conn.write_tls(&mut out).map_err(ClientError::Io)?;
        if out.len() == before {
            break;
        }
    }
    if out.is_empty() {
        return Err(ClientError::Config(
            "ShadowTLS ClientHello generation produced no TLS bytes".into(),
        ));
    }
    Ok(out)
}

fn patch_client_hello_hmac(frame: &mut [u8], password: &str) -> Result<()> {
    if frame.len() < TLS_HEADER_SIZE {
        return Err(ClientError::Config(
            "ShadowTLS ClientHello was shorter than a TLS record header".into(),
        ));
    }
    let record_len = u16::from_be_bytes([frame[3], frame[4]]) as usize;
    let record_end = TLS_HEADER_SIZE + record_len;
    if frame.len() < record_end {
        return Err(ClientError::Config(
            "ShadowTLS ClientHello record was truncated".into(),
        ));
    }
    let record = &mut frame[..record_end];
    if record[0] != HANDSHAKE || record[TLS_HEADER_SIZE] != CLIENT_HELLO {
        return Err(ClientError::Config(
            "ShadowTLS expected an initial TLS ClientHello record".into(),
        ));
    }
    if record.len() <= SESSION_ID_LENGTH_INDEX {
        return Err(ClientError::Config(
            "ShadowTLS ClientHello did not contain a session id".into(),
        ));
    }
    if record[SESSION_ID_LENGTH_INDEX] != TLS_SESSION_ID_SIZE as u8 {
        return Err(ClientError::Config(format!(
            "ShadowTLS expected a 32-byte TLS session id, got {} bytes",
            record[SESSION_ID_LENGTH_INDEX]
        )));
    }

    let hmac_index = SESSION_ID_LENGTH_INDEX + 1 + TLS_SESSION_ID_SIZE - HMAC_SIZE;
    if record.len() < hmac_index + HMAC_SIZE {
        return Err(ClientError::Config(
            "ShadowTLS ClientHello session id was too short for auth".into(),
        ));
    }

    let mut hmac = new_shadowtls_hmac(password)?;
    hmac.update(&record[TLS_HEADER_SIZE..hmac_index]);
    hmac.update(&[0, 0, 0, 0]);
    hmac.update(&record[hmac_index + HMAC_SIZE..]);
    let sum = hmac.finalize().into_bytes();
    record[hmac_index..hmac_index + HMAC_SIZE].copy_from_slice(&sum[..HMAC_SIZE]);
    Ok(())
}

pub struct ShadowTlsTunnel {
    read_state: Arc<Mutex<ShadowTlsReadState>>,
    write_state: Arc<Mutex<ShadowTlsWriteState>>,
    write_lock: Arc<Mutex<()>>,
    socket: TcpStream,
    read_buf: Vec<u8>,
}

impl Read for ShadowTlsTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.read_buf.is_empty() {
            let n = self.read_buf.len().min(buf.len());
            buf[..n].copy_from_slice(&self.read_buf[..n]);
            self.read_buf.drain(..n);
            return Ok(n);
        }

        loop {
            let frame = extract_tls_frame(&mut self.socket)?;
            let payload = {
                let mut state = lock_read_state(&self.read_state)?;
                if try_consume_handshake_relay_frame(&frame, &mut state)? {
                    continue;
                }
                decode_shadowtls_application_data(&frame, &mut state.post_auth)?
            };
            if payload.is_empty() {
                continue;
            }
            let n = payload.len().min(buf.len());
            buf[..n].copy_from_slice(&payload[..n]);
            if n < payload.len() {
                self.read_buf.extend_from_slice(&payload[n..]);
            }
            return Ok(n);
        }
    }
}

impl Write for ShadowTlsTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut frames = Vec::new();
        {
            let mut state = lock_write_state(&self.write_state)?;
            for chunk in buf.chunks(SHADOWTLS_RECORD_CHUNK) {
                frames.push(encode_shadowtls_application_data(
                    chunk,
                    &mut state.post_auth,
                )?);
            }
        }

        if frames.is_empty() {
            return Ok(0);
        }
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| io::Error::other("ShadowTLS write lock poisoned"))?;
        for frame in &frames {
            self.socket.write_all(frame)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| io::Error::other("ShadowTLS write lock poisoned"))?;
        self.socket.flush()
    }
}

impl Tunnel for ShadowTlsTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(Self {
            read_state: Arc::clone(&self.read_state),
            write_state: Arc::clone(&self.write_state),
            write_lock: Arc::clone(&self.write_lock),
            socket: self.socket.try_clone()?,
            read_buf: Vec::new(),
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
        self.socket.shutdown(Shutdown::Write)
    }

    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        self.socket.set_read_timeout(read)?;
        self.socket.set_write_timeout(write)?;
        Ok(())
    }
}

fn try_consume_handshake_relay_frame(
    frame: &[u8],
    state: &mut ShadowTlsReadState,
) -> io::Result<bool> {
    let Some(relay) = state.handshake_relay.as_mut() else {
        return Ok(false);
    };

    if frame[0] == ALERT {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "ShadowTLS peer sent TLS alert",
        ));
    }

    if frame[0] != APPLICATION_DATA {
        return Ok(true);
    }
    if frame.len() < TLS_HMAC_HEADER_SIZE || frame[1] != 3 || frame[2] != 3 {
        state.handshake_relay = None;
        return Ok(false);
    }

    let payload = &frame[TLS_HMAC_HEADER_SIZE..];
    let mut candidate = relay.hmac.clone();
    candidate.update(payload);
    let expected = candidate.clone().finalize().into_bytes();
    if frame[TLS_HEADER_SIZE..TLS_HMAC_HEADER_SIZE] != expected[..HMAC_SIZE] {
        state.handshake_relay = None;
        return Ok(false);
    }

    relay.hmac = candidate;
    Ok(true)
}

fn lock_read_state(
    state: &Arc<Mutex<ShadowTlsReadState>>,
) -> io::Result<std::sync::MutexGuard<'_, ShadowTlsReadState>> {
    state
        .lock()
        .map_err(|_| io::Error::other("ShadowTLS read state lock poisoned"))
}

fn lock_write_state(
    state: &Arc<Mutex<ShadowTlsWriteState>>,
) -> io::Result<std::sync::MutexGuard<'_, ShadowTlsWriteState>> {
    state
        .lock()
        .map_err(|_| io::Error::other("ShadowTLS write state lock poisoned"))
}

fn new_shadowtls_hmac(password: &str) -> Result<HmacSha1> {
    HmacSha1::new_from_slice(password.as_bytes())
        .map_err(|e| ClientError::Config(format!("ShadowTLS HMAC init: {e}")))
}

fn seed_shadowtls_hmac(
    password: &str,
    server_random: &[u8; TLS_RANDOM_SIZE],
    suffix: &[u8],
) -> Result<HmacSha1> {
    let mut hmac = new_shadowtls_hmac(password)?;
    hmac.update(server_random);
    hmac.update(suffix);
    Ok(hmac)
}

fn extract_tls_frame(reader: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut header = [0u8; TLS_HEADER_SIZE];
    reader.read_exact(&mut header)?;
    let len = u16::from_be_bytes([header[3], header[4]]) as usize;
    let mut frame = Vec::with_capacity(TLS_HEADER_SIZE + len);
    frame.extend_from_slice(&header);
    frame.resize(TLS_HEADER_SIZE + len, 0);
    reader.read_exact(&mut frame[TLS_HEADER_SIZE..])?;
    Ok(frame)
}

fn extract_server_random(frame: &[u8]) -> Option<[u8; TLS_RANDOM_SIZE]> {
    let min_len = TLS_HEADER_SIZE + 1 + 3 + 2 + TLS_RANDOM_SIZE;
    if frame.len() < min_len || frame[0] != HANDSHAKE || frame[TLS_HEADER_SIZE] != SERVER_HELLO {
        return None;
    }

    let mut random = [0u8; TLS_RANDOM_SIZE];
    random.copy_from_slice(&frame[SERVER_RANDOM_INDEX..SERVER_RANDOM_INDEX + TLS_RANDOM_SIZE]);
    Some(random)
}

fn encode_shadowtls_application_data(payload: &[u8], state: &mut HmacSha1) -> io::Result<Vec<u8>> {
    let record_len = HMAC_SIZE + payload.len();
    if record_len > u16::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "shadowtls record too large",
        ));
    }

    state.update(payload);
    let hmac = state.clone().finalize().into_bytes();
    state.update(&hmac[..HMAC_SIZE]);

    let mut frame = Vec::with_capacity(TLS_HMAC_HEADER_SIZE + payload.len());
    frame.push(APPLICATION_DATA);
    frame.push(3);
    frame.push(3);
    frame.extend_from_slice(&(record_len as u16).to_be_bytes());
    frame.extend_from_slice(&hmac[..HMAC_SIZE]);
    frame.extend_from_slice(payload);
    Ok(frame)
}

fn decode_shadowtls_application_data(frame: &[u8], state: &mut HmacSha1) -> io::Result<Vec<u8>> {
    if frame.len() < TLS_HMAC_HEADER_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "shadowtls frame too short",
        ));
    }
    if frame[0] != APPLICATION_DATA || frame[1] != 3 || frame[2] != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unexpected ShadowTLS record type {}", frame[0]),
        ));
    }

    let payload = &frame[TLS_HMAC_HEADER_SIZE..];
    state.update(payload);
    let expected = state.clone().finalize().into_bytes();
    if frame[TLS_HEADER_SIZE..TLS_HMAC_HEADER_SIZE] != expected[..HMAC_SIZE] {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "shadowtls application data verification failed",
        ));
    }
    state.update(&frame[TLS_HEADER_SIZE..TLS_HMAC_HEADER_SIZE]);
    Ok(payload.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadowtls_application_data_roundtrip() {
        let server_random = [7u8; TLS_RANDOM_SIZE];
        let mut writer = seed_shadowtls_hmac("secret", &server_random, b"S").unwrap();
        let mut reader = seed_shadowtls_hmac("secret", &server_random, b"S").unwrap();
        let frame = encode_shadowtls_application_data(b"hello-shadowtls", &mut writer).unwrap();
        let payload = decode_shadowtls_application_data(&frame, &mut reader).unwrap();
        assert_eq!(payload, b"hello-shadowtls");
    }
}
