use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::client::{Tunnel, TunnelReader, TunnelWriter, connect_tcp};
use crate::endpoint::{MeekOptions, OuterSecurity};
use crate::error::{ClientError, Result};
use crate::protocol::{
    Target, encode_raw_vless_header, encode_udp_vless_header, read_raw_vless_response,
};
use crate::tls;

const ROUNDTRIP_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(25);

#[allow(clippy::too_many_arguments)]
pub fn connect_meek(
    server_host: &str,
    server_port: u16,
    opts: &MeekOptions,
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
    let session_id = random_session_id();
    let mut initial_response = meek_roundtrip(
        server_host,
        server_port,
        opts,
        outer_security,
        &session_id,
        &header,
    )?;
    let deadline = Instant::now() + ROUNDTRIP_TIMEOUT;
    while initial_response.is_empty() && Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);
        initial_response = meek_roundtrip(
            server_host,
            server_port,
            opts,
            outer_security,
            &session_id,
            &[],
        )?;
    }
    if initial_response.is_empty() {
        return Err(ClientError::Io(io::Error::new(
            io::ErrorKind::TimedOut,
            "Meek VLESS response timeout",
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

            match meek_roundtrip(
                &server_host,
                server_port,
                &opts,
                &outer_security,
                &session_id,
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

    Ok(Box::new(MeekTunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

struct MeekTunnel {
    read_rx: Receiver<Vec<u8>>,
    write_tx: SyncSender<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

struct MeekReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

#[derive(Clone)]
struct MeekWriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for MeekTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for MeekTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Meek write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for MeekReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for MeekWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Meek write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for MeekWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for MeekTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Meek tunnel cannot be cloned (single request session)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let MeekTunnel {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(MeekReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(MeekWriteHalf { write_tx }),
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

fn meek_roundtrip(
    server_host: &str,
    server_port: u16,
    opts: &MeekOptions,
    outer_security: &OuterSecurity,
    session_id: &str,
    body: &[u8],
) -> Result<Vec<u8>> {
    let mut stream = open_meek_connection(server_host, server_port, outer_security)?;
    let path = normalized_path(&opts.path, "/");
    let host = host_header(opts.host.as_deref(), server_host, server_port);
    let request = build_http_post(&path, &host, session_id, body);
    stream.write_all(&request)?;
    stream.flush()?;
    read_http_response(stream.as_mut()).map_err(ClientError::Io)
}

fn open_meek_connection(
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
            "Meek only supports none or TLS outer security, got {}",
            other.id()
        ))),
    }
}

fn build_http_post(path: &str, host: &str, session_id: &str, body: &[u8]) -> Vec<u8> {
    let mut request = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         X-Session-ID: {session_id}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    request
}

fn read_http_response(stream: &mut dyn Read) -> io::Result<Vec<u8>> {
    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    let mut header_end = None;
    let mut content_length = None;

    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);
        if header_end.is_none() {
            if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
                header_end = Some(index + 4);
                let headers = std::str::from_utf8(&response[..index]).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP response")
                })?;
                let mut lines = headers.lines();
                let status = lines.next().unwrap_or_default();
                if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unexpected Meek HTTP status: {status}"),
                    ));
                }
                for line in lines {
                    if let Some((key, value)) = line.split_once(':') {
                        if key.trim().eq_ignore_ascii_case("content-length") {
                            content_length = Some(value.trim().parse::<usize>().map_err(|_| {
                                io::Error::new(io::ErrorKind::InvalidData, "invalid content-length")
                            })?);
                        }
                    }
                }
            }
        }
        if let (Some(header_end), Some(content_length)) = (header_end, content_length) {
            if response.len() >= header_end + content_length {
                break;
            }
        }
    }

    let header_end = header_end
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "short HTTP response"))?;
    let body = if let Some(content_length) = content_length {
        response[header_end..header_end + content_length.min(response.len() - header_end)].to_vec()
    } else {
        response[header_end..].to_vec()
    };
    Ok(body)
}

fn normalized_path(value: &str, default: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() {
        return default.to_string();
    }
    if raw.starts_with('/') {
        raw.to_string()
    } else {
        format!("/{raw}")
    }
}

fn host_header(explicit: Option<&str>, server_host: &str, server_port: u16) -> String {
    explicit
        .map(|value| value.to_string())
        .unwrap_or_else(|| format!("{server_host}:{server_port}"))
}

fn random_session_id() -> String {
    let bytes: [u8; 16] = rand::random();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
