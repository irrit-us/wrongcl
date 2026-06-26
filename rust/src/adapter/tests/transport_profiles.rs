use super::*;

#[test]
fn adapts_hysteria2_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[hysteria2]
password = "secret"
disable_udp = false
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "hysteria2");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Udp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.proxy {
        ProxyProtocol::Hysteria2(opts) => {
            assert_eq!(opts.server_name, "foo.cloudfront.net");
            assert_eq!(opts.password, "secret");
            assert!(opts.udp_enabled);
        }
        other => panic!("expected Hysteria2, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "Hysteria2 → QUIC → TLS → TCP"
    );
}

#[test]
fn adapts_tuic_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[tuic]

[[tuic.users]]
uuid = "12345678-1234-1234-1234-123456789abc"
password = "tuic-pass"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "tuic");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Udp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.proxy {
        ProxyProtocol::Tuic(opts) => {
            assert_eq!(opts.server_name, "foo.cloudfront.net");
            assert_eq!(opts.uuid, "12345678-1234-1234-1234-123456789abc");
            assert_eq!(opts.password, "tuic-pass");
        }
        other => panic!("expected TUIC, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "TUIC → QUIC → TLS → TCP"
    );
}

#[test]
fn adapts_quic_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[quic]
udp_relay = true
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "quic");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Udp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Quic(opts) => {
            assert_eq!(opts.server_name, "cloudfront.net");
            assert!(opts.udp_enabled);
        }
        other => panic!("expected QUIC transport, got {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::None
    ));
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → QUIC → TLS → TCP"
    );
}

#[test]
fn adapts_kcp_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
udp = false

[kcp]
seed = "kcp-seed"
mtu = 1400
tti = 20
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "kcp");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(assessment.payload_networks, vec![PayloadNetwork::Tcp]);
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Udp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Kcp(opts) => {
            assert_eq!(opts.seed, "kcp-seed");
            assert_eq!(opts.mtu, 1400);
            assert_eq!(opts.tti, 20);
        }
        other => panic!("expected KCP transport, got {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::None
    ));
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → KCP → TCP"
    );
}

#[test]
fn adapts_webtransport_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[webtransport]
path = "/wt"
host = "wt.example"
udp_relay = true
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "webtransport");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Udp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Webtransport(WebTransportOptions {
            authority,
            path,
            udp_enabled,
        }) => {
            assert_eq!(authority, "wt.example");
            assert_eq!(path, "/wt");
            assert!(*udp_enabled);
        }
        other => panic!("expected WebTransport transport, got {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::None
    ));
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → WebTransport → QUIC → TLS → TCP"
    );
}

#[test]
fn adapts_meek_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[meek]
path = "/meek"
host = "cdn.example"

[meek.tls]
dest = "cover.example:443"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "meek");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Tcp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Meek(opts) => {
            assert_eq!(opts.path, "/meek");
            assert_eq!(opts.host.as_deref(), Some("cdn.example"));
        }
        other => panic!("expected Meek transport, got {other:?}"),
    }
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::Tls(tls) => {
            assert_eq!(tls.server_name, "cover.example");
            assert!(tls.insecure_skip_verify);
        }
        other => panic!("expected TLS outer security, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → Meek → TLS → TCP"
    );
}

#[test]
fn adapts_naive_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[naive]
padding_header_name = "Padding"

[naive.tls]
dest = "cover.example:443"

[[naive.users]]
username = "alice"
password = "secret"
email = "alice@example.com"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "naive");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(assessment.payload_networks, vec![PayloadNetwork::Tcp]);
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Tcp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.proxy {
        ProxyProtocol::Naive(opts) => {
            assert_eq!(opts.username, "alice");
            assert_eq!(opts.password, "secret");
            assert_eq!(opts.padding_header_name, "Padding");
        }
        other => panic!("expected Naive proxy, got {other:?}"),
    }
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::Tls(tls) => {
            assert_eq!(tls.server_name, "cover.example");
            assert!(tls.insecure_skip_verify);
            assert_eq!(tls.alpn, vec!["h2".to_string()]);
        }
        other => panic!("expected TLS outer security, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "Naive → h2 CONNECT → TLS → TCP"
    );
}

#[test]
fn adapts_snell_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[snell]
psk = "hunter2"
version = 1
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "snell");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(assessment.payload_networks, vec![PayloadNetwork::Tcp]);
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Tcp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.proxy {
        ProxyProtocol::Snell(opts) => {
            assert_eq!(opts.psk, "hunter2");
            assert_eq!(opts.version, 1);
        }
        other => panic!("expected Snell proxy, got {other:?}"),
    }
    assert!(matches!(
        config.endpoints[0].server.endpoint.transport,
        Transport::Raw
    ));
    assert!(matches!(
        config.endpoints[0].server.endpoint.outer_security,
        OuterSecurity::None
    ));
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "Snell → raw TCP"
    );
}

#[test]
fn wireguard_config_reports_partial_and_draft_only() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:51820"

[wireguard]
private_key = "EGs4lTSJPmgELx6YiJAmPR2meWi6bY+e9rTdCipSj10="
server_cidrs = ["10.77.0.1/32"]
outbound = true

[[wireguard.peers]]
public_key = "MmLJ5iHFVVBp7VsB0hxfpQ0wEzAbT2KQnpQpj0+RtBw="
allowed_ips = ["10.77.0.2/32"]
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "wireguard");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Partial);
    assert!(!assessment.config_adaptable);
    assert_eq!(assessment.payload_networks, vec![PayloadNetwork::Ip]);
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Udp]);
    assert_eq!(assessment.missing_fields.len(), 1);
    assert_eq!(
        assessment.missing_fields[0].field,
        "wireguard.peers.private-key"
    );

    let resolution = import_resolution_hint(&cfg);
    let plan =
        build_wrongcl_adapt_plan(&cfg, &resolution, "wrong.example", "127.0.0.1", 1080).unwrap();
    assert!(plan.draft_config.is_some());
    assert!(plan.strict_config.is_none());

    let err = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
    match err {
        ClientError::Config(msg) => {
            assert!(msg.contains("wireguard.peers.private-key"), "msg: {msg}");
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn adapts_gdocsviewer_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[gdocsviewer]
path_prefix = "/gdocsviewer"
shared_key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="

[gdocsviewer.tls]
dest = "cover.example:443"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "gdocsviewer");
    let assessment = active_capability(&cfg);
    assert_eq!(assessment.support, SupportLevel::Supported);
    assert!(assessment.config_adaptable);
    assert_eq!(
        assessment.payload_networks,
        vec![PayloadNetwork::Tcp, PayloadNetwork::Udp]
    );
    assert_eq!(assessment.base_carriers, vec![BaseCarrier::Tcp]);

    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    match &config.endpoints[0].server.endpoint.transport {
        Transport::Gdocsviewer(GdocsViewerOptions {
            path_prefix,
            shared_key,
        }) => {
            assert_eq!(path_prefix, "/gdocsviewer");
            assert_eq!(
                shared_key.as_deref(),
                Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
            );
        }
        other => panic!("expected Google Docs Viewer transport, got {other:?}"),
    }
    match &config.endpoints[0].server.endpoint.outer_security {
        OuterSecurity::Tls(tls) => {
            assert_eq!(tls.server_name, "cover.example");
            assert!(tls.insecure_skip_verify);
        }
        other => panic!("expected TLS outer security, got {other:?}"),
    }
    assert_eq!(
        config.endpoints[0].server.endpoint.stack_summary(),
        "VLESS → Google Docs Viewer → TLS → TCP"
    );
}
