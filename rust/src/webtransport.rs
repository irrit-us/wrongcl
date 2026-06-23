use std::io::{self, Read, Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::pin::Pin;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;

use wtransport::Endpoint as WtEndpoint;
use wtransport::config::{DnsLookupFuture, DnsResolver};

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::endpoint::{TlsOptions, WebTransportOptions};
use crate::error::{ClientError, Result};
use crate::protocol::{Target, encode_raw_vless_header, encode_udp_vless_header};
use crate::tls;

pub fn connect_webtransport(
    server_host: &str,
    server_port: u16,
    opts: &WebTransportOptions,
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
            match webtransport_handshake(&server_host, server_port, &opts, &header).await {
                Ok((_endpoint, _connection, mut send, mut recv)) => {
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
                                Ok(None) | Err(_) => {
                                    let _ = read_tx.send(Vec::new());
                                    break;
                                }
                            }
                        }
                    });

                    while let Some(data) = tokio_write_rx.recv().await {
                        if send.write_all(&data).await.is_err() {
                            break;
                        }
                    }

                    let _ = send.finish().await;
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
        .map_err(|_| ClientError::Io(io::Error::other("WebTransport thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(WebTransportTunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

struct WebTransportTunnel {
    read_rx: Receiver<Vec<u8>>,
    write_tx: SyncSender<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct WebTransportReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

#[derive(Clone)]
struct WebTransportWriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for WebTransportTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for WebTransportTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx.send(buf.to_vec()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "WebTransport write channel closed",
            )
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for WebTransportReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for WebTransportWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx.send(buf.to_vec()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "WebTransport write channel closed",
            )
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for WebTransportWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for WebTransportTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "WebTransport tunnel cannot be cloned (single WebTransport stream)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let WebTransportTunnel {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(WebTransportReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(WebTransportWriteHalf { write_tx }),
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

#[derive(Clone, Debug)]
struct StaticDnsResolver {
    socket_addr: SocketAddr,
}

impl DnsResolver for StaticDnsResolver {
    fn resolve(&self, _host: &str) -> Pin<Box<dyn DnsLookupFuture>> {
        let socket_addr = self.socket_addr;
        Box::pin(async move { Ok(Some(socket_addr)) })
    }
}

async fn webtransport_handshake(
    server_host: &str,
    server_port: u16,
    opts: &WebTransportOptions,
    vless_header: &[u8],
) -> io::Result<(
    WtEndpoint<wtransport::endpoint::endpoint_side::Client>,
    wtransport::Connection,
    wtransport::SendStream,
    wtransport::RecvStream,
)> {
    let server_addr = resolve_server_addr(server_host, server_port)?;
    let url = build_connect_url(&opts.authority, server_port, &opts.path)?;
    let tls_config = make_webtransport_tls_config(&opts.authority)?;
    let wt_client_config = wtransport::ClientConfig::builder()
        .with_bind_default()
        .with_custom_tls(tls_config)
        .dns_resolver(StaticDnsResolver {
            socket_addr: server_addr,
        })
        .build();
    let endpoint = WtEndpoint::client(wt_client_config)
        .map_err(|err| io::Error::other(format!("WebTransport endpoint: {err}")))?;
    let connection = endpoint.connect(url.as_str()).await.map_err(|err| {
        io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("WebTransport connect: {err}"),
        )
    })?;

    let opening = connection.open_bi().await.map_err(|err| {
        io::Error::other(format!("open WebTransport bidirectional stream: {err}"))
    })?;
    let (mut send, mut recv) = opening.await.map_err(|err| {
        io::Error::other(format!("open WebTransport bidirectional stream: {err}"))
    })?;
    send.write_all(vless_header)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;

    let mut resp = [0u8; 2];
    recv.read_exact(&mut resp)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    if resp[0] != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid VLESS response version: {}", resp[0]),
        ));
    }
    if resp[1] > 0 {
        let mut addons = vec![0u8; resp[1] as usize];
        recv.read_exact(&mut addons)
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    }

    Ok((endpoint, connection, send, recv))
}

fn make_webtransport_tls_config(authority: &str) -> io::Result<rustls::ClientConfig> {
    let tls_opts = TlsOptions {
        server_name: authority_host(authority)?,
        insecure_skip_verify: true,
        alpn: vec!["h3".into()],
    };
    tls::build_client_config(&tls_opts).map_err(|err| io::Error::other(err.to_string()))
}

fn authority_host(authority: &str) -> io::Result<String> {
    authority
        .trim()
        .parse::<http::uri::Authority>()
        .map(|authority| authority.host().to_string())
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid WebTransport authority: {err}"),
            )
        })
}

fn build_connect_url(authority: &str, server_port: u16, path: &str) -> io::Result<String> {
    let authority = authority.trim();
    if authority.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "WebTransport authority is required",
        ));
    }
    let parsed = authority.parse::<http::uri::Authority>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid WebTransport authority: {err}"),
        )
    })?;
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };
    let authority = if parsed.port_u16().is_some() || server_port == 443 {
        authority.to_string()
    } else {
        format!("{authority}:{server_port}")
    };
    Ok(format!("https://{authority}{path}"))
}

fn resolve_server_addr(server_host: &str, server_port: u16) -> io::Result<SocketAddr> {
    (server_host, server_port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "no WebTransport server addresses resolved",
            )
        })
}
