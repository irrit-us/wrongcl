use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig as RustlsServerConfig, ServerConnection, StreamOwned};

use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use wrongcl_native::endpoint::{
    Endpoint, OuterSecurity, ProxyProtocol, TlsOptions, Transport, VlessOptions, WsOptions,
};
use wrongcl_native::protocol::Target;
use wrongcl_native::proxy::{ProxyHandle, ProxySnapshot};

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

struct WsTlsServer {
    port: u16,
}

fn spawn_ws_tls_server() -> WsTlsServer {
    let cert = generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der: PrivateKeyDer<'static> =
        PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()).into();

    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .expect("server cert");
    let config = Arc::new(config);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let server_config = Arc::clone(&config);
            thread::spawn(move || {
                let conn = match ServerConnection::new(server_config) {
                    Ok(conn) => conn,
                    Err(_) => return,
                };
                let mut tls = StreamOwned::new(conn, stream);
                let _ = handle_ws_vless_echo(&mut tls);
            });
        }
    });
    thread::sleep(Duration::from_millis(50));
    WsTlsServer { port }
}

fn handle_ws_vless_echo(tls: &mut StreamOwned<ServerConnection, TcpStream>) -> std::io::Result<()> {
    read_http_request(tls)?;
    tls.write_all(
        b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
    )?;
    tls.flush()?;

    let header = read_ws_frame(tls)?;
    let mut cursor = std::io::Cursor::new(header.as_slice());
    let mut fixed = [0u8; 19];
    cursor.read_exact(&mut fixed)?;
    let addons_len = fixed[17] as usize;
    if addons_len > 0 {
        let mut addons = vec![0u8; addons_len];
        cursor.read_exact(&mut addons)?;
    }
    let mut port = [0u8; 2];
    cursor.read_exact(&mut port)?;
    let mut atyp = [0u8; 1];
    cursor.read_exact(&mut atyp)?;
    consume_address(&mut cursor, atyp[0])?;
    write_ws_frame(tls, &[0x00, 0x00])?;

    loop {
        let payload = match read_ws_frame(tls) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };
        write_ws_frame(tls, &payload)?;
    }
}

fn read_http_request<S: Read>(stream: &mut S) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte)?;
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            return Ok(());
        }
        if buf.len() > 8 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "WS upgrade headers too large",
            ));
        }
    }
}

fn read_ws_frame<S: Read>(stream: &mut S) -> std::io::Result<Vec<u8>> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;
    let opcode = header[0] & 0x0f;
    assert_eq!(opcode, 0x02, "expected Binary WS opcode");
    let masked = header[1] & 0x80 != 0;
    let mut len = (header[1] & 0x7f) as u64;
    if len == 126 {
        let mut ext = [0u8; 2];
        stream.read_exact(&mut ext)?;
        len = u16::from_be_bytes(ext) as u64;
    } else if len == 127 {
        let mut ext = [0u8; 8];
        stream.read_exact(&mut ext)?;
        len = u64::from_be_bytes(ext);
    }
    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }
    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload)?;
    if masked {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
    }
    Ok(payload)
}

fn write_ws_frame<S: Write>(stream: &mut S, payload: &[u8]) -> std::io::Result<()> {
    let mut header = Vec::with_capacity(10);
    header.push(0x80 | 0x02);
    match payload.len() {
        len if len < 126 => header.push(len as u8),
        len if len <= u16::MAX as usize => {
            header.push(126);
            header.extend_from_slice(&(len as u16).to_be_bytes());
        }
        len => {
            header.push(127);
            header.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }
    stream.write_all(&header)?;
    stream.write_all(payload)?;
    stream.flush()
}

fn consume_address<R: Read>(reader: &mut R, atyp: u8) -> std::io::Result<()> {
    match atyp {
        0x01 => {
            let mut addr = [0u8; 4];
            reader.read_exact(&mut addr)?;
        }
        0x02 | 0x03 => {
            let mut len = [0u8; 1];
            reader.read_exact(&mut len)?;
            let mut domain = vec![0u8; len[0] as usize];
            reader.read_exact(&mut domain)?;
        }
        0x04 => {
            let mut addr = [0u8; 16];
            reader.read_exact(&mut addr)?;
        }
        other => panic!("unexpected atyp {other}"),
    }
    Ok(())
}

#[test]
fn probe_works_against_vless_over_websocket_over_tls_server() {
    let server = spawn_ws_tls_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Websocket(WsOptions {
                path: "/ws".into(),
                host: None,
            }),
            outer_security: OuterSecurity::Tls(TlsOptions {
                server_name: "localhost".into(),
                insecure_skip_verify: true,
                alpn: vec![],
            }),
        },
    })
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping-ws-tls")
        .expect("probe over VLESS+WS+TLS");
    assert_eq!(result.preview, "ping-ws-tls");
}

#[test]
fn socks_proxy_works_against_vless_over_websocket_over_tls_server() {
    let server = spawn_ws_tls_server();

    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        ServerConfig {
            host: "127.0.0.1".into(),
            port: server.port,
            endpoint: Endpoint {
                proxy: ProxyProtocol::Vless(VlessOptions {
                    uuid: TEST_UUID.into(),
                    flow: String::new(),
                }),
                transport: Transport::Websocket(WsOptions {
                    path: "/ws".into(),
                    host: None,
                }),
                outer_security: OuterSecurity::Tls(TlsOptions {
                    server_name: "localhost".into(),
                    insecure_skip_verify: true,
                    alpn: vec![],
                }),
            },
        },
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-ws-tls".to_vec());
}

#[test]
fn socks_proxy_udp_works_against_vless_over_websocket_over_tls_server() {
    let server = spawn_ws_tls_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Websocket(WsOptions {
                path: "/ws".into(),
                host: None,
            }),
            outer_security: OuterSecurity::Tls(TlsOptions {
                server_name: "localhost".into(),
                insecure_skip_verify: true,
                alpn: vec![],
            }),
        },
    })
    .unwrap();

    let mut session = client
        .connect_udp_session(&Target::new("example.com", 53).unwrap())
        .unwrap();
    session.send_packet(b"ping-udp").unwrap();
    for _ in 0..20 {
        if let Some(packet) = session.try_recv_packet().unwrap() {
            assert_eq!(packet.payload, b"ping-udp");
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("no UDP response from WS+TLS session");
}

fn run_socks_echo(local_addr: SocketAddr) -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(&[0x05, 0x01, 0x00])?;

    let mut greeting = [0u8; 2];
    stream.read_exact(&mut greeting)?;
    assert_eq!(greeting, [0x05, 0x00]);

    let host = b"example.com";
    let mut request = vec![0x05, 0x01, 0x00, 0x03, host.len() as u8];
    request.extend_from_slice(host);
    request.extend_from_slice(&80u16.to_be_bytes());
    stream.write_all(&request)?;

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply)?;
    assert_eq!(reply[1], 0x00);

    stream.write_all(b"hello-ws-tls")?;
    let mut response = [0u8; 12];
    stream.read_exact(&mut response)?;
    Ok(response.to_vec())
}

trait SnapshotAddr {
    fn socket_addr(&self) -> SocketAddr;
}

impl SnapshotAddr for ProxySnapshot {
    fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.local_host, self.local_port)
            .parse()
            .unwrap()
    }
}
