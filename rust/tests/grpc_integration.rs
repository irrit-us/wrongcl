use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bytes::Bytes;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig as RustlsServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use wrongcl_native::client::WrongsvClient;
use wrongcl_native::config::ServerConfig;
use wrongcl_native::endpoint::{
    Endpoint, GrpcOptions, OuterSecurity, ProxyProtocol, TlsOptions, Transport, VlessOptions,
};
use wrongcl_native::protocol::Target;

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

fn spawn_grpc_tls_server() -> u16 {
    let cert = generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der: PrivateKeyDer<'static> =
        PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()).into();

    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut server_config = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .expect("server cert");
    server_config.alpn_protocols = vec![b"h2".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    let (port_tx, port_rx) = std::sync::mpsc::channel::<u16>();

    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            port_tx.send(listener.local_addr().unwrap().port()).unwrap();
            loop {
                let (tcp, _) = match listener.accept().await {
                    Ok(v) => v,
                    Err(_) => return,
                };
                let acc = acceptor.clone();
                tokio::spawn(async move {
                    let _ = serve_one(acc, tcp).await;
                });
            }
        });
    });

    let port = port_rx.recv().expect("server port");
    thread::sleep(Duration::from_millis(50));
    port
}

async fn serve_one(acceptor: TlsAcceptor, tcp: tokio::net::TcpStream) -> std::io::Result<()> {
    let _ = tcp.set_nodelay(true);
    let tls = acceptor.accept(tcp).await.map_err(std::io::Error::other)?;
    let mut h2 = h2::server::handshake(tls)
        .await
        .map_err(std::io::Error::other)?;

    while let Some(req) = h2.accept().await {
        let (request, respond) = req.map_err(std::io::Error::other)?;
        tokio::spawn(async move {
            let _ = handle_stream(request, respond).await;
        });
    }
    Ok(())
}

async fn handle_stream(
    mut request: http::Request<h2::RecvStream>,
    mut respond: h2::server::SendResponse<Bytes>,
) -> std::io::Result<()> {
    assert_eq!(request.method(), http::Method::POST);
    assert_eq!(request.uri().path(), "/GunService/Tun");
    let response = http::Response::builder()
        .status(200)
        .header("content-type", "application/grpc")
        .body(())
        .unwrap();
    let mut send = respond
        .send_response(response, false)
        .map_err(std::io::Error::other)?;
    let recv = request.body_mut();

    let mut reader = wrongsv_grpc::GrpcFrameReader::new();
    let mut plain: Vec<u8> = Vec::new();
    let header_end = read_vless_header_grpc(recv, &mut reader, &mut plain).await?;

    let resp_frame = wrongsv_grpc::encode_hunk_frame(&[0x00, 0x00]);
    send.send_data(resp_frame, false)
        .map_err(std::io::Error::other)?;

    if plain.len() > header_end {
        let leftover = plain.split_off(header_end);
        let echo_frame = wrongsv_grpc::encode_hunk_frame(&leftover);
        send.send_data(echo_frame, false)
            .map_err(std::io::Error::other)?;
    }

    loop {
        let chunk = match recv.data().await {
            Some(Ok(c)) => c,
            Some(Err(e)) => return Err(std::io::Error::other(e)),
            None => break,
        };
        let len = chunk.len();
        let _ = recv.flow_control().release_capacity(len);
        if chunk.is_empty() {
            continue;
        }
        let mut first = true;
        loop {
            let res = if first {
                first = false;
                reader.feed(&chunk)
            } else {
                reader.feed(&[])
            };
            match res {
                Ok(Some(payload)) => {
                    if !payload.is_empty() {
                        let frame = wrongsv_grpc::encode_hunk_frame(&payload);
                        send.send_data(frame, false).map_err(std::io::Error::other)?;
                    }
                }
                Ok(None) => break,
                Err(e) => return Err(std::io::Error::other(format!("frame: {e}"))),
            }
        }
    }
    Ok(())
}

async fn read_vless_header_grpc(
    recv: &mut h2::RecvStream,
    reader: &mut wrongsv_grpc::GrpcFrameReader,
    plain: &mut Vec<u8>,
) -> std::io::Result<usize> {
    fill_plain_until(recv, reader, plain, 19).await?;
    let addons_len = plain[17] as usize;
    let after_cmd = 19 + addons_len;
    fill_plain_until(recv, reader, plain, after_cmd + 3).await?;
    let atyp = plain[after_cmd + 2];
    let header_end = match atyp {
        0x01 => after_cmd + 3 + 4,
        0x04 => after_cmd + 3 + 16,
        0x02 | 0x03 => {
            fill_plain_until(recv, reader, plain, after_cmd + 3 + 1).await?;
            let dlen = plain[after_cmd + 3] as usize;
            after_cmd + 3 + 1 + dlen
        }
        other => {
            return Err(std::io::Error::other(format!("bad atyp {other}")));
        }
    };
    fill_plain_until(recv, reader, plain, header_end).await?;
    Ok(header_end)
}

async fn fill_plain_until(
    recv: &mut h2::RecvStream,
    reader: &mut wrongsv_grpc::GrpcFrameReader,
    plain: &mut Vec<u8>,
    target: usize,
) -> std::io::Result<()> {
    while plain.len() < target {
        let mut first = true;
        loop {
            let res = if first {
                let chunk = recv
                    .data()
                    .await
                    .ok_or_else(|| std::io::Error::other("stream ended before header"))?
                    .map_err(std::io::Error::other)?;
                let len = chunk.len();
                let _ = recv.flow_control().release_capacity(len);
                if chunk.is_empty() {
                    return Err(std::io::Error::other("empty chunk before header complete"));
                }
                first = false;
                reader.feed(&chunk)
            } else {
                reader.feed(&[])
            };
            match res {
                Ok(Some(payload)) => {
                    plain.extend_from_slice(&payload);
                    if plain.len() >= target {
                        return Ok(());
                    }
                }
                Ok(None) => break,
                Err(e) => return Err(std::io::Error::other(format!("frame: {e}"))),
            }
        }
    }
    Ok(())
}

#[test]
fn probe_works_against_vless_over_grpc_over_tls_server() {
    let port = spawn_grpc_tls_server();

    let client = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: TEST_UUID.into(),
                flow: String::new(),
            }),
            transport: Transport::Grpc(GrpcOptions {
                service_name: "GunService".into(),
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
        .probe(&Target::new("example.com", 80).unwrap(), "ping-grpc-tls")
        .expect("probe over VLESS+gRPC+TLS");
    assert_eq!(result.preview, "ping-grpc-tls");
}
