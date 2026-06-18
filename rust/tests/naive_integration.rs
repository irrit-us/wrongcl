use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use wrongcl_native::endpoint::{
    Endpoint, NaiveOptions, OuterSecurity, ProxyProtocol, TlsOptions, Transport,
};
use wrongcl_native::protocol::Target;
use wrongcl_native::proxy::{ProxyHandle, ProxySnapshot};
use wrongsv_server::{Config as WrongsvServerConfig, InboundServer, ShutdownSignal};

struct NaiveServer {
    port: u16,
    _shutdown: ShutdownSignal,
    _handle: wrongsv_server::ServerHandle,
}

fn spawn_naive_server() -> NaiveServer {
    let port = free_tcp_port();
    let config: WrongsvServerConfig = toml::from_str(&format!(
        r#"
listen = "127.0.0.1:{port}"

[naive]
padding_header_name = "Padding"

[naive.tls]

[[naive.users]]
username = "alice"
password = "secret"
email = "alice@example.com"
"#,
        port = port,
    ))
    .unwrap();
    let shutdown = ShutdownSignal::new();
    let handle = InboundServer::new(config)
        .unwrap()
        .spawn_with_shutdown(shutdown.clone());
    thread::sleep(Duration::from_millis(150));
    NaiveServer {
        port,
        _shutdown: shutdown,
        _handle: handle,
    }
}

fn naive_server_config(port: u16) -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".into(),
        port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Naive(NaiveOptions {
                username: "alice".into(),
                password: "secret".into(),
                padding_header_name: "Padding".into(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::Tls(TlsOptions {
                server_name: "naive.example".into(),
                insecure_skip_verify: true,
                alpn: vec!["h2".into()],
            }),
        },
    }
}

fn free_tcp_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
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

fn spawn_flaky_front_proxy(backend_port: u16) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let mut drop_first = true;
        for stream in listener.incoming().flatten() {
            if drop_first {
                drop_first = false;
                drop(stream);
                continue;
            }
            let backend = TcpStream::connect(("127.0.0.1", backend_port)).unwrap();
            let mut client_read = stream.try_clone().unwrap();
            let mut client_write = stream;
            let mut backend_read = backend.try_clone().unwrap();
            let mut backend_write = backend;
            thread::spawn(move || {
                let _ = std::io::copy(&mut client_read, &mut backend_write);
                let _ = backend_write.shutdown(Shutdown::Write);
            });
            thread::spawn(move || {
                let _ = std::io::copy(&mut backend_read, &mut client_write);
                let _ = client_write.shutdown(Shutdown::Write);
            });
        }
    });
    port
}

#[test]
fn probe_works_against_naive_server() {
    let server = spawn_naive_server();
    let echo_addr = spawn_tcp_echo_server();

    let client = WrongsvClient::new(naive_server_config(server.port)).unwrap();
    let result = client
        .probe(
            &Target::new(echo_addr.ip().to_string(), echo_addr.port()).unwrap(),
            "ping-naive",
        )
        .expect("probe over Naive");
    assert_eq!(result.preview, "ping-naive");
}

#[test]
fn socks_proxy_works_against_naive_server() {
    let server = spawn_naive_server();
    let echo_addr = spawn_tcp_echo_server();

    let mut proxy = ProxyHandle::start(ClientConfig {
        server: naive_server_config(server.port),
        local: LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
        },
    })
    .unwrap();

    let response =
        run_socks_echo(proxy.snapshot().socket_addr(), echo_addr, b"hello-naive").unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-naive".to_vec());
}

#[test]
fn probe_retries_after_initial_connection_drop() {
    let backend = spawn_naive_server();
    let front_port = spawn_flaky_front_proxy(backend.port);
    let echo_addr = spawn_tcp_echo_server();

    let client = WrongsvClient::new(naive_server_config(front_port)).unwrap();
    let result = client
        .probe(
            &Target::new(echo_addr.ip().to_string(), echo_addr.port()).unwrap(),
            "ping-naive-retry",
        )
        .expect("probe over Naive after transient connect drop");
    assert_eq!(result.preview, "ping-naive-retry");
}

#[test]
fn naive_profile_rejects_udp_session() {
    let server = spawn_naive_server();

    let client = WrongsvClient::new(naive_server_config(server.port)).unwrap();
    match client.connect_udp_session(&Target::new("example.com", 53).unwrap()) {
        Ok(_) => panic!("expected Naive to reject UDP session"),
        Err(err) => assert!(
            err.to_string().contains("does not support UDP"),
            "expected UDP unsupported error, got {err}"
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
