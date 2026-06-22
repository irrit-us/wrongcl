use super::*;

#[test]
fn adapts_shadowsocks_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:8388"

[shadowsocks]
method = "chacha20-ietf-poly1305"
password = "hunter2"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "shadowsocks");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.proxy {
        ProxyProtocol::Shadowsocks(opts) => {
            assert_eq!(opts.method, "chacha20-ietf-poly1305");
            assert_eq!(opts.password, "hunter2");
        }
        other => panic!("unexpected proxy {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.transport,
        Transport::Raw
    ));
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::None
    ));
}

#[test]
fn adapts_reality_config_with_vision_flow() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
flow = "xtls-rprx-vision"

[reality]
public_key = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "reality");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(assessment.payload_networks, vec![PayloadNetwork::Tcp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.proxy {
        ProxyProtocol::Vless(opts) => assert_eq!(opts.flow, "xtls-rprx-vision"),
        other => panic!("expected VLESS, got {other:?}"),
    }
}

#[test]
fn rejects_unknown_vless_flow() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
flow = "xtls-rprx-direct"

[reality]
public_key = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
    )
    .unwrap();

    let err = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
    match err {
        ClientError::UnsupportedProtocol(msg) => {
            assert!(msg.contains("xtls-rprx-direct"));
        }
        other => panic!("expected UnsupportedProtocol, got {other:?}"),
    }
}

#[test]
fn adapts_reality_config_without_vision_flow() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[reality]
public_key = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "reality");
    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::Reality(opts) => {
            assert_eq!(opts.server_name, "www.microsoft.com");
            assert_eq!(opts.short_id, "aaaaaaaa");
            assert_eq!(
                opts.public_key,
                "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
            );
        }
        other => panic!("expected REALITY, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → raw → REALITY → TCP"
    );
}

#[test]
fn reality_config_missing_public_key_rejected() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[reality]
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
    )
    .unwrap();

    let err = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
    match err {
        ClientError::Config(msg) => assert!(msg.contains("public-key")),
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn adapts_anytls_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[anytls]
password = "hunter2"
server_name = "cloudfront.net"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "anytls");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::AnyTls(opts) => {
            assert_eq!(opts.password, "hunter2");
            assert_eq!(opts.server_name, "cloudfront.net");
            assert!(opts.insecure_skip_verify);
        }
        other => panic!("expected AnyTLS, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → raw → AnyTLS → TCP"
    );
}

#[test]
fn adapts_shadowtls_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[shadowtls]
password = "shadow-pass"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "shadowtls");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::ShadowTls(opts) => {
            assert_eq!(opts.server_name, "cloudfront.net");
            assert_eq!(opts.password, "shadow-pass");
        }
        other => panic!("expected ShadowTLS, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → raw → ShadowTLS → TCP"
    );
}
