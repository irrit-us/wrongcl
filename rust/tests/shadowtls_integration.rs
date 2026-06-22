use std::io::{Cursor, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Digest, Sha256};

use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use wrongcl_native::endpoint::{
    Endpoint, OuterSecurity, ProxyProtocol, ShadowTlsOptions, Transport, VlessOptions,
};
use wrongcl_native::protocol::Target;
use wrongcl_native::proxy::{ProxyHandle, ProxySnapshot};

type HmacSha1 = Hmac<Sha1>;

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";
const TEST_PASSWORD: &str = "shadow-pass";
const TLS_HEADER_SIZE: usize = 5;
const TLS_RANDOM_SIZE: usize = 32;
const TLS_SESSION_ID_SIZE: usize = 32;
const HANDSHAKE: u8 = 22;
const APPLICATION_DATA: u8 = 23;
const CLIENT_HELLO: u8 = 1;
const SERVER_HELLO: u8 = 2;
const HMAC_SIZE: usize = 4;
const TLS_HMAC_HEADER_SIZE: usize = TLS_HEADER_SIZE + HMAC_SIZE;
const SERVER_RANDOM_INDEX: usize = TLS_HEADER_SIZE + 1 + 3 + 2;
const SESSION_ID_LENGTH_INDEX: usize = TLS_HEADER_SIZE + 1 + 3 + 2 + TLS_RANDOM_SIZE;

struct ShadowTlsServer {
    port: u16,
}

fn spawn_shadowtls_server() -> ShadowTlsServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let _ = handle_shadowtls_echo(stream);
            });
        }
    });
    thread::sleep(Duration::from_millis(50));
    ShadowTlsServer { port }
}

fn handle_shadowtls_echo(mut stream: TcpStream) -> std::io::Result<()> {
    let client_hello = extract_tls_frame(&mut stream)?;
    verify_client_hello(&client_hello, TEST_PASSWORD)?;

    let server_random = [9u8; TLS_RANDOM_SIZE];
    stream.write_all(&make_server_hello(server_random))?;

    // Emit one cover-handshake frame so the client must skip the relay phase.
    let mut handshake_state = seed_shadowtls_hmac(TEST_PASSWORD, &server_random, b"")?;
    let handshake_frame =
        encode_handshake_relay_frame(&mut handshake_state, &server_random, b"cover-handshake")?;
    stream.write_all(&handshake_frame)?;

    let mut client_state = seed_shadowtls_hmac(TEST_PASSWORD, &server_random, b"C")?;
    let mut server_state = seed_shadowtls_hmac(TEST_PASSWORD, &server_random, b"S")?;

    let first_payload = decode_client_frame(&extract_tls_frame(&mut stream)?, &mut client_state)?;
    validate_vless_header(&first_payload)?;

    let response = encode_shadowtls_application_data(&mut server_state, &[0x00, 0x00])?;
    stream.write_all(&response)?;
    stream.flush()?;

    echo_shadowtls_payloads(&mut stream, &mut client_state, &mut server_state)
}

fn echo_shadowtls_payloads(
    stream: &mut TcpStream,
    client_state: &mut HmacSha1,
    server_state: &mut HmacSha1,
) -> std::io::Result<()> {
    loop {
        let frame = match extract_tls_frame(stream) {
            Ok(frame) => frame,
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(err) => return Err(err),
        };
        let payload = decode_client_frame(&frame, client_state)?;
        let response = encode_shadowtls_application_data(server_state, &payload)?;
        stream.write_all(&response)?;
        stream.flush()?;
    }
}

fn validate_vless_header(payload: &[u8]) -> std::io::Result<()> {
    let mut cursor = Cursor::new(payload);
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
        other => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unexpected VLESS atyp {other}"),
            ));
        }
    }
    Ok(())
}

fn extract_tls_frame(reader: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut header = [0u8; TLS_HEADER_SIZE];
    reader.read_exact(&mut header)?;
    let len = u16::from_be_bytes([header[3], header[4]]) as usize;
    let mut frame = Vec::with_capacity(TLS_HEADER_SIZE + len);
    frame.extend_from_slice(&header);
    frame.resize(TLS_HEADER_SIZE + len, 0);
    reader.read_exact(&mut frame[TLS_HEADER_SIZE..])?;
    Ok(frame)
}

fn verify_client_hello(frame: &[u8], password: &str) -> std::io::Result<()> {
    let min_len = TLS_HEADER_SIZE + 1 + 3 + 2 + TLS_RANDOM_SIZE + 1 + TLS_SESSION_ID_SIZE;
    let hmac_index = SESSION_ID_LENGTH_INDEX + 1 + TLS_SESSION_ID_SIZE - HMAC_SIZE;
    if frame.len() < min_len {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "shadowtls client hello too short",
        ));
    }
    if frame[0] != HANDSHAKE || frame[TLS_HEADER_SIZE] != CLIENT_HELLO {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "shadowtls expected TLS ClientHello",
        ));
    }
    if frame[SESSION_ID_LENGTH_INDEX] != TLS_SESSION_ID_SIZE as u8 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "shadowtls unexpected session id length",
        ));
    }

    let mut hmac = new_shadowtls_hmac(password)?;
    hmac.update(&frame[TLS_HEADER_SIZE..hmac_index]);
    hmac.update(&[0, 0, 0, 0]);
    hmac.update(&frame[hmac_index + HMAC_SIZE..]);
    let expected = hmac.finalize().into_bytes();
    if frame[hmac_index..hmac_index + HMAC_SIZE] != expected[..HMAC_SIZE] {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "shadowtls client hello hmac mismatch",
        ));
    }
    Ok(())
}

fn make_server_hello(server_random: [u8; TLS_RANDOM_SIZE]) -> Vec<u8> {
    let body_len = 2 + TLS_RANDOM_SIZE;
    let mut frame = vec![0u8; TLS_HEADER_SIZE + 1 + 3 + body_len];
    frame[0] = HANDSHAKE;
    frame[1] = 3;
    frame[2] = 3;
    let record_len = frame.len() - TLS_HEADER_SIZE;
    frame[3..5].copy_from_slice(&(record_len as u16).to_be_bytes());
    frame[TLS_HEADER_SIZE] = SERVER_HELLO;
    let handshake_len = body_len;
    frame[TLS_HEADER_SIZE + 1] = ((handshake_len >> 16) & 0xff) as u8;
    frame[TLS_HEADER_SIZE + 2] = ((handshake_len >> 8) & 0xff) as u8;
    frame[TLS_HEADER_SIZE + 3] = (handshake_len & 0xff) as u8;
    frame[TLS_HEADER_SIZE + 4] = 3;
    frame[TLS_HEADER_SIZE + 5] = 3;
    frame[SERVER_RANDOM_INDEX..SERVER_RANDOM_INDEX + TLS_RANDOM_SIZE]
        .copy_from_slice(&server_random);
    frame
}

fn new_shadowtls_hmac(password: &str) -> std::io::Result<HmacSha1> {
    HmacSha1::new_from_slice(password.as_bytes())
        .map_err(|err| std::io::Error::other(format!("shadowtls hmac init: {err}")))
}

fn seed_shadowtls_hmac(
    password: &str,
    server_random: &[u8; TLS_RANDOM_SIZE],
    suffix: &[u8],
) -> std::io::Result<HmacSha1> {
    let mut hmac = new_shadowtls_hmac(password)?;
    hmac.update(server_random);
    hmac.update(suffix);
    Ok(hmac)
}

fn encode_shadowtls_application_data(
    state: &mut HmacSha1,
    payload: &[u8],
) -> std::io::Result<Vec<u8>> {
    let record_len = HMAC_SIZE + payload.len();
    if record_len > u16::MAX as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "shadowtls record too large",
        ));
    }
    state.update(payload);
    let hmac = state.clone().finalize().into_bytes();
    state.update(&hmac[..HMAC_SIZE]);

    let mut frame = Vec::with_capacity(TLS_HMAC_HEADER_SIZE + payload.len());
    frame.push(APPLICATION_DATA);
    frame.push(3);
    frame.push(3);
    frame.extend_from_slice(&(record_len as u16).to_be_bytes());
    frame.extend_from_slice(&hmac[..HMAC_SIZE]);
    frame.extend_from_slice(payload);
    Ok(frame)
}

fn decode_client_frame(frame: &[u8], state: &mut HmacSha1) -> std::io::Result<Vec<u8>> {
    if frame.len() < TLS_HMAC_HEADER_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "shadowtls frame too short",
        ));
    }
    if frame[0] != APPLICATION_DATA || frame[1] != 3 || frame[2] != 3 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "shadowtls expected application data frame",
        ));
    }
    let payload = &frame[TLS_HMAC_HEADER_SIZE..];
    state.update(payload);
    let expected = state.clone().finalize().into_bytes();
    if frame[TLS_HEADER_SIZE..TLS_HMAC_HEADER_SIZE] != expected[..HMAC_SIZE] {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "shadowtls frame hmac mismatch",
        ));
    }
    state.update(&frame[TLS_HEADER_SIZE..TLS_HMAC_HEADER_SIZE]);
    Ok(payload.to_vec())
}

fn encode_handshake_relay_frame(
    state: &mut HmacSha1,
    server_random: &[u8; TLS_RANDOM_SIZE],
    payload: &[u8],
) -> std::io::Result<Vec<u8>> {
    let mut encrypted = payload.to_vec();
    let key = shadowtls_kdf(TEST_PASSWORD, server_random);
    xor_slice(&mut encrypted, &key);
    state.update(&encrypted);
    let hmac = state.clone().finalize().into_bytes();
    let record_len = HMAC_SIZE + encrypted.len();
    let mut frame = Vec::with_capacity(TLS_HMAC_HEADER_SIZE + encrypted.len());
    frame.push(APPLICATION_DATA);
    frame.push(3);
    frame.push(3);
    frame.extend_from_slice(&(record_len as u16).to_be_bytes());
    frame.extend_from_slice(&hmac[..HMAC_SIZE]);
    frame.extend_from_slice(&encrypted);
    Ok(frame)
}

fn shadowtls_kdf(password: &str, server_random: &[u8; TLS_RANDOM_SIZE]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(server_random);
    hasher.finalize().into()
}

fn xor_slice(data: &mut [u8], key: &[u8]) {
    for (idx, byte) in data.iter_mut().enumerate() {
        *byte ^= key[idx % key.len()];
    }
}

#[test]
fn probe_works_against_shadowtls_server() {
    let server = spawn_shadowtls_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::ShadowTls(ShadowTlsOptions {
                server_name: "cloudfront.net".into(),
                password: TEST_PASSWORD.into(),
            }),
        },
    })
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping-shadowtls")
        .expect("probe over ShadowTLS");
    assert_eq!(result.preview, "ping-shadowtls");
}

#[test]
fn socks_proxy_works_against_shadowtls_server() {
    let server = spawn_shadowtls_server();

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
                outer_security: OuterSecurity::ShadowTls(ShadowTlsOptions {
                    server_name: "cloudfront.net".into(),
                    password: TEST_PASSWORD.into(),
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

    assert_eq!(response, b"hello-shadowtls".to_vec());
}

#[test]
fn socks_proxy_udp_works_against_shadowtls_server() {
    let server = spawn_shadowtls_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: server.port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::ShadowTls(ShadowTlsOptions {
                server_name: "cloudfront.net".into(),
                password: TEST_PASSWORD.into(),
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
    panic!("no UDP response from ShadowTLS session");
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

    stream.write_all(b"hello-shadowtls")?;
    let mut response = vec![0u8; "hello-shadowtls".len()];
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
