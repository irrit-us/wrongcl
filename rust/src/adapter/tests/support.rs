use super::*;

#[test]
fn anytls_config_missing_password_rejected() {
    let err = toml::from_str::<WrongsvConfig>(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[anytls]
"#,
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("password"),
        "expected missing field error, got {err}"
    );
}

#[test]
fn adapts_raw_tls_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[tls]
server_name = "example.com"
"#,
    )
    .unwrap();

    assert_eq!(active_profile(&cfg), "tls");
    let config = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
    assert!(matches!(config.server.endpoint.transport, Transport::Raw));
    match &config.server.endpoint.outer_security {
        OuterSecurity::Tls(tls) => assert_eq!(tls.server_name, "example.com"),
        other => panic!("expected TLS, got {other:?}"),
    }
}

#[test]
fn tls_config_with_udp_disabled_reports_supported() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
udp = false

[tls]
server_name = "example.com"
"#,
    )
    .unwrap();

    let report = report_for(&cfg);
    assert_eq!(report.active_profile, "tls");
    assert_eq!(report.active_support, SupportLevel::Supported);
    assert_eq!(report.payload_networks, vec![PayloadNetwork::Tcp]);
    assert!(report.missing_fields.is_empty());
    assert!(active_capability(&cfg).config_adaptable);
}

#[test]
fn vision_config_with_udp_disabled_reports_supported() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
flow = "xtls-rprx-vision"
udp = false
"#,
    )
    .unwrap();

    let report = report_for(&cfg);
    assert_eq!(report.active_profile, "raw");
    assert_eq!(report.active_support, SupportLevel::Supported);
    assert_eq!(report.payload_networks, vec![PayloadNetwork::Tcp]);
    assert!(active_capability(&cfg).config_adaptable);
}

#[test]
fn reality_config_with_public_key_and_udp_disabled_reports_supported() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
udp = false

[reality]
public_key = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA"
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
    )
    .unwrap();

    let report = report_for(&cfg);
    assert_eq!(report.active_profile, "reality");
    assert_eq!(report.active_support, SupportLevel::Supported);
    assert_eq!(report.payload_networks, vec![PayloadNetwork::Tcp]);
    assert!(report.missing_fields.is_empty());
    assert!(active_capability(&cfg).config_adaptable);
}

#[test]
fn shadowsocks_tcp_only_reports_supported() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:8388"

[shadowsocks]
method = "chacha20-ietf-poly1305"
password = "hunter2"
udp = false
"#,
    )
    .unwrap();

    let report = report_for(&cfg);
    assert_eq!(report.active_profile, "shadowsocks");
    assert_eq!(report.active_support, SupportLevel::Supported);
    assert_eq!(report.payload_networks, vec![PayloadNetwork::Tcp]);
    assert!(active_capability(&cfg).config_adaptable);
}

#[test]
fn reality_missing_public_key_reports_missing_field_and_no_config() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
udp = false

[reality]
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
    )
    .unwrap();

    let report = report_for(&cfg);
    assert_eq!(report.active_profile, "reality");
    assert_eq!(report.active_support, SupportLevel::Partial);
    assert_eq!(report.payload_networks, vec![PayloadNetwork::Tcp]);
    assert_eq!(report.missing_fields.len(), 1);
    assert_eq!(report.missing_fields[0].field, "reality.public-key");

    let assessment = active_capability(&cfg);
    assert!(!assessment.config_adaptable);
    assert!(assessment.reason.contains("missing client-side fields"));
}

#[test]
fn rejects_unparseable_listen_string() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "not-a-host-port-string"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"
"#,
    )
    .unwrap();
    let err = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
    match err {
        ClientError::Config(msg) => assert!(msg.contains("invalid wrongsv listen")),
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn rejects_unimplemented_profile() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[vmess]
alter_id = 0
"#,
    )
    .unwrap();
    let err = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
    match err {
        ClientError::UnsupportedProtocol(msg) => {
            assert!(msg.contains("vmess"), "msg: {msg}");
            assert!(msg.contains("not implemented"), "msg: {msg}");
        }
        other => panic!("expected UnsupportedProtocol, got {other:?}"),
    }
}

#[test]
fn rejects_trojan_with_empty_password() {
    let cfg: WrongsvConfig = toml::from_str(
        r#"
listen = "0.0.0.0:443"

[trojan]
password = ""
"#,
    )
    .unwrap();
    let err = client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
    match err {
        ClientError::Config(msg) => {
            assert!(msg.to_lowercase().contains("trojan"), "msg: {msg}");
            assert!(msg.to_lowercase().contains("password"), "msg: {msg}");
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}
