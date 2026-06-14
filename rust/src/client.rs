use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::time::Duration;

use base64::Engine as _;
use rand::RngCore;
use serde::Serialize;

use crate::config::{Protocol, ServerConfig};
use crate::error::{ClientError, Result};
use crate::protocol::{encode_raw_vless_header, read_raw_vless_response, Target};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub trait Tunnel: Read + Write + Send {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>>;
    fn shutdown_write(&mut self) -> io::Result<()>;
}

impl Tunnel for TcpStream {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(self.try_clone()?))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        self.shutdown(Shutdown::Write)
    }
}

#[derive(Clone, Debug)]
pub struct WrongsvClient {
    server: ServerConfig,
}

pub type RawVlessTcpClient = WrongsvClient;

impl WrongsvClient {
    pub fn new(server: ServerConfig) -> Result<Self> {
        match server.protocol {
            Protocol::RawVlessTcp | Protocol::VlessWebsocket | Protocol::VlessHttpupgrade => {
                Ok(Self { server })
            }
        }
    }

    pub fn connect(&self, target: &Target) -> Result<Box<dyn Tunnel>> {
        match self.server.protocol {
            Protocol::RawVlessTcp => self.connect_raw(target),
            Protocol::VlessHttpupgrade => self.connect_httpupgrade(target),
            Protocol::VlessWebsocket => self.connect_websocket(target),
        }
    }

    pub fn probe(&self, target: &Target, payload: &str) -> Result<ProbeResult> {
        let payload = if payload.is_empty() {
            format!(
                "HEAD / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                target.host
            )
        } else {
            payload.to_string()
        };

        let mut tunnel = self.connect(target)?;
        tunnel.write_all(payload.as_bytes())?;
        tunnel.flush().ok();

        let mut buf = [0u8; 2048];
        let n = tunnel.read(&mut buf)?;
        if n == 0 {
            return Err(ClientError::Config(
                "probe target closed without response".into(),
            ));
        }

        Ok(ProbeResult {
            bytes_read: n,
            preview: String::from_utf8_lossy(&buf[..n]).to_string(),
        })
    }

    fn connect_raw(&self, target: &Target) -> Result<Box<dyn Tunnel>> {
        let mut stream = self.connect_tcp_with_timeouts()?;
        let header = encode_raw_vless_header(&self.server.uuid, target)?;
        stream.write_all(&header)?;
        read_raw_vless_response(&mut stream)?;
        clear_timeouts(&stream)?;
        Ok(Box::new(stream))
    }

    fn connect_httpupgrade(&self, target: &Target) -> Result<Box<dyn Tunnel>> {
        let mut stream = self.connect_tcp_with_timeouts()?;
        let path = normalized_path(self.server.path.as_deref(), "/");
        http_upgrade_handshake(
            &mut stream,
            &path,
            host_header(&self.server, self.server.port),
        )?;

        let header = encode_raw_vless_header(&self.server.uuid, target)?;
        stream.write_all(&header)?;
        read_raw_vless_response(&mut stream)?;
        clear_timeouts(&stream)?;
        Ok(Box::new(stream))
    }

    fn connect_websocket(&self, target: &Target) -> Result<Box<dyn Tunnel>> {
        let mut stream = self.connect_tcp_with_timeouts()?;
        let path = normalized_path(self.server.path.as_deref(), "/");
        websocket_handshake(
            &mut stream,
            &path,
            host_header(&self.server, self.server.port),
        )?;

        let header = encode_raw_vless_header(&self.server.uuid, target)?;
        write_ws_frame(&mut stream, &header, OpCode::Binary, true)?;
        let mut ws = WebSocketTunnel::new(stream);
        read_raw_vless_response(&mut ws)?;
        ws.clear_timeouts()?;
        Ok(Box::new(ws))
    }

    fn connect_tcp_with_timeouts(&self) -> Result<TcpStream> {
        let stream = connect_tcp(&self.server.host, self.server.port)?;
        stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
        stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;
        Ok(stream)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProbeResult {
    pub bytes_read: usize,
    pub preview: String,
}

pub(crate) fn connect_tcp(host: &str, port: u16) -> Result<TcpStream> {
    let addrs = (host, port).to_socket_addrs().map_err(|e| {
        ClientError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("resolve {host}:{port}: {e}"),
        ))
    })?;

    let mut last_error = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(e) => last_error = Some(e),
        }
    }

    Err(ClientError::Io(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no addresses resolved for {host}:{port}"),
        )
    })))
}

fn clear_timeouts(stream: &TcpStream) -> io::Result<()> {
    stream.set_read_timeout(None)?;
    stream.set_write_timeout(None)?;
    Ok(())
}

fn normalized_path(value: Option<&str>, default: &str) -> String {
    let raw = value.unwrap_or(default).trim();
    if raw.is_empty() {
        return default.to_string();
    }
    if raw.starts_with('/') {
        raw.to_string()
    } else {
        format!("/{raw}")
    }
}

fn host_header(server: &ServerConfig, default_port: u16) -> String {
    server
        .host_header
        .clone()
        .unwrap_or_else(|| format!("{}:{default_port}", server.host))
}

fn read_http_headers(stream: &mut impl Read, context: &str) -> io::Result<String> {
    let mut buf = vec![0u8; 4096];
    let mut total = 0usize;
    loop {
        match stream.read(&mut buf[total..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("{context}: connection closed"),
                ));
            }
            Ok(n) => total += n,
            Err(e) => return Err(e),
        }
        if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
            return Ok(String::from_utf8_lossy(&buf[..total]).to_string());
        }
        if total == buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{context}: response headers too large"),
            ));
        }
    }
}

fn http_upgrade_handshake(stream: &mut TcpStream, path: &str, host: String) -> Result<()> {
    let req = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Upgrade: websocket\r\n\
         Connection: keep-alive, Upgrade\r\n\
         \r\n"
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;

    let response = read_http_headers(stream, "HTTPUpgrade")?;
    if !response.starts_with("HTTP/1.1 101 ") {
        return Err(ClientError::Io(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("expected HTTP 101, got: {response}"),
        )));
    }
    Ok(())
}

fn websocket_handshake(stream: &mut TcpStream, path: &str, host: String) -> Result<()> {
    let mut random_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut random_bytes);
    let key = base64::engine::general_purpose::STANDARD.encode(random_bytes);
    let req = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\n\
         Sec-WebSocket-Version: 13\r\n\
         \r\n"
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;

    let response = read_http_headers(stream, "WebSocket")?;
    if !response.starts_with("HTTP/1.1 101 ") {
        return Err(ClientError::Io(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("expected WebSocket HTTP 101, got: {response}"),
        )));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpCode {
    Close = 0x08,
    Ping = 0x09,
    Pong = 0x0a,
    Binary = 0x02,
}

struct WebSocketTunnel {
    inner: TcpStream,
    read_buf: Vec<u8>,
}

impl WebSocketTunnel {
    fn new(inner: TcpStream) -> Self {
        Self {
            inner,
            read_buf: Vec::new(),
        }
    }

    fn clear_timeouts(&self) -> io::Result<()> {
        clear_timeouts(&self.inner)
    }
}

impl Tunnel for WebSocketTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(Self {
            inner: self.inner.try_clone()?,
            read_buf: Vec::new(),
        }))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        write_ws_frame(&mut self.inner, &[], OpCode::Close, true)
    }
}

impl Read for WebSocketTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.read_buf.is_empty() {
            let n = self.read_buf.len().min(buf.len());
            buf[..n].copy_from_slice(&self.read_buf[..n]);
            self.read_buf.drain(..n);
            return Ok(n);
        }

        loop {
            let (opcode, payload) = read_ws_frame(&mut self.inner)?;
            match opcode {
                OpCode::Binary => {
                    let n = payload.len().min(buf.len());
                    buf[..n].copy_from_slice(&payload[..n]);
                    if n < payload.len() {
                        self.read_buf.extend_from_slice(&payload[n..]);
                    }
                    return Ok(n);
                }
                OpCode::Close => return Ok(0),
                OpCode::Ping => {
                    write_ws_frame(&mut self.inner, &payload, OpCode::Pong, true)?;
                }
                OpCode::Pong => {}
            }
        }
    }
}

impl Write for WebSocketTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_ws_frame(&mut self.inner, buf, OpCode::Binary, true)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn write_ws_frame(
    mut stream: impl Write,
    payload: &[u8],
    opcode: OpCode,
    masked: bool,
) -> io::Result<()> {
    let mut header = Vec::with_capacity(14);
    header.push(0x80 | opcode as u8);
    let mask_bit = if masked { 0x80 } else { 0x00 };
    match payload.len() {
        len if len < 126 => header.push(mask_bit | len as u8),
        len if len <= u16::MAX as usize => {
            header.push(mask_bit | 126);
            header.extend_from_slice(&(len as u16).to_be_bytes());
        }
        len => {
            header.push(mask_bit | 127);
            header.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }

    stream.write_all(&header)?;
    if masked {
        let mut mask = [0u8; 4];
        rand::thread_rng().fill_bytes(&mut mask);
        stream.write_all(&mask)?;
        for (idx, byte) in payload.iter().enumerate() {
            stream.write_all(&[*byte ^ mask[idx % 4]])?;
        }
    } else {
        stream.write_all(payload)?;
    }
    stream.flush()
}

fn read_ws_frame(stream: &mut impl Read) -> io::Result<(OpCode, Vec<u8>)> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;
    let opcode = match header[0] & 0x0f {
        0x02 => OpCode::Binary,
        0x08 => OpCode::Close,
        0x09 => OpCode::Ping,
        0x0a => OpCode::Pong,
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported WebSocket opcode {other:#x}"),
            ));
        }
    };
    let masked = header[1] & 0x80 != 0;
    let mut len = (header[1] & 0x7f) as u64;
    if len == 126 {
        let mut extended = [0u8; 2];
        stream.read_exact(&mut extended)?;
        len = u16::from_be_bytes(extended) as u64;
    } else if len == 127 {
        let mut extended = [0u8; 8];
        stream.read_exact(&mut extended)?;
        len = u64::from_be_bytes(extended);
    }

    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }

    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload)?;
    if masked {
        for (idx, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[idx % 4];
        }
    }
    Ok((opcode, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Protocol, ServerConfig};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

    #[test]
    fn probe_works_against_fake_raw_vless_server() {
        let server = spawn_fake_server(FakeCarrier::Raw);
        let client = WrongsvClient::new(ServerConfig {
            host: "127.0.0.1".into(),
            port: server.port,
            uuid: TEST_UUID.into(),
            protocol: Protocol::RawVlessTcp,
            path: None,
            host_header: None,
            flow: String::new(),
        })
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();

        assert_eq!(result.bytes_read, 4);
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn probe_works_against_fake_httpupgrade_server() {
        let server = spawn_fake_server(FakeCarrier::HttpUpgrade);
        let client = WrongsvClient::new(ServerConfig {
            host: "127.0.0.1".into(),
            port: server.port,
            uuid: TEST_UUID.into(),
            protocol: Protocol::VlessHttpupgrade,
            path: Some("/up".into()),
            host_header: None,
            flow: String::new(),
        })
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();

        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn probe_works_against_fake_websocket_server() {
        let server = spawn_fake_server(FakeCarrier::WebSocket);
        let client = WrongsvClient::new(ServerConfig {
            host: "127.0.0.1".into(),
            port: server.port,
            uuid: TEST_UUID.into(),
            protocol: Protocol::VlessWebsocket,
            path: Some("/ws".into()),
            host_header: None,
            flow: String::new(),
        })
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();

        assert_eq!(result.preview, "ping");
    }

    enum FakeCarrier {
        Raw,
        HttpUpgrade,
        WebSocket,
    }

    struct FakeServer {
        port: u16,
    }

    fn spawn_fake_server(carrier: FakeCarrier) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            ready_tx.send(()).unwrap();
            for stream in listener.incoming().flatten() {
                let _ = handle_fake_connection(stream, &carrier);
            }
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        FakeServer { port }
    }

    fn handle_fake_connection(mut stream: TcpStream, carrier: &FakeCarrier) -> io::Result<()> {
        match carrier {
            FakeCarrier::Raw => handle_fake_vless(stream),
            FakeCarrier::HttpUpgrade => {
                let _ = read_http_headers(&mut stream, "fake-httpupgrade")?;
                stream.write_all(
                    b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
                )?;
                handle_fake_vless(stream)
            }
            FakeCarrier::WebSocket => {
                let _ = read_http_headers(&mut stream, "fake-websocket")?;
                stream.write_all(
                    b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
                )?;
                let (_opcode, header) = read_ws_frame(&mut stream)?;
                let response = fake_vless_response(&header)?;
                write_ws_frame(&mut stream, &response, OpCode::Binary, false)?;
                loop {
                    let (_opcode, payload) = read_ws_frame(&mut stream)?;
                    write_ws_frame(&mut stream, &payload, OpCode::Binary, false)?;
                }
            }
        }
    }

    fn handle_fake_vless(mut stream: TcpStream) -> io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let mut fixed = [0u8; 19];
        stream.read_exact(&mut fixed)?;
        let addons_len = fixed[17] as usize;
        if addons_len > 0 {
            let mut addons = vec![0u8; addons_len];
            stream.read_exact(&mut addons)?;
        }

        let mut target = [0u8; 2];
        stream.read_exact(&mut target)?;
        let mut atyp = [0u8; 1];
        stream.read_exact(&mut atyp)?;
        read_fake_address(&mut stream, atyp[0])?;

        stream.write_all(&[0x00, 0x00])?;
        let mut buf = [0u8; 1024];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => return Ok(()),
                Ok(n) => stream.write_all(&buf[..n])?,
                Err(e) => return Err(e),
            }
        }
    }

    fn fake_vless_response(header: &[u8]) -> io::Result<Vec<u8>> {
        let mut cursor = io::Cursor::new(header);
        let mut fixed = [0u8; 19];
        cursor.read_exact(&mut fixed)?;
        let addons_len = fixed[17] as usize;
        if addons_len > 0 {
            let mut addons = vec![0u8; addons_len];
            cursor.read_exact(&mut addons)?;
        }
        let mut target = [0u8; 2];
        cursor.read_exact(&mut target)?;
        let mut atyp = [0u8; 1];
        cursor.read_exact(&mut atyp)?;
        read_fake_address(&mut cursor, atyp[0])?;
        Ok(vec![0x00, 0x00])
    }

    fn read_fake_address(reader: &mut impl Read, atyp: u8) -> io::Result<()> {
        match atyp {
            0x01 => {
                let mut addr = [0u8; 4];
                reader.read_exact(&mut addr)?;
            }
            0x02 => {
                let mut len = [0u8; 1];
                reader.read_exact(&mut len)?;
                let mut domain = vec![0u8; len[0] as usize];
                reader.read_exact(&mut domain)?;
            }
            0x03 => {
                let mut addr = [0u8; 16];
                reader.read_exact(&mut addr)?;
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected address type {other}"),
                ));
            }
        }
        Ok(())
    }
}
