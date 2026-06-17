use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, ClientConnection, DigitallySignedStruct, RootCertStore, SignatureScheme,
};

use crate::client::{split_cloneable_tunnel, Tunnel};
use crate::endpoint::TlsOptions;
use crate::error::{ClientError, Result};

pub fn wrap(socket: TcpStream, opts: &TlsOptions) -> Result<Box<dyn Tunnel>> {
    let config = Arc::new(build_client_config(opts)?);
    let server_name = ServerName::try_from(opts.server_name.clone())
        .map_err(|e| ClientError::Config(format!("invalid TLS server-name: {e}")))?;
    let conn = ClientConnection::new(config, server_name)
        .map_err(|e| ClientError::Config(format!("TLS init: {e}")))?;

    let mut state = TlsState { conn };
    let mut socket = socket;
    perform_handshake(&mut state, &mut socket)?;

    let inner = Arc::new(Mutex::new(state));
    let write_lock = Arc::new(Mutex::new(()));
    Ok(Box::new(TlsTunnel {
        state: inner,
        write_lock,
        socket,
    }))
}

fn perform_handshake(state: &mut TlsState, socket: &mut TcpStream) -> Result<()> {
    while state.conn.is_handshaking() {
        let mut progress = false;
        if state.conn.wants_write() {
            while state.conn.wants_write() {
                state.conn.write_tls(socket).map_err(ClientError::Io)?;
                progress = true;
            }
        }
        if state.conn.is_handshaking() && state.conn.wants_read() {
            let n = state.conn.read_tls(socket).map_err(ClientError::Io)?;
            if n == 0 {
                return Err(ClientError::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TLS handshake: peer closed",
                )));
            }
            state
                .conn
                .process_new_packets()
                .map_err(|e| ClientError::Config(format!("TLS handshake: {e}")))?;
            progress = true;
        }
        if !progress {
            return Err(ClientError::Config(
                "TLS handshake stalled with no I/O progress".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn build_client_config(opts: &TlsOptions) -> Result<ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| ClientError::Config(format!("TLS config: {e}")))?;

    let mut config = if opts.insecure_skip_verify {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertVerify::new()))
            .with_no_client_auth()
    } else {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        builder.with_root_certificates(roots).with_no_client_auth()
    };

    if !opts.alpn.is_empty() {
        config.alpn_protocols = opts
            .alpn
            .iter()
            .map(|protocol| protocol.as_bytes().to_vec())
            .collect();
    }
    Ok(config)
}

struct TlsState {
    conn: ClientConnection,
}

pub struct TlsTunnel {
    state: Arc<Mutex<TlsState>>,
    write_lock: Arc<Mutex<()>>,
    socket: TcpStream,
}

impl TlsTunnel {
    pub fn socket_ref(&self) -> &TcpStream {
        &self.socket
    }
}

impl Read for TlsTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let pending = {
                let mut guard = lock_state(&self.state)?;
                match guard.conn.reader().read(buf) {
                    Ok(n) => return Ok(n),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e),
                }
                drain_writes(&mut guard.conn)?
            };
            self.write_drained(&pending)?;

            let mut tmp = [0u8; 16 * 1024];
            let n = self.socket.read(&mut tmp)?;
            if n == 0 {
                return Ok(0);
            }

            let pending_after = {
                let mut guard = lock_state(&self.state)?;
                let mut cursor: &[u8] = &tmp[..n];
                while !cursor.is_empty() {
                    let consumed = guard.conn.read_tls(&mut cursor)?;
                    if consumed == 0 {
                        break;
                    }
                }
                guard
                    .conn
                    .process_new_packets()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                drain_writes(&mut guard.conn)?
            };
            self.write_drained(&pending_after)?;
        }
    }
}

impl Write for TlsTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let (written, pending) = {
            let mut guard = lock_state(&self.state)?;
            let written = guard.conn.writer().write(buf)?;
            let pending = drain_writes(&mut guard.conn)?;
            (written, pending)
        };
        self.write_drained(&pending)?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        let pending = {
            let mut guard = lock_state(&self.state)?;
            drain_writes(&mut guard.conn)?
        };
        self.write_drained(&pending)?;
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| io::Error::other("TLS write lock poisoned"))?;
        self.socket.flush()
    }
}

impl TlsTunnel {
    fn write_drained(&mut self, bytes: &[u8]) -> io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| io::Error::other("TLS write lock poisoned"))?;
        self.socket.write_all(bytes)
    }
}

impl Tunnel for TlsTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(TlsTunnel {
            state: Arc::clone(&self.state),
            write_lock: Arc::clone(&self.write_lock),
            socket: self.socket.try_clone()?,
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
        let pending = {
            let mut guard = lock_state(&self.state)?;
            guard.conn.send_close_notify();
            drain_writes(&mut guard.conn).unwrap_or_default()
        };
        let _ = self.write_drained(&pending);
        self.socket.shutdown(std::net::Shutdown::Write)
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

fn lock_state(state: &Arc<Mutex<TlsState>>) -> io::Result<std::sync::MutexGuard<'_, TlsState>> {
    state
        .lock()
        .map_err(|_| io::Error::other("TLS connection lock poisoned"))
}

fn drain_writes(conn: &mut ClientConnection) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    while conn.wants_write() {
        let before = out.len();
        conn.write_tls(&mut out)?;
        if out.len() == before {
            break;
        }
    }
    Ok(out)
}

#[derive(Debug)]
struct NoCertVerify {
    schemes: Vec<SignatureScheme>,
}

impl NoCertVerify {
    fn new() -> Self {
        let schemes = CryptoProvider::get_default()
            .map(|p| p.signature_verification_algorithms.supported_schemes())
            .unwrap_or_else(|| {
                rustls::crypto::ring::default_provider()
                    .signature_verification_algorithms
                    .supported_schemes()
            });
        Self { schemes }
    }
}

impl ServerCertVerifier for NoCertVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes.clone()
    }
}
