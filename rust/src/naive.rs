use std::io;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use bytes::Bytes;
use http::{HeaderName, Method};

use crate::client::Tunnel;
use crate::endpoint::{NaiveOptions, TlsOptions};
use crate::error::{ClientError, Result};
use crate::tls;

mod padding;
mod tunnel;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_RETRY_TIMEOUT: Duration = Duration::from_secs(3);
const CONNECT_RETRY_INTERVAL: Duration = Duration::from_millis(100);
const NAIVE_PAD_OPS: usize = 8;
const NAIVE_PAD_MAX_PAYLOAD: usize = u16::MAX as usize;

use padding::{random_padding_header_value, NaivePadDecoder, NaivePadEncoder};
use tunnel::NaiveTunnel;

pub fn connect_naive(
    server_host: &str,
    server_port: u16,
    opts: &NaiveOptions,
    tls_opts: &TlsOptions,
    target_host: &str,
    target_port: u16,
) -> Result<Box<dyn Tunnel>> {
    let deadline = Instant::now() + CONNECT_RETRY_TIMEOUT;
    loop {
        match connect_naive_once(
            server_host,
            server_port,
            opts,
            tls_opts,
            target_host,
            target_port,
        ) {
            Ok(tunnel) => return Ok(tunnel),
            Err(ClientError::Io(error))
                if is_retryable_connect_error(&error) && Instant::now() < deadline =>
            {
                std::thread::sleep(CONNECT_RETRY_INTERVAL);
            }
            Err(error) => return Err(error),
        }
    }
}

pub fn connect_naive_once(
    server_host: &str,
    server_port: u16,
    opts: &NaiveOptions,
    tls_opts: &TlsOptions,
    target_host: &str,
    target_port: u16,
) -> Result<Box<dyn Tunnel>> {
    let addr = format!("{server_host}:{server_port}");
    let tls_opts = tls_opts.clone();
    let opts = opts.clone();
    let authority = connect_authority(target_host, target_port);

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
            Err(error) => {
                let _ = hs_tx.send(Err(io::Error::other(format!("tokio runtime: {error}"))));
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
                Ok(Ok(stream)) => stream,
                Ok(Err(error)) => {
                    let _ = hs_tx.send(Err(io::Error::other(format!("TCP connect: {error}"))));
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
                naive_handshake(tcp, tls_opts, opts, &authority),
            )
            .await;

            let (body, send_stream, use_padding) = match handshake {
                Ok(Ok(value)) => value,
                Ok(Err(error)) => {
                    let _ = hs_tx.send(Err(error));
                    return;
                }
                Err(_) => {
                    let _ = hs_tx.send(Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Naive handshake timed out",
                    )));
                    return;
                }
            };
            let _ = hs_tx.send(Ok(()));
            stream_loop(body, send_stream, use_padding, read_tx, &mut tokio_write_rx).await;
        });
    });

    hs_rx
        .recv()
        .map_err(|_| ClientError::Io(io::Error::other("Naive thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(NaiveTunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

fn is_retryable_connect_error(error: &io::Error) -> bool {
    if matches!(
        error.kind(),
        io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::NotConnected
            | io::ErrorKind::TimedOut
            | io::ErrorKind::UnexpectedEof
            | io::ErrorKind::WouldBlock
    ) {
        return true;
    }
    let message = error.to_string().to_ascii_lowercase();
    [
        "broken pipe",
        "connection refused",
        "connection reset",
        "handshake eof",
        "peer closed",
        "resource temporarily unavailable",
        "timed out",
    ]
    .iter()
    .any(|fragment| message.contains(fragment))
}

async fn naive_handshake(
    tcp: tokio::net::TcpStream,
    tls_opts: TlsOptions,
    opts: NaiveOptions,
    authority: &str,
) -> std::result::Result<(h2::RecvStream, h2::SendStream<Bytes>, bool), io::Error> {
    let mut tls_config =
        tls::build_client_config(&tls_opts).map_err(|error| io::Error::other(error.to_string()))?;
    if !tls_config
        .alpn_protocols
        .iter()
        .any(|value| value.as_slice() == b"h2")
    {
        tls_config.alpn_protocols.push(b"h2".to_vec());
    }

    let server_name = rustls::pki_types::ServerName::try_from(tls_opts.server_name.clone())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid SNI"))?;
    let connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config));
    let tls_stream = connector
        .connect(server_name, tcp)
        .await
        .map_err(|error| io::Error::other(format!("TLS: {error}")))?;

    let (client, conn) = h2::client::Builder::new()
        .initial_window_size(1_048_576)
        .handshake(tls_stream)
        .await
        .map_err(|error| io::Error::other(format!("h2: {error}")))?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let mut client = client
        .ready()
        .await
        .map_err(|error| io::Error::other(format!("h2 ready: {error}")))?;

    let uri = http::Uri::builder()
        .authority(authority)
        .build()
        .map_err(|error| io::Error::other(format!("CONNECT uri: {error}")))?;
    let auth_value = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", opts.username, opts.password));
    let padding_header_name =
        HeaderName::from_bytes(opts.padding_header_name.as_bytes()).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid padding header name: {error}"),
            )
        })?;

    let request = http::Request::builder()
        .method(Method::CONNECT)
        .uri(uri)
        .header(
            http::header::PROXY_AUTHORIZATION,
            format!("Basic {auth_value}"),
        )
        .header(padding_header_name.clone(), random_padding_header_value())
        .body(())
        .map_err(|error| io::Error::other(format!("h2 request build: {error}")))?;

    let (response, send_stream) = client
        .send_request(request, false)
        .map_err(|error| io::Error::other(format!("send: {error}")))?;
    let response = response
        .await
        .map_err(|error| io::Error::other(format!("response: {error}")))?;
    if response.status() != http::StatusCode::OK {
        return Err(io::Error::other(format!(
            "Naive status: {}",
            response.status()
        )));
    }
    let use_padding = response.headers().contains_key(&padding_header_name);
    Ok((response.into_body(), send_stream, use_padding))
}

async fn stream_loop(
    mut body: h2::RecvStream,
    mut send_stream: h2::SendStream<Bytes>,
    use_padding: bool,
    read_tx: mpsc::Sender<Vec<u8>>,
    tokio_write_rx: &mut tokio::sync::mpsc::Receiver<Vec<u8>>,
) {
    let mut decoder = NaivePadDecoder::new();
    let mut encoder = NaivePadEncoder::new();
    let mut decoded = Vec::with_capacity(8192);

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
                        if use_padding {
                            decoded.clear();
                            decoder.feed_into(&data, &mut decoded);
                            if !decoded.is_empty() && read_tx.send(decoded.clone()).is_err() {
                                break;
                            }
                        } else if read_tx.send(data.to_vec()).is_err() {
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
                        let frame = if use_padding {
                            Bytes::from(encoder.encode(&data))
                        } else {
                            Bytes::from(data)
                        };
                        if send_stream.send_data(frame, false).is_err() {
                            break;
                        }
                    }
                    None => {
                        let _ = send_stream.send_data(Bytes::new(), true);
                        break;
                    }
                }
            }
        }
    }
}

fn connect_authority(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_reset_message_is_retryable() {
        let error = io::Error::other("response: connection reset");
        assert!(is_retryable_connect_error(&error));
    }
}
