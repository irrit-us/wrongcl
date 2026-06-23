use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine as _;
use rand::RngCore;

use crate::client::{Tunnel, TunnelReader, TunnelWriter, connect_tcp};
use crate::endpoint::{GdocsViewerOptions, OuterSecurity};
use crate::error::{ClientError, Result};
use crate::protocol::{
    Target, encode_raw_vless_header, encode_udp_vless_header, read_raw_vless_response,
};
use crate::tls;

mod util;

const ROUNDTRIP_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const ENCRYPTED_REQUEST_VERSION: u8 = 1;
const RESPONSE_FRAME_SUCCESS: u8 = 0;
const RESPONSE_FRAME_ERROR: u8 = 1;

use util::{
    build_http_get, normalized_path_prefix, random_path_segment, random_session_bytes,
    read_channel, read_http_response, request_host_header,
};

enum GdocsSession {
    Plain { session_id: String },
    Encrypted { session: Vec<u8>, key: [u8; 32] },
}

impl GdocsSession {
    fn request_path(&self, path_prefix: &str, payload: &[u8]) -> Result<String> {
        match self {
            GdocsSession::Plain { session_id } => {
                let payload_segment =
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
                let nonce = random_path_segment();
                Ok(format!(
                    "{path_prefix}/r/{session_id}/{payload_segment}/{nonce}.txt"
                ))
            }
            GdocsSession::Encrypted { session, key } => {
                let mut frame = Vec::with_capacity(3 + session.len() + payload.len());
                frame.push(ENCRYPTED_REQUEST_VERSION);
                frame.extend_from_slice(&(session.len() as u16).to_be_bytes());
                frame.extend_from_slice(session);
                frame.extend_from_slice(payload);
                let combined = encrypt_aead(key, &frame)?;
                let path_segment =
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(combined);
                Ok(format!("{path_prefix}/t/{path_segment}.log"))
            }
        }
    }

    fn decode_response(&self, body: &[u8]) -> Result<Vec<u8>> {
        match self {
            GdocsSession::Plain { .. } => base64::engine::general_purpose::STANDARD
                .decode(body)
                .map_err(|_| {
                    ClientError::Config("invalid Google Docs Viewer response encoding".into())
                }),
            GdocsSession::Encrypted { key, .. } => {
                let ciphertext = base64::engine::general_purpose::STANDARD
                    .decode(body)
                    .map_err(|_| {
                        ClientError::Config(
                            "invalid encrypted Google Docs Viewer response encoding".into(),
                        )
                    })?;
                let frame = decrypt_aead(key, &ciphertext)
                    .map_err(|error| ClientError::Config(error.to_string()))?;
                let Some((&status, payload)) = frame.split_first() else {
                    return Err(ClientError::Config(
                        "encrypted Google Docs Viewer response frame is empty".into(),
                    ));
                };
                match status {
                    RESPONSE_FRAME_SUCCESS => Ok(payload.to_vec()),
                    RESPONSE_FRAME_ERROR => Err(ClientError::Config(format!(
                        "Google Docs Viewer response error: {}",
                        String::from_utf8_lossy(payload)
                    ))),
                    other => Err(ClientError::Config(format!(
                        "unknown Google Docs Viewer response frame type: {other}"
                    ))),
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn connect_gdocsviewer(
    server_host: &str,
    server_port: u16,
    opts: &GdocsViewerOptions,
    outer_security: &OuterSecurity,
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
    let session = build_session(opts.shared_key.as_deref())?;
    let mut initial_response = gdocsviewer_roundtrip(
        server_host,
        server_port,
        opts,
        outer_security,
        &session,
        &header,
    )?;
    let deadline = Instant::now() + ROUNDTRIP_TIMEOUT;
    while initial_response.is_empty() && Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);
        initial_response = gdocsviewer_roundtrip(
            server_host,
            server_port,
            opts,
            outer_security,
            &session,
            &[],
        )?;
    }
    if initial_response.is_empty() {
        return Err(ClientError::Io(io::Error::new(
            io::ErrorKind::TimedOut,
            "Google Docs Viewer VLESS response timeout",
        )));
    }

    let mut cursor = io::Cursor::new(initial_response.as_slice());
    read_raw_vless_response(&mut cursor)?;
    let initial_payload = initial_response[cursor.position() as usize..].to_vec();

    let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>();
    let (write_tx, write_rx) = mpsc::sync_channel::<Vec<u8>>(32);
    let server_host = server_host.to_string();
    let opts = opts.clone();
    let outer_security = outer_security.clone();

    let handle = std::thread::spawn(move || {
        if !initial_payload.is_empty() && read_tx.send(initial_payload).is_err() {
            return;
        }

        loop {
            let mut request_body = match write_rx.recv_timeout(POLL_INTERVAL) {
                Ok(data) => data,
                Err(mpsc::RecvTimeoutError::Timeout) => Vec::new(),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            while let Ok(more) = write_rx.try_recv() {
                request_body.extend_from_slice(&more);
            }

            match gdocsviewer_roundtrip(
                &server_host,
                server_port,
                &opts,
                &outer_security,
                &session,
                &request_body,
            ) {
                Ok(response) => {
                    if !response.is_empty() && read_tx.send(response).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = read_tx.send(Vec::new());
                    break;
                }
            }
        }
    });

    Ok(Box::new(GdocsViewerTunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

struct GdocsViewerTunnel {
    read_rx: Receiver<Vec<u8>>,
    write_tx: SyncSender<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct GdocsViewerReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

#[derive(Clone)]
struct GdocsViewerWriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for GdocsViewerTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for GdocsViewerTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx.send(buf.to_vec()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Google Docs Viewer write channel closed",
            )
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for GdocsViewerReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for GdocsViewerWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx.send(buf.to_vec()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Google Docs Viewer write channel closed",
            )
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for GdocsViewerWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for GdocsViewerTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Google Docs Viewer tunnel cannot be cloned (single request session)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let GdocsViewerTunnel {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(GdocsViewerReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(GdocsViewerWriteHalf { write_tx }),
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

fn build_session(shared_key: Option<&str>) -> Result<GdocsSession> {
    match shared_key {
        Some(shared_key) => Ok(GdocsSession::Encrypted {
            session: random_session_bytes(),
            key: decode_shared_key(shared_key)?,
        }),
        None => Ok(GdocsSession::Plain {
            session_id: random_path_segment(),
        }),
    }
}

fn gdocsviewer_roundtrip(
    server_host: &str,
    server_port: u16,
    opts: &GdocsViewerOptions,
    outer_security: &OuterSecurity,
    session: &GdocsSession,
    body: &[u8],
) -> Result<Vec<u8>> {
    let mut stream = open_gdocsviewer_connection(server_host, server_port, outer_security)?;
    let path_prefix = normalized_path_prefix(&opts.path_prefix, "/gdocsviewer");
    let path = session.request_path(&path_prefix, body)?;
    let host = request_host_header(outer_security, server_host, server_port);
    let request = build_http_get(&path, &host);
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    let response = read_http_response(stream.as_mut()).map_err(ClientError::Io)?;
    session.decode_response(&response)
}

fn open_gdocsviewer_connection(
    server_host: &str,
    server_port: u16,
    outer_security: &OuterSecurity,
) -> Result<Box<dyn Tunnel>> {
    let socket = connect_tcp(server_host, server_port)?;
    match outer_security {
        OuterSecurity::None => {
            socket.set_read_timeout(Some(ROUNDTRIP_TIMEOUT))?;
            socket.set_write_timeout(Some(ROUNDTRIP_TIMEOUT))?;
            Ok(Box::new(socket))
        }
        OuterSecurity::Tls(opts) => {
            let stream = tls::wrap(socket, opts)?;
            stream.set_socket_timeouts(Some(ROUNDTRIP_TIMEOUT), Some(ROUNDTRIP_TIMEOUT))?;
            Ok(stream)
        }
        other => Err(ClientError::Config(format!(
            "Google Docs Viewer only supports none or TLS outer security, got {}",
            other.id()
        ))),
    }
}

fn encrypt_aead(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| ClientError::Config(format!("Google Docs Viewer aes-gcm init: {e}")))?;
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|e| ClientError::Config(format!("Google Docs Viewer aes-gcm encrypt: {e}")))?;
    let mut out = Vec::with_capacity(nonce.len() + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt_aead(key: &[u8; 32], combined: &[u8]) -> Result<Vec<u8>> {
    if combined.len() < 12 + 16 {
        return Err(ClientError::Config(
            "encrypted Google Docs Viewer frame is too short".into(),
        ));
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| ClientError::Config(format!("Google Docs Viewer aes-gcm init: {e}")))?;
    cipher
        .decrypt(Nonce::from_slice(&combined[..12]), &combined[12..])
        .map_err(|e| ClientError::Config(format!("Google Docs Viewer aes-gcm decrypt: {e}")))
}

fn decode_shared_key(shared_key: &str) -> Result<[u8; 32]> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(shared_key)
        .map_err(|_| ClientError::Config("Google Docs Viewer shared-key must be base64".into()))?;
    decoded.try_into().map_err(|_| {
        ClientError::Config("Google Docs Viewer shared-key must decode to 32 bytes".into())
    })
}
