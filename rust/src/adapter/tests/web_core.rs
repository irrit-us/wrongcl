use super::*;

#[test]
fn adapts_websocket_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[websocket]
path = "/ws"
"#,
    )
    .unwrap();

    let report = report_for(&cfg);
    assert_eq!(report.active_profile, "websocket");
    assert_eq!(report.active_support, SupportLevel::Supported);
    assert_eq!(
        report.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );
    assert!(active_capability(&cfg).config_adaptable);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    assert!(matches!(
        config.endpoints[0].server.endpoint.proxy,
        ProxyProtocol::Vless(_)
    ));
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Websocket(ws) => assert_eq!(ws.path, "/ws"),
        other => panic!("unexpected transport {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::None
    ));
}

#[test]
fn adapts_websocket_over_tls_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[websocket]
path = "/ws"

[websocket.tls]
server_name = "example.com"
alpn = ["h2", "http/1.1"]
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "websocket");
    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::Tls(tls) => {
            assert_eq!(tls.server_name, "example.com");
            assert_eq!(tls.alpn, vec!["h2".to_string(), "http/1.1".to_string()]);
        }
        other => panic!("expected TLS, got {other:?}"),
    }
}

#[test]
fn adapts_xhttp_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[xhttp]
path = "/xhttp"
host = "xhttp.example"
"#,
    )
    .unwrap();

    let report = report_for(&cfg);
    assert_eq!(report.active_profile, "xhttp");
    assert_eq!(report.active_support, SupportLevel::Supported);
    assert!(active_capability(&cfg).config_adaptable);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Xhttp(xh) => {
            assert_eq!(xh.path, "/xhttp");
            assert_eq!(xh.host.as_deref(), Some("xhttp.example"));
        }
        other => panic!("unexpected transport {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::None
    ));
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → XHTTP → TCP"
    );
}

#[test]
fn adapts_xhttp_over_tls_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[xhttp]
path = "/x"

[xhttp.tls]
server_name = "xhttp.example"
"#,
    )
    .unwrap();

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::Tls(tls) => assert_eq!(tls.server_name, "xhttp.example"),
        other => panic!("expected TLS, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → XHTTP → TLS → TCP"
    );
}

#[test]
fn adapts_grpc_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[grpc]
service_name = "MyGun"
"#,
    )
    .unwrap();

    let report = report_for(&cfg);
    assert_eq!(report.active_profile, "grpc");
    assert_eq!(report.active_support, SupportLevel::Supported);
    assert!(active_capability(&cfg).config_adaptable);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Grpc(gr) => assert_eq!(gr.service_name, "MyGun"),
        other => panic!("unexpected transport {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::None
    ));
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → gRPC → TCP"
    );
}

#[test]
fn adapts_grpc_over_tls_config_defaults_service_name() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[grpc]

[grpc.tls]
server_name = "grpc.example"
"#,
    )
    .unwrap();

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Grpc(gr) => assert_eq!(gr.service_name, "GunService"),
        other => panic!("unexpected transport {other:?}"),
    }
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::Tls(tls) => assert_eq!(tls.server_name, "grpc.example"),
        other => panic!("expected TLS, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → gRPC → TLS → TCP"
    );
}

#[test]
fn adapts_trojan_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[trojan]
password = "hunter2"

[trojan.tls]
server_name = "example.com"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "trojan");
    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.proxy {
        ProxyProtocol::Trojan(opts) => assert_eq!(opts.password, "hunter2"),
        other => panic!("unexpected proxy {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::Tls(_)
    ));
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "Trojan → raw → TLS → TCP"
    );
}

#[test]
fn adapts_mixed_config_with_credentials() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "127.0.0.1:1080"

[mixed]
username = "admin"
password = "secret"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "mixed");
    assert_eq!(active_capability(&cfg).support, SupportLevel::Supported);
    assert!(active_capability(&cfg).config_adaptable);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1081).unwrap();
    match &config.endpoints[0].server.endpoint.proxy {
        ProxyProtocol::Mixed(opts) => {
            assert_eq!(opts.username.as_deref(), Some("admin"));
            assert_eq!(opts.password.as_deref(), Some("secret"));
        }
        other => panic!("unexpected proxy {other:?}"),
    }
}
