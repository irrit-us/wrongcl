use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::endpoint::{OuterSecurity, TlsOptions, XhttpOptions};
use crate::error::{ClientError, Result};
use crate::tls;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub fn connect_xhttp(
    server_host: &str,
    server_port: u16,
    opts: &XhttpOptions,
    outer: &OuterSecurity,
) -> Result<Box<dyn Tunnel>> {
    let tls_config = match outer {
        OuterSecurity::None => None,
        OuterSecurity::Tls(tls_opts) => Some((
            Arc::new(tls::build_client_config(tls_opts)?),
            tls_opts.clone(),
        )),
        OuterSecurity::Reality(_) | OuterSecurity::AnyTls(_) | OuterSecurity::ShadowTls(_) => {
            return Err(ClientError::Config(
                "XHTTP transport only supports 'none' or 'tls' outer security".into(),
            ));
        }
    };

    let addr = format!("{server_host}:{server_port}");
    let path = opts.path.clone();
    let authority = host_header(opts.host.as_deref(), server_host, server_port);

    let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>();
    let (write_tx, write_rx) = mpsc::sync_channel::<Vec<u8>>(32);
    let (hs_tx, hs_rx) = mpsc::sync_channel::<std::result::Result<(), io::Error>>(1);

    let (tokio_write_tx, mut tokio_write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);

    let handle = std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = hs_tx.send(Err(io::Error::other(format!("tokio runtime: {e}"))));
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
            let tcp = match tokio::time::timeout(
                HANDSHAKE_TIMEOUT,
                tokio::net::TcpStream::connect(&addr),
            )
            .await
            {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => {
                    let _ = hs_tx.send(Err(io::Error::other(format!("TCP connect: {e}"))));
                    return;
                }
                Err(_) => {
                    let _ = hs_tx.send(Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "TCP connect timed out",
                    )));
                    return;
                }
            };
            let _ = tcp.set_nodelay(true);

            let handshake = tokio::time::timeout(
                HANDSHAKE_TIMEOUT,
                xhttp_handshake(tcp, tls_config, &path, &authority),
            )
            .await;

            let (body, send_stream) = match handshake {
                Ok(Ok(pair)) => pair,
                Ok(Err(e)) => {
                    let _ = hs_tx.send(Err(e));
                    return;
                }
                Err(_) => {
                    let _ = hs_tx.send(Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "XHTTP handshake timed out",
                    )));
                    return;
                }
            };
            let _ = hs_tx.send(Ok(()));
            stream_loop(body, send_stream, read_tx, &mut tokio_write_rx).await;
        });
    });

    hs_rx
        .recv()
        .map_err(|_| ClientError::Io(io::Error::other("XHTTP thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(XhttpTunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

fn host_header(host: Option<&str>, server_host: &str, server_port: u16) -> String {
    match host {
        Some(h) if !h.trim().is_empty() => h.trim().to_string(),
        _ => {
            if (server_port == 80) || (server_port == 443) {
                server_host.to_string()
            } else {
                format!("{server_host}:{server_port}")
            }
        }
    }
}

enum H2Body {
    Plain(h2::RecvStream),
    Tls(h2::RecvStream),
}

impl H2Body {
    async fn data(&mut self) -> Option<std::result::Result<bytes::Bytes, h2::Error>> {
        match self {
            H2Body::Plain(b) | H2Body::Tls(b) => b.data().await,
        }
    }
}

async fn xhttp_handshake(
    tcp: tokio::net::TcpStream,
    tls_pair: Option<(Arc<rustls::ClientConfig>, TlsOptions)>,
    path: &str,
    authority: &str,
) -> std::result::Result<(H2Body, h2::SendStream<bytes::Bytes>), io::Error> {
    let (client, scheme) = if let Some((cfg, tls_opts)) = tls_pair {
        let mut cfg = (*cfg).clone();
        if cfg.alpn_protocols.is_empty() {
            cfg.alpn_protocols = vec![b"h2".to_vec()];
        }
        let server_name = rustls::pki_types::ServerName::try_from(tls_opts.server_name.clone())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid SNI"))?;
        let connector = tokio_rustls::TlsConnector::from(Arc::new(cfg));
        let tls_stream = connector
            .connect(server_name, tcp)
            .await
            .map_err(|e| io::Error::other(format!("TLS: {e}")))?;
        let (client, conn) = h2::client::Builder::new()
            .initial_window_size(1_048_576)
            .handshake(tls_stream)
            .await
            .map_err(|e| io::Error::other(format!("h2: {e}")))?;
        tokio::spawn(async move {
            let _ = conn.await;
        });
        (client, "https")
    } else {
        let (client, conn) = h2::client::Builder::new()
            .initial_window_size(1_048_576)
            .handshake(tcp)
            .await
            .map_err(|e| io::Error::other(format!("h2: {e}")))?;
        tokio::spawn(async move {
            let _ = conn.await;
        });
        (client, "http")
    };

    let mut client = client
        .ready()
        .await
        .map_err(|e| io::Error::other(format!("h2 ready: {e}")))?;

    let uri = format!("{scheme}://{authority}{path}");
    let request = http::Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .header("content-type", "application/octet-stream")
        .body(())
        .map_err(|e| io::Error::other(format!("h2 request build: {e}")))?;

    let (response, send_stream) = client
        .send_request(request, false)
        .map_err(|e| io::Error::other(format!("send: {e}")))?;

    let response = response
        .await
        .map_err(|e| io::Error::other(format!("response: {e}")))?;
    if response.status() != http::StatusCode::OK {
        return Err(io::Error::other(format!(
            "XHTTP status: {}",
            response.status()
        )));
    }

    let body = response.into_body();
    let body = if scheme == "https" {
        H2Body::Tls(body)
    } else {
        H2Body::Plain(body)
    };
    Ok((body, send_stream))
}

async fn stream_loop(
    mut body: H2Body,
    mut send_stream: h2::SendStream<bytes::Bytes>,
    read_tx: mpsc::Sender<Vec<u8>>,
    tokio_write_rx: &mut tokio::sync::mpsc::Receiver<Vec<u8>>,
) {
    loop {
        tokio::select! {
            result = body.data() => {
                match result {
                    Some(Ok(data)) => {
                        if !data.is_empty() && read_tx.send(data.to_vec()).is_err() {
                            break;
                        }
                    }
                    Some(Err(_)) | None => {
                        let _ = read_tx.send(Vec::new());
                        break;
                    }
                }
            }
            maybe_data = tokio_write_rx.recv() => {
                match maybe_data {
                    Some(data) => {
                        if send_stream
                            .send_data(bytes::Bytes::from(data), false)
                            .is_err()
                        {
                            break;
                        }
                    }
                    None => {
                        let _ = send_stream.send_data(bytes::Bytes::new(), true);
                        break;
                    }
                }
            }
        }
    }
}

struct XhttpTunnel {
    read_rx: Receiver<Vec<u8>>,
    write_tx: SyncSender<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct XhttpReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct XhttpWriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for XhttpTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Read for XhttpReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for XhttpTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "XHTTP write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for XhttpWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "XHTTP write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for XhttpWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for XhttpTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "XHTTP tunnel cannot be cloned (single h2 stream)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let XhttpTunnel {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(XhttpReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(XhttpWriteHalf { write_tx }),
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
