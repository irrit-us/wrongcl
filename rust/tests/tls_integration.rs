use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig as RustlsServerConfig, ServerConnection, StreamOwned};

use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::ServerConfig;
use wrongcl_native::endpoint::{
    Endpoint, OuterSecurity, ProxyProtocol, TlsOptions, Transport, TrojanOptions, VlessOptions,
};
use wrongcl_native::protocol::Target;

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

struct TlsServer {
    port: u16,
}

fn spawn_tls_server<F>(handler: F) -> TlsServer
where
    F: Send + Sync + 'static + Fn(&mut StreamOwned<ServerConnection, TcpStream>) -> std::io::Result<()>,
{
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
    let handler = Arc::new(handler);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let server_config = Arc::clone(&config);
            let handler = Arc::clone(&handler);
            thread::spawn(move || {
                let conn = match ServerConnection::new(server_config) {
                    Ok(conn) => conn,
                    Err(_) => return,
                };
                let mut tls = StreamOwned::new(conn, stream);
                let _ = handler(&mut tls);
            });
        }
    });
    thread::sleep(Duration::from_millis(50));
    TlsServer { port }
}

#[test]
fn probe_works_against_vless_over_tls_server() {
    let server = spawn_tls_server(handle_vless_echo);

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::Tls(TlsOptions {
                server_name: "localhost".into(),
                insecure_skip_verify: true,
                alpn: vec![],
            }),
        },
    })
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .expect("probe over TLS");
    assert_eq!(result.preview, "ping");
}

#[test]
fn probe_works_against_trojan_over_tls_server() {
    let server = spawn_tls_server(handle_trojan_echo);

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Trojan(TrojanOptions {
                password: "hunter2".into(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::Tls(TlsOptions {
                server_name: "localhost".into(),
                insecure_skip_verify: true,
                alpn: vec![],
            }),
        },
    })
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .expect("probe over Trojan");
    assert_eq!(result.preview, "ping");
}

fn handle_vless_echo(
    tls: &mut StreamOwned<ServerConnection, TcpStream>,
) -> std::io::Result<()> {
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
    consume_address(tls, atyp[0])?;
    tls.write_all(&[0x00, 0x00])?;
    tls.flush()?;
    echo_loop(tls)
}

fn handle_trojan_echo(
    tls: &mut StreamOwned<ServerConnection, TcpStream>,
) -> std::io::Result<()> {
    let mut hash = [0u8; 56];
    tls.read_exact(&mut hash)?;
    let mut crlf = [0u8; 2];
    tls.read_exact(&mut crlf)?;
    assert_eq!(&crlf, b"\r\n");
    let mut cmd_atyp = [0u8; 2];
    tls.read_exact(&mut cmd_atyp)?;
    assert_eq!(cmd_atyp[0], 0x01);
    consume_address(tls, cmd_atyp[1])?;
    let mut port = [0u8; 2];
    tls.read_exact(&mut port)?;
    tls.read_exact(&mut crlf)?;
    assert_eq!(&crlf, b"\r\n");
    echo_loop(tls)
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

fn echo_loop(
    tls: &mut StreamOwned<ServerConnection, TcpStream>,
) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        match tls.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                tls.write_all(&buf[..n])?;
                tls.flush()?;
            }
            Err(e) => return Err(e),
        }
    }
}
