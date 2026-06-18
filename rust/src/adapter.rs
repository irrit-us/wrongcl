use std::path::Path;

use wrongsv::{
    build_wrongcl_adapt_plan, build_wrongcl_adapt_result, build_wrongcl_inspection,
    import_resolution_hint, load_import_config_path, BaseCarrierId as WrongsvBaseCarrier,
    ImportConfig as WrongsvConfig, ImportResolutionHint, PayloadNetworkId as WrongsvPayloadNetwork,
    WrongclAdaptResultDocument, WrongclInspection, WrongclMissingField, WrongclProfileView,
    WrongclSupportLevel as WrongsvSupportLevel,
};

use crate::error::{ClientError, Result};

pub type CapabilityReport = WrongclInspection;
pub type ProfileSupport = WrongclProfileView;
pub type AdaptedConfig = WrongclAdaptResultDocument;
pub type SupportLevel = WrongsvSupportLevel;
pub type PayloadNetwork = WrongsvPayloadNetwork;
pub type BaseCarrier = WrongsvBaseCarrier;
pub type MissingField = WrongclMissingField;

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CapabilityAssessment {
    payload_networks: Vec<PayloadNetwork>,
    base_carriers: Vec<BaseCarrier>,
    support: SupportLevel,
    reason: String,
    missing_fields: Vec<MissingField>,
    config_adaptable: bool,
}

pub fn inspect_wrongsv_config(path: impl AsRef<Path>) -> Result<CapabilityReport> {
    let path = path.as_ref();
    let cfg = read_wrongsv_config(path)?;
    let resolution = read_source_of_truth(path, &cfg)?;
    Ok(build_wrongcl_inspection(&cfg, &resolution))
}

pub fn adapt_wrongsv_config(
    path: impl AsRef<Path>,
    server_host: impl Into<String>,
    listen_host: impl Into<String>,
    listen_port: u16,
) -> Result<AdaptedConfig> {
    let path = path.as_ref();
    let cfg = read_wrongsv_config(path)?;
    let resolution = read_source_of_truth(path, &cfg)?;
    let server_host = server_host.into();
    let listen_host = listen_host.into();
    let plan = build_wrongcl_adapt_plan(&cfg, &resolution, &server_host, &listen_host, listen_port)
        .map_err(map_import_spec_error)?;
    Ok(build_wrongcl_adapt_result(&plan))
}

fn read_wrongsv_config(path: impl AsRef<Path>) -> Result<WrongsvConfig> {
    load_import_config_path(path).map_err(ClientError::Config)
}

fn read_source_of_truth(
    path: impl AsRef<Path>,
    cfg: &WrongsvConfig,
) -> Result<ImportResolutionHint> {
    let inspection = wrongsv::inspect_server_config_path(path).map_err(|error| {
        ClientError::Config(format!("wrongsv endpoint diagnostics failed: {error}"))
    })?;
    let local_hint = import_resolution_hint(cfg);
    let local_profile = local_hint.active_profile.clone();
    let local_inspection = build_wrongcl_inspection(cfg, &local_hint);
    let local_missing_fields = local_inspection.missing_fields;
    if !local_missing_fields.is_empty() && inspection.active_profile != local_profile {
        return Ok(local_hint);
    }
    Ok(ImportResolutionHint {
        active_profile: inspection.active_profile,
        payload_networks: inspection.payload_networks,
        base_carriers: inspection.base_carriers,
    })
}

#[cfg(test)]
fn report_for(cfg: &WrongsvConfig) -> CapabilityReport {
    let resolution = import_resolution_hint(cfg);
    build_wrongcl_inspection(cfg, &resolution)
}

#[cfg(test)]
fn active_capability(cfg: &WrongsvConfig) -> CapabilityAssessment {
    let hint = import_resolution_hint(cfg);
    let inspection = build_wrongcl_inspection(cfg, &hint);
    capability_assessment_from_inspection(&inspection)
}

#[cfg(test)]
fn capability_assessment_from_inspection(inspection: &WrongclInspection) -> CapabilityAssessment {
    CapabilityAssessment {
        payload_networks: inspection.payload_networks.clone(),
        base_carriers: inspection.base_carriers.clone(),
        support: inspection.active_support,
        reason: inspection.active_reason.clone(),
        missing_fields: inspection.missing_fields.clone(),
        config_adaptable: inspection.config_adaptable,
    }
}

#[cfg(test)]
fn client_config_from_document(
    document: &wrongsv::WrongclClientConfigDocument,
    validate: bool,
) -> Result<crate::config::ClientConfig> {
    let value = serde_json::to_value(document)?;
    let config: crate::config::ClientConfig = serde_json::from_value(value)?;
    if validate {
        config.validate()?;
    }
    Ok(config)
}

#[cfg(test)]
fn client_config_for(
    cfg: WrongsvConfig,
    server_host: String,
    listen_host: String,
    listen_port: u16,
) -> Result<crate::config::ClientConfig> {
    let resolution = import_resolution_hint(&cfg);
    let plan = build_wrongcl_adapt_plan(&cfg, &resolution, &server_host, &listen_host, listen_port)
        .map_err(map_import_spec_error)?;
    let document = match plan.strict_config {
        Some(config) => config,
        None => {
            if matches!(
                plan.inspection.active_support,
                WrongsvSupportLevel::Unsupported
            ) {
                return Err(ClientError::UnsupportedProtocol(format!(
                    "wrongsv profile '{}' is recognized but not implemented in wrongcl yet",
                    plan.inspection.active_profile
                )));
            }
            return Err(ClientError::Config(plan.inspection.active_reason));
        }
    };
    client_config_from_document(&document, true)
}

fn map_import_spec_error(error: String) -> ClientError {
    if error.contains("recognized but not implemented in wrongcl yet") {
        ClientError::UnsupportedProtocol(error)
    } else {
        ClientError::Config(error)
    }
}

#[cfg(test)]
fn active_profile(cfg: &WrongsvConfig) -> &'static str {
    wrongsv::active_profile_id(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::{
        GdocsViewerOptions, OuterSecurity, ProxyProtocol, Transport, WebTransportOptions,
    };

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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        assert!(matches!(
            config.server.endpoint.proxy,
            ProxyProtocol::Vless(_)
        ));
        match &config.server.endpoint.transport {
            Transport::Websocket(ws) => assert_eq!(ws.path, "/ws"),
            other => panic!("unexpected transport {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
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
        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Xhttp(xh) => {
                assert_eq!(xh.path, "/xhttp");
                assert_eq!(xh.host.as_deref(), Some("xhttp.example"));
            }
            other => panic!("unexpected transport {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
        assert_eq!(
            config.server.endpoint.stack_summary(),
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
            OuterSecurity::Tls(tls) => assert_eq!(tls.server_name, "xhttp.example"),
            other => panic!("expected TLS, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Grpc(gr) => assert_eq!(gr.service_name, "MyGun"),
            other => panic!("unexpected transport {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
        assert_eq!(config.server.endpoint.stack_summary(), "VLESS → gRPC → TCP");
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Grpc(gr) => assert_eq!(gr.service_name, "GunService"),
            other => panic!("unexpected transport {other:?}"),
        }
        match &config.server.endpoint.outer_security {
            OuterSecurity::Tls(tls) => assert_eq!(tls.server_name, "grpc.example"),
            other => panic!("expected TLS, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
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
        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Trojan(opts) => assert_eq!(opts.password, "hunter2"),
            other => panic!("unexpected proxy {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::Tls(_)
        ));
        assert_eq!(
            config.server.endpoint.stack_summary(),
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1081).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Mixed(opts) => {
                assert_eq!(opts.username.as_deref(), Some("admin"));
                assert_eq!(opts.password.as_deref(), Some("secret"));
            }
            other => panic!("unexpected proxy {other:?}"),
        }
    }

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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Shadowsocks(opts) => {
                assert_eq!(opts.method, "chacha20-ietf-poly1305");
                assert_eq!(opts.password, "hunter2");
            }
            other => panic!("unexpected proxy {other:?}"),
        }
        assert!(matches!(config.server.endpoint.transport, Transport::Raw));
        assert!(matches!(
            config.server.endpoint.outer_security,
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.proxy {
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

        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
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
        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
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
            config.server.endpoint.stack_summary(),
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

        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
            OuterSecurity::AnyTls(opts) => {
                assert_eq!(opts.password, "hunter2");
                assert_eq!(opts.server_name, "cloudfront.net");
                assert!(opts.insecure_skip_verify);
            }
            other => panic!("expected AnyTLS, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.outer_security {
            OuterSecurity::ShadowTls(opts) => {
                assert_eq!(opts.server_name, "cloudfront.net");
                assert_eq!(opts.password, "shadow-pass");
            }
            other => panic!("expected ShadowTLS, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → raw → ShadowTLS → TCP"
        );
    }

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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Hysteria2(opts) => {
                assert_eq!(opts.server_name, "foo.cloudfront.net");
                assert_eq!(opts.password, "secret");
                assert!(opts.udp_enabled);
            }
            other => panic!("expected Hysteria2, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.proxy {
            ProxyProtocol::Tuic(opts) => {
                assert_eq!(opts.server_name, "foo.cloudfront.net");
                assert_eq!(opts.uuid, "12345678-1234-1234-1234-123456789abc");
                assert_eq!(opts.password, "tuic-pass");
            }
            other => panic!("expected TUIC, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Quic(opts) => {
                assert_eq!(opts.server_name, "cloudfront.net");
                assert!(opts.udp_enabled);
            }
            other => panic!("expected QUIC transport, got {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
        assert_eq!(
            config.server.endpoint.stack_summary(),
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Kcp(opts) => {
                assert_eq!(opts.seed, "kcp-seed");
                assert_eq!(opts.mtu, 1400);
                assert_eq!(opts.tti, 20);
            }
            other => panic!("expected KCP transport, got {other:?}"),
        }
        assert!(matches!(
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
        assert_eq!(config.server.endpoint.stack_summary(), "VLESS → KCP → TCP");
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
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
            config.server.endpoint.outer_security,
            OuterSecurity::None
        ));
        assert_eq!(
            config.server.endpoint.stack_summary(),
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
            Transport::Meek(opts) => {
                assert_eq!(opts.path, "/meek");
                assert_eq!(opts.host.as_deref(), Some("cdn.example"));
            }
            other => panic!("expected Meek transport, got {other:?}"),
        }
        match &config.server.endpoint.outer_security {
            OuterSecurity::Tls(tls) => {
                assert_eq!(tls.server_name, "cover.example");
                assert!(tls.insecure_skip_verify);
            }
            other => panic!("expected TLS outer security, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → Meek → TLS → TCP"
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
        assert_eq!(assessment.missing_fields[0].field, "wireguard.private-key");

        let resolution = import_resolution_hint(&cfg);
        let plan = build_wrongcl_adapt_plan(&cfg, &resolution, "wrong.example", "127.0.0.1", 1080)
            .unwrap();
        assert!(plan.draft_config.is_some());
        assert!(plan.strict_config.is_none());

        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
        match err {
            ClientError::Config(msg) => {
                assert!(msg.contains("wireguard.private-key"), "msg: {msg}");
                assert!(
                    msg.contains("no TUN or routed-tunnel runtime"),
                    "msg: {msg}"
                );
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

        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
        match &config.server.endpoint.transport {
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
        match &config.server.endpoint.outer_security {
            OuterSecurity::Tls(tls) => {
                assert_eq!(tls.server_name, "cover.example");
                assert!(tls.insecure_skip_verify);
            }
            other => panic!("expected TLS outer security, got {other:?}"),
        }
        assert_eq!(
            config.server.endpoint.stack_summary(),
            "VLESS → Google Docs Viewer → TLS → TCP"
        );
    }

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
        let config =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap();
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
        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
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
        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
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
        let err =
            client_config_for(cfg, "wrong.example".into(), "127.0.0.1".into(), 1080).unwrap_err();
        match err {
            ClientError::Config(msg) => {
                assert!(msg.to_lowercase().contains("trojan"), "msg: {msg}");
                assert!(msg.to_lowercase().contains("password"), "msg: {msg}");
            }
            other => panic!("expected Config error, got {other:?}"),
        }
    }
}
