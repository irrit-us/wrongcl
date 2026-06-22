use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::thread;
use std::time::Duration;

use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use wrongcl_native::endpoint::{
    Endpoint, OuterSecurity, ProxyProtocol, QuicOptions, Transport, VlessOptions,
};
use wrongcl_native::protocol::Target;
use wrongcl_native::proxy::{ProxyHandle, ProxySnapshot};
use wrongsv_server::{Config as WrongsvServerConfig, InboundServer, ShutdownSignal};

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

struct QuicServer {
    port: u16,
    _shutdown: ShutdownSignal,
    _handle: wrongsv_server::ServerHandle,
}

fn spawn_quic_server(udp_enabled: bool) -> QuicServer {
    let port = free_udp_port();
    let config: WrongsvServerConfig = toml::from_str(&format!(
        r#"
listen = "127.0.0.1:{port}"

[[users]]
id = "{uuid}"

[quic]
udp_relay = {udp_enabled}
"#,
        port = port,
        uuid = TEST_UUID,
        udp_enabled = udp_enabled,
    ))
    .unwrap();
    let shutdown = ShutdownSignal::new();
    let handle = InboundServer::new(config)
        .unwrap()
        .spawn_with_shutdown(shutdown.clone());
    thread::sleep(Duration::from_millis(150));
    QuicServer {
        port,
        _shutdown: shutdown,
        _handle: handle,
    }
}

fn free_udp_port() -> u16 {
    UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn spawn_tcp_echo_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let mut stream: TcpStream = stream;
                let mut buf = [0u8; 4096];
                loop {
                    let n = match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    if stream.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            });
        }
    });
    addr
}

fn spawn_udp_echo_server() -> SocketAddr {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap();
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(value) = socket.recv_from(&mut buf) {
            let (n, peer) = value;
            if socket.send_to(&buf[..n], peer).is_err() {
                break;
            }
        }
    });
    addr
}

#[test]
fn probe_works_against_quic_server() {
    let server = spawn_quic_server(true);
    let echo_addr = spawn_tcp_echo_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Quic(QuicOptions {
                server_name: "cloudfront.net".into(),
                udp_enabled: true,
            }),
            outer_security: OuterSecurity::None,
        },
    })
    .unwrap();

    let result = client
        .probe(
            &Target::new(echo_addr.ip().to_string(), echo_addr.port()).unwrap(),
            "ping-quic",
        )
        .expect("probe over QUIC");
    assert_eq!(result.preview, "ping-quic");
}

#[test]
fn socks_proxy_works_against_quic_server() {
    let server = spawn_quic_server(true);
    let echo_addr = spawn_tcp_echo_server();

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
                transport: Transport::Quic(QuicOptions {
                    server_name: "cloudfront.net".into(),
                    udp_enabled: true,
                }),
                outer_security: OuterSecurity::None,
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

    let response =
        run_socks_echo(proxy.snapshot().socket_addr(), echo_addr, b"hello-quic").unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-quic".to_vec());
}

#[test]
fn socks_proxy_udp_works_against_quic_server() {
    let server = spawn_quic_server(true);
    let echo_addr = spawn_udp_echo_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Quic(QuicOptions {
                server_name: "cloudfront.net".into(),
                udp_enabled: true,
            }),
            outer_security: OuterSecurity::None,
        },
    })
    .unwrap();

    let mut session = client
        .connect_udp_session(&Target::new(echo_addr.ip().to_string(), echo_addr.port()).unwrap())
        .unwrap();
    session.send_packet(b"ping-udp").unwrap();
    for _ in 0..40 {
        if let Some(packet) = session.try_recv_packet().unwrap() {
            assert_eq!(packet.payload, b"ping-udp");
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("no UDP response from QUIC session");
}

#[test]
fn quic_udp_disabled_profile_rejects_udp_session() {
    let server = spawn_quic_server(false);

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Quic(QuicOptions {
                server_name: "cloudfront.net".into(),
                udp_enabled: false,
            }),
            outer_security: OuterSecurity::None,
        },
    })
    .unwrap();

    match client.connect_udp_session(&Target::new("example.com", 53).unwrap()) {
        Ok(_) => panic!("expected QUIC UDP-disabled profile to reject UDP session"),
        Err(err) => assert!(
            err.to_string().contains("disabled"),
            "expected UDP-disabled error, got {err}"
        ),
    }
}

fn run_socks_echo(
    local_addr: SocketAddr,
    target_addr: SocketAddr,
    payload: &[u8],
) -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(&[0x05, 0x01, 0x00])?;

    let mut greeting = [0u8; 2];
    stream.read_exact(&mut greeting)?;
    assert_eq!(greeting, [0x05, 0x00]);

    let host = target_addr.ip().to_string();
    let host = host.as_bytes();
    let mut request = vec![0x05, 0x01, 0x00, 0x03, host.len() as u8];
    request.extend_from_slice(host);
    request.extend_from_slice(&target_addr.port().to_be_bytes());
    stream.write_all(&request)?;

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply)?;
    assert_eq!(reply[1], 0x00);

    stream.write_all(payload)?;
    let mut response = vec![0u8; payload.len()];
    stream.read_exact(&mut response)?;
    Ok(response)
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
