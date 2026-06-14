use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::Serialize;

use crate::config::{Protocol, ServerConfig};
use crate::error::{ClientError, Result};
use crate::protocol::{encode_raw_vless_header, read_raw_vless_response, Target};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug)]
pub struct RawVlessTcpClient {
    server: ServerConfig,
}

impl RawVlessTcpClient {
    pub fn new(server: ServerConfig) -> Result<Self> {
        if server.protocol != Protocol::RawVlessTcp {
            return Err(ClientError::UnsupportedProtocol(
                server.protocol.as_str().into(),
            ));
        }
        Ok(Self { server })
    }

    pub fn connect(&self, target: &Target) -> Result<TcpStream> {
        let mut stream = connect_tcp(&self.server.host, self.server.port)?;
        stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
        stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;

        let header = encode_raw_vless_header(&self.server.uuid, target)?;
        stream.write_all(&header)?;
        read_raw_vless_response(&mut stream)?;

        stream.set_read_timeout(None)?;
        stream.set_write_timeout(None)?;
        Ok(stream)
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
        tunnel.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
        tunnel.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProbeResult {
    pub bytes_read: usize,
    pub preview: String,
}

pub(crate) fn connect_tcp(host: &str, port: u16) -> Result<TcpStream> {
    let addrs = (host, port).to_socket_addrs().map_err(|e| {
        ClientError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
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
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no addresses resolved for {host}:{port}"),
        )
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Protocol, ServerConfig};
    use std::io;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

    #[test]
    fn probe_works_against_fake_vless_server() {
        let server = spawn_fake_vless_server();
        let client = RawVlessTcpClient::new(ServerConfig {
            host: "127.0.0.1".into(),
            port: server.port,
            uuid: TEST_UUID.into(),
            protocol: Protocol::RawVlessTcp,
        })
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();

        assert_eq!(result.bytes_read, 4);
        assert_eq!(result.preview, "ping");
    }

    struct FakeServer {
        port: u16,
    }

    fn spawn_fake_vless_server() -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            ready_tx.send(()).unwrap();
            for stream in listener.incoming().flatten() {
                thread::spawn(move || {
                    let _ = handle_fake_vless(stream);
                });
            }
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        FakeServer { port }
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

        let mut port = [0u8; 2];
        stream.read_exact(&mut port)?;
        let mut atyp = [0u8; 1];
        stream.read_exact(&mut atyp)?;
        match atyp[0] {
            0x01 => {
                let mut addr = [0u8; 4];
                stream.read_exact(&mut addr)?;
            }
            0x02 => {
                let mut len = [0u8; 1];
                stream.read_exact(&mut len)?;
                let mut domain = vec![0u8; len[0] as usize];
                stream.read_exact(&mut domain)?;
            }
            0x03 => {
                let mut addr = [0u8; 16];
                stream.read_exact(&mut addr)?;
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected address type {other}"),
                ));
            }
        }

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
}
