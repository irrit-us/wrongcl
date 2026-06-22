use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use base64::Engine as _;
use x25519_dalek::{PublicKey, StaticSecret};

use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use wrongcl_native::endpoint::{
    Endpoint, OuterSecurity, ProxyProtocol, RealityOptions, Transport, VlessOptions,
};
use wrongcl_native::protocol::Target;
use wrongcl_native::proxy::{ProxyHandle, ProxySnapshot};

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";
const TEST_SHORT_ID: &str = "aaaaaaaa";

struct RealityServer {
    port: u16,
    public_key: String,
}

fn spawn_reality_server() -> RealityServer {
    let private_key = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let public_key = PublicKey::from(&private_key);
    let cert_material = wrongsv_reality::cert::build_cert_material().unwrap();
    let config = wrongsv_reality::RealityConfig::new(
        *private_key.as_bytes(),
        vec![hex_short_id(TEST_SHORT_ID)],
        30,
        cert_material,
        None,
    );

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = handle_reality_vless_echo(stream, &config);
        }
    });
    thread::sleep(Duration::from_millis(50));

    RealityServer {
        port,
        public_key: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key.as_bytes()),
    }
}

fn handle_reality_vless_echo(
    stream: TcpStream,
    config: &wrongsv_reality::RealityConfig,
) -> std::io::Result<()> {
    let mut tls = wrongsv_reality::accept_reality(stream, config)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    wrongsv_reality::complete_handshake(&mut tls)
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let mut fixed = [0u8; 19];
    tls.read_exact(&mut fixed)?;
    let addons_len = fixed[17] as usize;
    if addons_len > 0 {
        let mut addons = vec![0u8; addons_len];
        tls.read_exact(&mut addons)?;
    }
    let mut port = [0u8; 2];
    tls.read_exact(&mut port)?;
    let mut atyp = [0u8; 1];
    tls.read_exact(&mut atyp)?;
    consume_address(&mut tls, atyp[0])?;
    tls.write_all(&[0x00, 0x00])?;
    tls.flush()?;
    echo_loop(&mut tls)
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

fn echo_loop(stream: &mut impl ReadAndWrite) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                stream.write_all(&buf[..n])?;
                stream.flush()?;
            }
            Err(e) => return Err(e),
        }
    }
}

trait ReadAndWrite: Read + Write {}

impl<T: Read + Write> ReadAndWrite for T {}

fn hex_short_id(value: &str) -> [u8; 4] {
    let mut out = [0u8; 4];
    for i in 0..4 {
        out[i] = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).unwrap();
    }
    out
}

#[test]
fn probe_works_against_reality_server() {
    let server = spawn_reality_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::Reality(RealityOptions {
                server_name: "localhost".into(),
                public_key: server.public_key.clone(),
                short_id: TEST_SHORT_ID.into(),
                raw_pubkey: String::new(),
            }),
        },
    })
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping-reality")
        .expect("probe over REALITY");
    assert_eq!(result.preview, "ping-reality");
}

#[test]
fn socks_proxy_works_against_reality_server() {
    let server = spawn_reality_server();

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
                transport: Transport::Raw,
                outer_security: OuterSecurity::Reality(RealityOptions {
                    server_name: "localhost".into(),
                    public_key: server.public_key,
                    short_id: TEST_SHORT_ID.into(),
                    raw_pubkey: String::new(),
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

    assert_eq!(response, b"hello-reality".to_vec());
}

#[test]
fn socks_proxy_udp_works_against_reality_server() {
    let server = spawn_reality_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::Reality(RealityOptions {
                server_name: "localhost".into(),
                public_key: server.public_key,
                short_id: TEST_SHORT_ID.into(),
                raw_pubkey: String::new(),
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
    panic!("no UDP response from REALITY session");
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

    stream.write_all(b"hello-reality")?;
    let mut response = [0u8; 13];
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
