use super::*;

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

#[test]
fn socks_proxy_relays_and_tracks_metrics() {
    let server = spawn_fake_vless_server();
    let config =
        ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
    let mut proxy = ProxyHandle::start(config).unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr()).unwrap();
    let snapshot = wait_for_inactive(&proxy);
    proxy.stop().unwrap();

    assert_eq!(response, b"hello".to_vec());
    assert_eq!(snapshot.active_connections, 0);
    assert_eq!(snapshot.total_connections, 1);
    assert_eq!(snapshot.failed_connections, 0);
    assert!(snapshot.bytes_uploaded >= 5);
    assert!(snapshot.bytes_downloaded >= 5);
}

#[test]
fn http_connect_proxy_relays_and_tracks_metrics() {
    let server = spawn_fake_vless_server();
    let config =
        ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
    let mut proxy = ProxyHandle::start(config).unwrap();

    let response = run_http_connect_echo(proxy.snapshot().socket_addr()).unwrap();
    let snapshot = wait_for_inactive(&proxy);
    proxy.stop().unwrap();

    assert_eq!(response, b"hello".to_vec());
    assert_eq!(snapshot.active_connections, 0);
    assert_eq!(snapshot.failed_connections, 0);
    assert!(snapshot.bytes_uploaded >= 5);
    assert!(snapshot.bytes_downloaded >= 5);
}

#[test]
fn socks_handshake_survives_nonblocking_client_socket() {
    let server = spawn_fake_vless_server();
    let config =
        ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let shared = ProxyShared {
            stop: AtomicBool::new(false),
            metrics: ProxyMetrics::new(),
        };
        let (stream, _) = listener.accept().unwrap();
        stream.set_nonblocking(true).unwrap();
        handle_socks_client(stream, config, &shared)
    });

    let response = run_socks_echo(local_addr).unwrap();
    let result = worker.join().unwrap();

    assert_eq!(response, b"hello".to_vec());
    result.unwrap();
}

#[test]
fn socks_proxy_relays_through_remote_http_connect_backend() {
    let backend = spawn_fake_http_connect_backend(None, None);
    let config = ClientConfig {
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port: backend.port,
            endpoint: Endpoint {
                proxy: ProxyProtocol::Mixed(crate::endpoint::MixedOptions {
                    username: None,
                    password: None,
                }),
                transport: Transport::Raw,
                outer_security: OuterSecurity::None,
            },
        },
        local: LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
        },
    };
    let mut proxy = ProxyHandle::start(config).unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello".to_vec());
}

#[test]
fn http_proxy_rejects_non_connect_requests() {
    let server = spawn_fake_vless_server();
    let config =
        ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
    let mut proxy = ProxyHandle::start(config).unwrap();

    let response = run_http_get_rejected(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert!(response.starts_with("HTTP/1.1 405 Method Not Allowed"));
}

#[test]
fn http_proxy_forwards_absolute_form_requests() {
    let server = spawn_fake_vless_server();
    let config =
        ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
    let mut proxy = ProxyHandle::start(config).unwrap();

    let echoed = run_http_absolute_form(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    let text = String::from_utf8_lossy(&echoed);
    assert!(text.starts_with("GET /hello?x=1 HTTP/1.1\r\n"), "{text}");
    assert!(text.contains("\r\nHost: example.com\r\n"), "{text}");
    assert!(
        !text.to_ascii_lowercase().contains("proxy-connection"),
        "{text}"
    );
}

#[test]
fn socks_proxy_relays_udp_associate() {
    let server = spawn_fake_vless_udp_server();
    let config =
        ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
    let mut proxy = ProxyHandle::start(config).unwrap();

    let response = run_socks_udp_echo(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"ping-udp".to_vec());
}

#[test]
fn socks_proxy_relays_shadowsocks_udp() {
    let server =
        spawn_fake_shadowsocks_udp_server("chacha20-ietf-poly1305".into(), "hunter2".into());
    let mut proxy = ProxyHandle::start(shadowsocks_client_config(
        server.port,
        "chacha20-ietf-poly1305",
        "hunter2",
    ))
    .unwrap();

    let response = run_socks_udp_echo(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"ping-udp".to_vec());
}

#[test]
fn socks_proxy_relays_shadowsocks_aead_2022_udp() {
    let psk_b64 = "AAAAAAAAAAAAAAAAAAAAAA==";
    let server =
        spawn_fake_shadowsocks_udp_server("2022-blake3-aes-128-gcm".into(), psk_b64.into());
    let mut proxy = ProxyHandle::start(shadowsocks_client_config(
        server.port,
        "2022-blake3-aes-128-gcm",
        psk_b64,
    ))
    .unwrap();

    let response = run_socks_udp_echo(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"ping-udp".to_vec());
}
