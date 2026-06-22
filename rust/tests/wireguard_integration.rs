use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use base64::Engine as _;
use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use wrongcl_native::endpoint::{
    Endpoint, OuterSecurity, ProxyProtocol, Transport, WireGuardOptions,
};
use wrongcl_native::protocol::Target;
use wrongcl_native::proxy::{ProxyHandle, ProxySnapshot};
use wrongsv_server::{Config as WrongsvServerConfig, InboundServer, ShutdownSignal};
use x25519_dalek::{PublicKey, StaticSecret};

struct WireGuardServer {
    port: u16,
    _shutdown: ShutdownSignal,
    _handle: wrongsv_server::ServerHandle,
}

fn wireguard_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn spawn_wireguard_server(server_private_key: &str, client_public_key: &str) -> WireGuardServer {
    let port = free_udp_port();
    let config: WrongsvServerConfig = toml::from_str(&format!(
        r#"
listen = "127.0.0.1:{port}"

[wireguard]
private_key = "{server_private_key}"
mtu = 1400
server_cidrs = ["10.77.0.1/32"]
outbound = true

[[wireguard.peers]]
public_key = "{client_public_key}"
allowed_ips = ["10.77.0.2/32"]
"#,
        port = port,
        server_private_key = server_private_key,
        client_public_key = client_public_key,
    ))
    .unwrap();
    let shutdown = ShutdownSignal::new();
    let handle = InboundServer::new(config)
        .unwrap()
        .spawn_with_shutdown(shutdown.clone());
    thread::sleep(Duration::from_millis(250));
    WireGuardServer {
        port,
        _shutdown: shutdown,
        _handle: handle,
    }
}

fn wireguard_client_config(
    server_port: u16,
    client_private_key: &str,
    server_public_key: &str,
) -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".into(),
        port: server_port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Wireguard(WireGuardOptions {
                private_key: client_private_key.into(),
                peer_public_key: server_public_key.into(),
                pre_shared_key: None,
                client_ip: "10.77.0.2/32".into(),
                allowed_ips: vec!["0.0.0.0/0".into()],
                mtu: 1400,
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::None,
        },
    }
}

fn derive_public_key_b64(private_key: [u8; 32]) -> String {
    let secret = StaticSecret::from(private_key);
    let public = PublicKey::from(&secret);
    base64::engine::general_purpose::STANDARD.encode(public.as_bytes())
}

fn encode_private_key_b64(private_key: [u8; 32]) -> String {
    base64::engine::general_purpose::STANDARD.encode(private_key)
}

fn free_udp_port() -> u16 {
    UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn routable_host_ip() -> std::net::IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("8.8.8.8:53").unwrap();
    socket.local_addr().unwrap().ip()
}

fn spawn_tcp_echo_server() -> SocketAddr {
    let listener = TcpListener::bind((routable_host_ip(), 0)).unwrap();
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
    let socket = UdpSocket::bind((routable_host_ip(), 0)).unwrap();
    let addr = socket.local_addr().unwrap();
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok((n, peer)) = socket.recv_from(&mut buf) {
            if socket.send_to(&buf[..n], peer).is_err() {
                break;
            }
        }
    });
    addr
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "WireGuard runtime is currently verified on Linux"
)]
fn probe_works_against_wireguard_server() {
    let _guard = wireguard_test_lock().lock().unwrap();
    let server_private = [7u8; 32];
    let client_private = [9u8; 32];
    let server_private_b64 = encode_private_key_b64(server_private);
    let server_public_b64 = derive_public_key_b64(server_private);
    let client_private_b64 = encode_private_key_b64(client_private);
    let client_public_b64 = derive_public_key_b64(client_private);

    let server = spawn_wireguard_server(&server_private_b64, &client_public_b64);
    let echo_addr = spawn_tcp_echo_server();

    let client = WrongsvClient::new(wireguard_client_config(
        server.port,
        &client_private_b64,
        &server_public_b64,
    ))
    .unwrap();
    let result = client
        .probe(
            &Target::new(echo_addr.ip().to_string(), echo_addr.port()).unwrap(),
            "ping-wireguard",
        )
        .expect("probe over WireGuard");
    assert_eq!(result.preview, "ping-wireguard");
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "WireGuard runtime is currently verified on Linux"
)]
fn socks_proxy_works_against_wireguard_server() {
    let _guard = wireguard_test_lock().lock().unwrap();
    let server_private = [7u8; 32];
    let client_private = [9u8; 32];
    let server_private_b64 = encode_private_key_b64(server_private);
    let server_public_b64 = derive_public_key_b64(server_private);
    let client_private_b64 = encode_private_key_b64(client_private);
    let client_public_b64 = derive_public_key_b64(client_private);

    let server = spawn_wireguard_server(&server_private_b64, &client_public_b64);
    let echo_addr = spawn_tcp_echo_server();

    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        wireguard_client_config(server.port, &client_private_b64, &server_public_b64),
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_echo(
        proxy.snapshot().socket_addr(),
        echo_addr,
        b"hello-wireguard",
    )
    .unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-wireguard".to_vec());
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "WireGuard runtime is currently verified on Linux"
)]
fn socks_proxy_udp_works_against_wireguard_server() {
    let _guard = wireguard_test_lock().lock().unwrap();
    let server_private = [7u8; 32];
    let client_private = [9u8; 32];
    let server_private_b64 = encode_private_key_b64(server_private);
    let server_public_b64 = derive_public_key_b64(server_private);
    let client_private_b64 = encode_private_key_b64(client_private);
    let client_public_b64 = derive_public_key_b64(client_private);

    let server = spawn_wireguard_server(&server_private_b64, &client_public_b64);
    let echo_addr = spawn_udp_echo_server();

    let client = WrongsvClient::new(wireguard_client_config(
        server.port,
        &client_private_b64,
        &server_public_b64,
    ))
    .unwrap();
    let mut session = client
        .connect_udp_session(&Target::new(echo_addr.ip().to_string(), echo_addr.port()).unwrap())
        .unwrap();
    session.send_packet(b"ping-wireguard-udp").unwrap();
    for _ in 0..200 {
        if let Some(packet) = session.try_recv_packet().unwrap() {
            assert_eq!(packet.payload, b"ping-wireguard-udp");
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("no UDP response from WireGuard session");
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
