use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use uuid::Uuid;
use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use wrongcl_native::endpoint::{Endpoint, OuterSecurity, ProxyProtocol, Transport, VlessOptions};
use wrongcl_native::protocol::Target;
use wrongcl_native::proxy::ProxyHandle;
use wrongsv_vless::vision::{TrafficState, VisionReader, VisionWriter};

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

struct VisionServer {
    port: u16,
}

fn spawn_vision_echo_server() -> VisionServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handler = Arc::new(handle_vless_vision_echo);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let handler = Arc::clone(&handler);
            thread::spawn(move || {
                let _ = handler(stream);
            });
        }
    });
    thread::sleep(Duration::from_millis(50));
    VisionServer { port }
}

fn handle_vless_vision_echo(mut stream: TcpStream) -> std::io::Result<()> {
    let mut fixed = [0u8; 17];
    stream.read_exact(&mut fixed)?;
    let uuid_bytes: [u8; 16] = fixed[1..17].try_into().unwrap();
    let mut addons_len = [0u8; 1];
    stream.read_exact(&mut addons_len)?;
    let mut flow_str = String::new();
    if addons_len[0] > 0 {
        let mut addons = vec![0u8; addons_len[0] as usize];
        stream.read_exact(&mut addons)?;
        assert_eq!(addons[0], 0x0a);
        let flow_len = addons[1] as usize;
        flow_str = String::from_utf8(addons[2..2 + flow_len].to_vec()).unwrap();
    }
    assert_eq!(flow_str, "xtls-rprx-vision", "expected Vision flow");

    let mut cmd = [0u8; 1];
    stream.read_exact(&mut cmd)?;
    let mut port = [0u8; 2];
    stream.read_exact(&mut port)?;
    let mut atyp = [0u8; 1];
    stream.read_exact(&mut atyp)?;
    consume_address(&mut stream, atyp[0])?;

    stream.write_all(&[0x00, 0x00])?;
    stream.flush()?;

    let read_half = stream.try_clone()?;
    let write_half = stream;

    let up_state = TrafficState::new(&uuid_bytes);
    let down_state = TrafficState::new(&uuid_bytes);

    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let echo_in = thread::spawn(move || {
        let mut reader = VisionReader::new(read_half, up_state, true);
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => return,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    });

    let echo_out = thread::spawn(move || {
        let mut writer = VisionWriter::new(write_half, down_state, false, vec![900, 500, 900, 256]);
        while let Ok(chunk) = rx.recv() {
            if writer.write(&chunk).is_err() {
                return;
            }
            if writer.flush().is_err() {
                return;
            }
        }
    });

    let _ = echo_in.join();
    let _ = echo_out.join();
    Ok(())
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
fn probe_works_against_vless_vision_echo_server() {
    let _ = Uuid::parse_str(TEST_UUID).unwrap();
    let server = spawn_vision_echo_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: "xtls-rprx-vision".into(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::None,
        },
    })
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping-vision")
        .expect("probe over VLESS+Vision");
    assert_eq!(result.preview, "ping-vision");
}

#[test]
fn socks_proxy_works_against_vless_vision_echo_server() {
    let _ = Uuid::parse_str(TEST_UUID).unwrap();
    let server = spawn_vision_echo_server();

    let mut proxy = ProxyHandle::start(ClientConfig {
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port: server.port,
            endpoint: Endpoint {
                proxy: ProxyProtocol::Vless(VlessOptions {
                    uuid: TEST_UUID.into(),
                    flow: "xtls-rprx-vision".into(),
                }),
                transport: Transport::Raw,
                outer_security: OuterSecurity::None,
            },
        },
        local: LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
        },
    })
    .unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-vision".to_vec());
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

    stream.write_all(b"hello-vision")?;
    let mut response = [0u8; 12];
    stream.read_exact(&mut response)?;
    Ok(response.to_vec())
}

trait SnapshotAddr {
    fn socket_addr(&self) -> SocketAddr;
}

impl SnapshotAddr for wrongcl_native::proxy::ProxySnapshot {
    fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.local_host, self.local_port)
            .parse()
            .unwrap()
    }
}
