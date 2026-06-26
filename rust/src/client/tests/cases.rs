use super::*;
use crate::WireGuardOptions;

#[test]
fn probe_works_against_fake_raw_vless_server() {
    let server = spawn_fake_server(FakeCarrier::Raw);
    let client = WrongsvClient::new(vless_server(
        "127.0.0.1",
        server.port,
        TEST_UUID,
        Transport::Raw,
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .unwrap();
    assert_eq!(result.bytes_read, 4);
    assert_eq!(result.preview, "ping");
}

#[test]
fn probe_works_with_fragment_transport() {
    let server = spawn_fake_server(FakeCarrier::Raw);
    let client = WrongsvClient::new(vless_server(
        "127.0.0.1",
        server.port,
        TEST_UUID,
        Transport::Fragment(FragmentOptions {
            length_min: 1,
            length_max: 1,
            delay_min_ms: 0,
            delay_max_ms: 0,
            packets_from: 1,
            packets_to: 2,
        }),
    ))
    .unwrap();

    let mut tunnel = client
        .connect(&Target::new("example.com", 80).unwrap())
        .unwrap();
    tunnel.write_all(b"frag").unwrap();
    let mut echoed = [0u8; 4];
    tunnel.read_exact(&mut echoed).unwrap();
    assert_eq!(&echoed, b"frag");
}

#[test]
fn probe_works_against_fake_httpupgrade_server() {
    let server = spawn_fake_server(FakeCarrier::HttpUpgrade);
    let client = WrongsvClient::new(vless_server(
        "127.0.0.1",
        server.port,
        TEST_UUID,
        Transport::Httpupgrade(HuOptions {
            path: "/up".into(),
            host: None,
        }),
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .unwrap();
    assert_eq!(result.preview, "ping");
}

#[test]
fn socks_proxy_udp_works_against_fake_httpupgrade_server() {
    let server = spawn_fake_server(FakeCarrier::HttpUpgrade);
    let client = WrongsvClient::new(vless_server(
        "127.0.0.1",
        server.port,
        TEST_UUID,
        Transport::Httpupgrade(HuOptions {
            path: "/up".into(),
            host: None,
        }),
    ))
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
    panic!("no UDP response from HTTPUpgrade session");
}

#[test]
fn socks_proxy_works_against_fake_httpupgrade_server() {
    let server = spawn_fake_server(FakeCarrier::HttpUpgrade);
    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        vless_server(
            "127.0.0.1",
            server.port,
            TEST_UUID,
            Transport::Httpupgrade(HuOptions {
                path: "/up".into(),
                host: None,
            }),
        ),
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-httpup").unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-httpup".to_vec());
}

#[test]
fn probe_works_against_fake_websocket_server() {
    let server = spawn_fake_server(FakeCarrier::WebSocket);
    let client = WrongsvClient::new(vless_server(
        "127.0.0.1",
        server.port,
        TEST_UUID,
        Transport::Websocket(WsOptions {
            path: "/ws".into(),
            host: None,
        }),
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .unwrap();
    assert_eq!(result.preview, "ping");
}

#[test]
fn socks_proxy_udp_works_against_fake_websocket_server() {
    let server = spawn_fake_server(FakeCarrier::WebSocket);
    let client = WrongsvClient::new(vless_server(
        "127.0.0.1",
        server.port,
        TEST_UUID,
        Transport::Websocket(WsOptions {
            path: "/ws".into(),
            host: None,
        }),
    ))
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
    panic!("no UDP response from WebSocket session");
}

#[test]
fn socks_proxy_udp_works_against_fake_kcp_server() {
    let kcp_opts = KcpOptions {
        seed: "kcp-seed".into(),
        mtu: 1350,
        tti: 50,
    };
    let server = spawn_fake_kcp_server(kcp_opts.clone());
    let client = WrongsvClient::new(vless_server(
        "127.0.0.1",
        server.port,
        TEST_UUID,
        Transport::Kcp(kcp_opts),
    ))
    .unwrap();

    let mut session = client
        .connect_udp_session(&Target::new("example.com", 53).unwrap())
        .unwrap();
    session.send_packet(b"ping-kcp-udp").unwrap();
    for _ in 0..40 {
        if let Some(packet) = session.try_recv_packet().unwrap() {
            assert_eq!(packet.payload, b"ping-kcp-udp");
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("no UDP response from KCP session");
}

#[test]
fn socks_proxy_works_against_fake_websocket_server() {
    let server = spawn_fake_server(FakeCarrier::WebSocket);
    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        vless_server(
            "127.0.0.1",
            server.port,
            TEST_UUID,
            Transport::Websocket(WsOptions {
                path: "/ws".into(),
                host: None,
            }),
        ),
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-ws").unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-ws".to_vec());
}

#[test]
fn probe_works_against_fake_remote_socks5_server() {
    let server = spawn_fake_socks5_server(None, None);
    let client = WrongsvClient::new(mixed_server(
        "127.0.0.1",
        server.port,
        MixedOptions::default(),
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .unwrap();
    assert_eq!(result.preview, "ping");
}

#[test]
fn probe_works_against_fake_authenticated_remote_socks5_server() {
    let server = spawn_fake_socks5_server(Some("user"), Some("pass"));
    let client = WrongsvClient::new(mixed_server(
        "127.0.0.1",
        server.port,
        MixedOptions {
            username: Some("user".into()),
            password: Some("pass".into()),
        },
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .unwrap();
    assert_eq!(result.preview, "ping");
}

#[test]
fn socks_proxy_udp_works_against_fake_remote_socks5_server() {
    let server = spawn_fake_socks5_server(None, None);
    let client = WrongsvClient::new(mixed_server(
        "127.0.0.1",
        server.port,
        MixedOptions::default(),
    ))
    .unwrap();

    let mut session = client
        .connect_udp_session(&Target::new("example.com", 53).unwrap())
        .unwrap();
    session.send_packet(b"ping-udp").unwrap();
    for _ in 0..40 {
        if let Some(packet) = session.try_recv_packet().unwrap() {
            assert_eq!(packet.payload, b"ping-udp");
            assert_eq!(packet.target.host, "example.com");
            assert_eq!(packet.target.port, 53);
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("no UDP response from remote SOCKS5 session");
}

#[test]
fn local_proxy_udp_works_against_fake_remote_socks5_server() {
    let server = spawn_fake_socks5_server(None, None);
    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        mixed_server("127.0.0.1", server.port, MixedOptions::default()),
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_udp_echo(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"ping-udp".to_vec());
}

#[test]
fn local_proxy_udp_works_against_fake_kcp_server() {
    let kcp_opts = KcpOptions {
        seed: "kcp-seed".into(),
        mtu: 1350,
        tti: 50,
    };
    let server = spawn_fake_kcp_server(kcp_opts.clone());
    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        vless_server(
            "127.0.0.1",
            server.port,
            TEST_UUID,
            Transport::Kcp(kcp_opts),
        ),
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_udp_echo(proxy.snapshot().socket_addr()).unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"ping-udp".to_vec());
}

#[test]
fn probe_works_against_fake_remote_http_connect_server() {
    let server = spawn_fake_http_connect_server(None, None);
    let client = WrongsvClient::new(mixed_server(
        "127.0.0.1",
        server.port,
        MixedOptions::default(),
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .unwrap();
    assert_eq!(result.preview, "ping");
}

#[test]
fn probe_works_against_fake_authenticated_remote_http_connect_server() {
    let server = spawn_fake_http_connect_server(Some("user"), Some("pass"));
    let client = WrongsvClient::new(mixed_server(
        "127.0.0.1",
        server.port,
        MixedOptions {
            username: Some("user".into()),
            password: Some("pass".into()),
        },
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .unwrap();
    assert_eq!(result.preview, "ping");
}

#[test]
fn probe_works_against_fake_shadowsocks_server() {
    let server = spawn_fake_shadowsocks_server("chacha20-ietf-poly1305".into(), "hunter2".into());
    let client = WrongsvClient::new(shadowsocks_server(
        "127.0.0.1",
        server.port,
        ShadowsocksOptions {
            method: "chacha20-ietf-poly1305".into(),
            password: "hunter2".into(),
        },
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping")
        .unwrap();
    assert_eq!(result.bytes_read, 4);
    assert_eq!(result.preview, "ping");
}

#[test]
fn probe_works_against_fake_snell_server() {
    let server = spawn_fake_snell_server("hunter2".into(), 1);
    let client = WrongsvClient::new(snell_server(
        "127.0.0.1",
        server.port,
        SnellOptions {
            psk: "hunter2".into(),
            version: 1,
        },
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping-snell")
        .unwrap();
    assert_eq!(result.bytes_read, 10);
    assert_eq!(result.preview, "ping-snell");
}

#[test]
fn supports_udp_tracks_transport_capability() {
    let raw =
        WrongsvClient::new(vless_server("127.0.0.1", 443, TEST_UUID, Transport::Raw)).unwrap();
    assert!(raw.supports_udp());

    let kcp = WrongsvClient::new(vless_server(
        "127.0.0.1",
        443,
        TEST_UUID,
        Transport::Kcp(KcpOptions {
            seed: String::new(),
            mtu: 1350,
            tti: 50,
        }),
    ))
    .unwrap();
    assert!(kcp.supports_udp());

    let quic_disabled = WrongsvClient::new(vless_server(
        "127.0.0.1",
        443,
        TEST_UUID,
        Transport::Quic(QuicOptions {
            server_name: "cloudfront.net".into(),
            udp_enabled: false,
        }),
    ))
    .unwrap();
    assert!(!quic_disabled.supports_udp());

    let webtransport_disabled = WrongsvClient::new(vless_server(
        "127.0.0.1",
        443,
        TEST_UUID,
        Transport::Webtransport(WebTransportOptions {
            authority: "wt.example".into(),
            path: "/wt".into(),
            udp_enabled: false,
        }),
    ))
    .unwrap();
    assert!(!webtransport_disabled.supports_udp());

    let mixed =
        WrongsvClient::new(mixed_server("127.0.0.1", 443, MixedOptions::default())).unwrap();
    assert!(mixed.supports_udp());

    let snell = WrongsvClient::new(snell_server(
        "127.0.0.1",
        443,
        SnellOptions {
            psk: "hunter2".into(),
            version: 1,
        },
    ))
    .unwrap();
    assert!(!snell.supports_udp());

    let wireguard = WrongsvClient::new(ServerConfig {
        host: "127.0.0.1".into(),
        port: 443,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Wireguard(WireGuardOptions {
                private_key: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
                peer_public_key: "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=".into(),
                pre_shared_key: None,
                client_ip: "10.66.66.2/32".into(),
                allowed_ips: vec!["0.0.0.0/0".into()],
                mtu: 1400,
            }),
            transport: Transport::Raw,
            outer_security: OuterSecurity::None,
        },
    })
    .unwrap();
    assert!(wireguard.supports_udp());
}

#[test]
fn probe_works_against_fake_shadowsocks_aead_2022_server() {
    let psk_b64 = "AAAAAAAAAAAAAAAAAAAAAA==";
    let server = spawn_fake_shadowsocks_server("2022-blake3-aes-128-gcm".into(), psk_b64.into());
    let client = WrongsvClient::new(shadowsocks_server(
        "127.0.0.1",
        server.port,
        ShadowsocksOptions {
            method: "2022-blake3-aes-128-gcm".into(),
            password: psk_b64.into(),
        },
    ))
    .unwrap();

    let result = client
        .probe(&Target::new("example.com", 80).unwrap(), "ping-2022")
        .unwrap();
    assert_eq!(result.preview, "ping-2022");
}

#[test]
fn socks_proxy_works_against_fake_shadowsocks_server() {
    let server = spawn_fake_shadowsocks_server("chacha20-ietf-poly1305".into(), "hunter2".into());
    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        shadowsocks_server(
            "127.0.0.1",
            server.port,
            ShadowsocksOptions {
                method: "chacha20-ietf-poly1305".into(),
                password: "hunter2".into(),
            },
        ),
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-ss").unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-ss".to_vec());
}

#[test]
fn socks_proxy_works_against_fake_shadowsocks_aead_2022_server() {
    let psk_b64 = "AAAAAAAAAAAAAAAAAAAAAA==";
    let server = spawn_fake_shadowsocks_server("2022-blake3-aes-128-gcm".into(), psk_b64.into());
    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        shadowsocks_server(
            "127.0.0.1",
            server.port,
            ShadowsocksOptions {
                method: "2022-blake3-aes-128-gcm".into(),
                password: psk_b64.into(),
            },
        ),
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-2022").unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-2022".to_vec());
}

#[test]
fn socks_proxy_works_against_fake_snell_server() {
    let server = spawn_fake_snell_server("hunter2".into(), 1);
    let mut proxy = ProxyHandle::start(ClientConfig::single_server(
        "default",
        snell_server(
            "127.0.0.1",
            server.port,
            SnellOptions {
                psk: "hunter2".into(),
                version: 1,
            },
        ),
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
            allow_socks: true,
            allow_http: true,
        },
    ))
    .unwrap();

    let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-snell").unwrap();
    proxy.stop().unwrap();

    assert_eq!(response, b"hello-snell".to_vec());
}
