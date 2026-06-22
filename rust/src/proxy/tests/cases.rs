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
        let shared = Arc::new(ProxyShared {
            stop: AtomicBool::new(false),
            started_at_unix: 0,
            registry: Arc::new(ConnRegistry::new()),
            config: RwLock::new(config),
        });
        let (stream, peer) = listener.accept().unwrap();
        stream.set_nonblocking(true).unwrap();
        let conn = shared.registry.register(peer, None);
        handle_socks_client(stream, &shared, &conn)
    });

    let response = run_socks_echo(local_addr).unwrap();
    let result = worker.join().unwrap();

    assert_eq!(response, b"hello".to_vec());
    result.unwrap();
}

#[test]
fn socks_proxy_relays_through_remote_http_connect_backend() {
    let backend = spawn_fake_http_connect_backend(None, None);
    let config = ClientConfig::single_server(
        "default",
        ServerConfig {
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
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    );
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
fn socks_proxy_can_be_disabled_per_listener() {
    let server = spawn_fake_vless_server();
    let config = ClientConfig::single_server(
        "default",
        ServerConfig {
            host: "127.0.0.1".into(),
            port: server.port,
            endpoint: Endpoint::default(),
        },
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: false,
            allow_http: true,
        },
    );
    let mut proxy = ProxyHandle::start(config).unwrap();

    let reply = run_socks_greeting_reply(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(reply, 0xff);
}

#[test]
fn http_proxy_can_be_disabled_per_listener() {
    let server = spawn_fake_vless_server();
    let config = ClientConfig::single_server(
        "default",
        ServerConfig {
            host: "127.0.0.1".into(),
            port: server.port,
            endpoint: Endpoint::default(),
        },
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: false,
        },
    );
    let mut proxy = ProxyHandle::start(config).unwrap();

    let response = run_http_connect_status(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert!(response.starts_with("HTTP/1.1 403 Forbidden"));
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

#[test]
fn direct_mode_bypasses_tunnel() {
    let echo = spawn_tcp_echo_server();
    let mut config = ClientConfig::raw_vless("127.0.0.1", 1, TEST_UUID, "127.0.0.1", 0).unwrap();
    config.set_active_mode("direct").unwrap();
    let mut proxy = ProxyHandle::start(config).unwrap();

    let response =
        run_socks_echo_to(proxy.snapshot().socket_addr(), "127.0.0.1", echo.port).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello".to_vec());
}

#[test]
fn reject_mode_returns_socks_failure() {
    use crate::config::{Mode, ModeKind};
    use crate::router::{Rule, RuleAction, Script};

    let server = spawn_fake_vless_server();
    let mut config =
        ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
    config
        .upsert_script(Script {
            name: "deny-all".into(),
            rules: vec![Rule::Match {
                action: RuleAction::Reject,
            }],
        })
        .unwrap();
    config
        .upsert_user_mode(Mode {
            name: "reject".into(),
            kind: ModeKind::User,
            proxy: Some("default".into()),
            script: Some("deny-all".into()),
        })
        .unwrap();
    config.set_active_mode("reject").unwrap();
    let mut proxy = ProxyHandle::start(config).unwrap();

    let reply = run_socks_connect_reply(proxy.snapshot().socket_addr(), "example.com", 80).unwrap();
    proxy.stop().unwrap();

    assert_eq!(reply, 0x02, "expected SOCKS connection-not-allowed reply");
}

#[test]
fn script_routes_per_rule_match() {
    use crate::config::{Mode, ModeKind};
    use crate::router::{Rule, RuleAction, Script};

    let server = spawn_fake_vless_server();
    let echo = spawn_tcp_echo_server();
    let mut config =
        ClientConfig::raw_vless("127.0.0.1", server.port, TEST_UUID, "127.0.0.1", 0).unwrap();
    config
        .upsert_script(Script {
            name: "split".into(),
            rules: vec![
                Rule::Domain {
                    value: "127.0.0.1".into(),
                    action: RuleAction::Direct,
                },
                Rule::Match {
                    action: RuleAction::Proxy {
                        name: "default".into(),
                    },
                },
            ],
        })
        .unwrap();
    config
        .upsert_user_mode(Mode {
            name: "split".into(),
            kind: ModeKind::User,
            proxy: Some("default".into()),
            script: Some("split".into()),
        })
        .unwrap();
    config.set_active_mode("split").unwrap();
    let mut proxy = ProxyHandle::start(config).unwrap();
    let local_addr = proxy.snapshot().socket_addr();

    let direct_response = run_socks_echo_to(local_addr, "127.0.0.1", echo.port).unwrap();
    let proxied_response = run_socks_echo_to(local_addr, "example.com", 80).unwrap();
    proxy.stop().unwrap();

    assert_eq!(direct_response, b"hello".to_vec());
    assert_eq!(proxied_response, b"hello".to_vec());
}
