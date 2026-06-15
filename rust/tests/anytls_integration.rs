use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig as RustlsServerConfig, ServerConnection, StreamOwned};
use sha2::{Digest, Sha256};

use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::ServerConfig;
use wrongcl_native::endpoint::{
    AnyTlsOptions, Endpoint, OuterSecurity, ProxyProtocol, Transport, VlessOptions,
};
use wrongcl_native::protocol::Target;

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";
const TEST_PASSWORD: &str = "hunter2";

struct AnyTlsServer {
    port: u16,
}

fn spawn_anytls_server() -> AnyTlsServer {
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
                let _ = handle_anytls_vless_echo(&mut tls);
            });
        }
    });
    thread::sleep(Duration::from_millis(50));
    AnyTlsServer { port }
}

fn handle_anytls_vless_echo(
    tls: &mut StreamOwned<ServerConnection, TcpStream>,
) -> std::io::Result<()> {
    let expected_hash: [u8; 32] = Sha256::digest(TEST_PASSWORD.as_bytes()).into();
    let mut auth = [0u8; 34];
    tls.read_exact(&mut auth)?;
    assert_eq!(&auth[..32], &expected_hash, "SHA256(password) mismatch");
    let padding_len = u16::from_be_bytes([auth[32], auth[33]]) as usize;
    if padding_len > 0 {
        let mut padding = vec![0u8; padding_len];
        tls.read_exact(&mut padding)?;
    }

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
fn probe_works_against_anytls_server() {
    let server = spawn_anytls_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::AnyTls(AnyTlsOptions {
                server_name: "localhost".into(),
                password: TEST_PASSWORD.into(),
                insecure_skip_verify: true,
                alpn: vec![],
            }),
        },
    })
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping-anytls")
        .expect("probe over AnyTLS");
    assert_eq!(result.preview, "ping-anytls");
}
