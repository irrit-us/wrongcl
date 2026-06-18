use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::endpoint::{GrpcOptions, OuterSecurity, TlsOptions};
use crate::error::{ClientError, Result};
use crate::tls;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub fn connect_grpc(
    server_host: &str,
    server_port: u16,
    opts: &GrpcOptions,
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
                "gRPC transport only supports 'none' or 'tls' outer security".into(),
            ));
        }
    };

    let addr = format!("{server_host}:{server_port}");
    let service_name = opts.service_name.clone();
    let authority = host_header(server_host, server_port);

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
                grpc_handshake(tcp, tls_config, &service_name, &authority),
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
                        "gRPC handshake timed out",
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
        .map_err(|_| ClientError::Io(io::Error::other("gRPC thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(GrpcTunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

fn host_header(server_host: &str, server_port: u16) -> String {
    if server_port == 80 || server_port == 443 {
        server_host.to_string()
    } else {
        format!("{server_host}:{server_port}")
    }
}

async fn grpc_handshake(
    tcp: tokio::net::TcpStream,
    tls_pair: Option<(Arc<rustls::ClientConfig>, TlsOptions)>,
    service_name: &str,
    authority: &str,
) -> std::result::Result<(h2::RecvStream, h2::SendStream<bytes::Bytes>), io::Error> {
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

    let uri = format!("{scheme}://{authority}/{service_name}/Tun");
    let request = http::Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .header("grpc-accept-encoding", "identity")
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
            "gRPC status: {}",
            response.status()
        )));
    }

    Ok((response.into_body(), send_stream))
}

async fn stream_loop(
    mut body: h2::RecvStream,
    mut send_stream: h2::SendStream<bytes::Bytes>,
    read_tx: mpsc::Sender<Vec<u8>>,
    tokio_write_rx: &mut tokio::sync::mpsc::Receiver<Vec<u8>>,
) {
    let mut reader = wrongsv_grpc::GrpcFrameReader::new();
    loop {
        tokio::select! {
            result = body.data() => {
                match result {
                    Some(Ok(data)) => {
                        let len = data.len();
                        let _ = body.flow_control().release_capacity(len);
                        if data.is_empty() {
                            continue;
                        }
                        let mut first = true;
                        loop {
                            let feed_result = if first {
                                first = false;
                                reader.feed(&data)
                            } else {
                                reader.feed(&[])
                            };
                            match feed_result {
                                Ok(Some(payload)) => {
                                    if !payload.is_empty() && read_tx.send(payload).is_err() {
                                        return;
                                    }
                                }
                                Ok(None) => break,
                                Err(_) => {
                                    let _ = read_tx.send(Vec::new());
                                    return;
                                }
                            }
                        }
                    }
                    Some(Err(_)) | None => {
                        let _ = read_tx.send(Vec::new());
                        return;
                    }
                }
            }
            maybe_data = tokio_write_rx.recv() => {
                match maybe_data {
                    Some(data) => {
                        let frame = wrongsv_grpc::encode_hunk_frame(&data);
                        if send_stream.send_data(frame, false).is_err() {
                            return;
                        }
                    }
                    None => {
                        let _ = send_stream.send_data(bytes::Bytes::new(), true);
                        return;
                    }
                }
            }
        }
    }
}

struct GrpcTunnel {
    read_rx: Receiver<Vec<u8>>,
    write_tx: SyncSender<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct GrpcReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct GrpcWriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for GrpcTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Read for GrpcReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for GrpcTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "gRPC write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for GrpcWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "gRPC write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for GrpcWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for GrpcTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "gRPC tunnel cannot be cloned (single h2 stream)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let GrpcTunnel {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(GrpcReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(GrpcWriteHalf { write_tx }),
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
