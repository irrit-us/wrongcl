use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::dns::DnsSettings;
use crate::endpoint::{Endpoint, OuterSecurity, ProxyProtocol, Transport, VlessOptions};
use crate::error::{ClientError, Result};
use crate::router::{self, Decision, Rule, RuleAction, Script};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(flatten)]
    pub endpoint: Endpoint,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalProxyConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_local_protocol_enabled")]
    pub allow_socks: bool,
    #[serde(default = "default_local_protocol_enabled")]
    pub allow_http: bool,
}

fn default_local_protocol_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedEndpoint {
    pub name: String,
    #[serde(flatten)]
    pub server: ServerConfig,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyGroupKind {
    Select,
    Fallback,
    UrlTest,
}

impl ProxyGroupKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Fallback => "fallback",
            Self::UrlTest => "url-test",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    pub kind: ProxyGroupKind,
    pub members: Vec<String>,
    #[serde(default)]
    pub selected: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ActiveSelection {
    Endpoint { name: String },
    Group { name: String },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModeKind {
    Global,
    Rule,
    Direct,
    User,
}

impl ModeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Rule => "rule",
            Self::Direct => "direct",
            Self::User => "user",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Mode {
    pub name: String,
    pub kind: ModeKind,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
}

pub const BUILTIN_GLOBAL: &str = "global";
pub const BUILTIN_RULE: &str = "rule";
pub const BUILTIN_DIRECT: &str = "direct";

fn default_modes() -> Vec<Mode> {
    vec![
        Mode {
            name: BUILTIN_GLOBAL.into(),
            kind: ModeKind::Global,
            proxy: None,
            script: None,
        },
        Mode {
            name: BUILTIN_RULE.into(),
            kind: ModeKind::Rule,
            proxy: None,
            script: None,
        },
        Mode {
            name: BUILTIN_DIRECT.into(),
            kind: ModeKind::Direct,
            proxy: None,
            script: None,
        },
    ]
}

fn default_active_mode() -> String {
    BUILTIN_GLOBAL.into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub endpoints: Vec<NamedEndpoint>,
    #[serde(default)]
    pub groups: Vec<ProxyGroup>,
    #[serde(default)]
    pub scripts: Vec<Script>,
    #[serde(default = "default_modes")]
    pub modes: Vec<Mode>,
    #[serde(default = "default_active_mode")]
    pub active_mode: String,
    pub active: ActiveSelection,
    pub local: LocalProxyConfig,
    #[serde(default)]
    pub dns: DnsSettings,
}

impl ClientConfig {
    pub fn single_server(
        name: impl Into<String>,
        server: ServerConfig,
        local: LocalProxyConfig,
    ) -> Self {
        let name = name.into();
        Self {
            endpoints: vec![NamedEndpoint {
                name: name.clone(),
                server,
            }],
            groups: vec![],
            scripts: vec![],
            modes: default_modes(),
            active_mode: default_active_mode(),
            active: ActiveSelection::Endpoint { name },
            local,
            dns: DnsSettings::default(),
        }
    }

    pub fn raw_vless(
        server_host: impl Into<String>,
        server_port: u16,
        uuid: impl Into<String>,
        local_host: impl Into<String>,
        local_port: u16,
    ) -> Result<Self> {
        let server = ServerConfig {
            host: server_host.into(),
            port: server_port,
            endpoint: Endpoint {
                proxy: ProxyProtocol::Vless(VlessOptions {
                    uuid: uuid.into(),
                    flow: String::new(),
                }),
                transport: Transport::Raw,
                outer_security: OuterSecurity::None,
            },
        };
        let local = LocalProxyConfig {
            host: local_host.into(),
            port: local_port,
            allow_socks: true,
            allow_http: true,
        };
        let config = Self::single_server("default", server, local);
        config.validate()?;
        Ok(config)
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let trimmed = content.trim_start();
        if trimmed.starts_with('{') {
            return Self::from_json(&content);
        }
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let config: Self = serde_json::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    /// Build a single-endpoint ClientConfig from the legacy `{server, local}`
    /// document shape emitted by the upstream `wrongsv` crate.
    pub fn from_legacy_document_json(text: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct LegacyDoc {
            server: ServerConfig,
            local: LocalProxyConfig,
        }
        let doc: LegacyDoc = serde_json::from_str(text)?;
        let config = Self::single_server("default", doc.server, doc.local);
        config.validate()?;
        Ok(config)
    }

    pub fn to_toml_string(&self) -> Result<String> {
        self.validate()?;
        toml::to_string_pretty(self)
            .map_err(|e| ClientError::Config(format!("serialize config to TOML: {e}")))
    }

    pub fn validate(&self) -> Result<()> {
        if self.endpoints.is_empty() {
            return Err(ClientError::Config(
                "at least one endpoint is required".into(),
            ));
        }
        let mut seen_endpoints = HashSet::new();
        for ep in &self.endpoints {
            if ep.name.trim().is_empty() {
                return Err(ClientError::Config(
                    "endpoint name must not be empty".into(),
                ));
            }
            if !seen_endpoints.insert(ep.name.as_str()) {
                return Err(ClientError::Config(format!(
                    "duplicate endpoint name '{}'",
                    ep.name
                )));
            }
            validate_host(&ep.server.host, "endpoint host")?;
            validate_port(ep.server.port, "endpoint port")?;
            ep.server.endpoint.validate()?;
        }

        let mut seen_groups = HashSet::new();
        for group in &self.groups {
            if group.name.trim().is_empty() {
                return Err(ClientError::Config("group name must not be empty".into()));
            }
            if !seen_groups.insert(group.name.as_str()) {
                return Err(ClientError::Config(format!(
                    "duplicate group name '{}'",
                    group.name
                )));
            }
            if group.members.is_empty() {
                return Err(ClientError::Config(format!(
                    "group '{}' must list at least one member",
                    group.name
                )));
            }
            for member in &group.members {
                if !self.endpoints.iter().any(|e| &e.name == member) {
                    return Err(ClientError::Config(format!(
                        "group '{}' references unknown endpoint '{}'",
                        group.name, member
                    )));
                }
            }
            if let Some(selected) = &group.selected {
                if !group.members.iter().any(|m| m == selected) {
                    return Err(ClientError::Config(format!(
                        "group '{}' selected member '{}' is not in members",
                        group.name, selected
                    )));
                }
            }
        }

        validate_host(&self.local.host, "local listen host")?;
        if !self.local.allow_socks && !self.local.allow_http {
            return Err(ClientError::Config(
                "local proxy must enable at least one of SOCKS or HTTP".into(),
            ));
        }
        self.resolve_active_endpoint()?;
        self.validate_scripts_and_modes()?;
        self.dns.validate()?;
        Ok(())
    }

    fn validate_scripts_and_modes(&self) -> Result<()> {
        let mut seen_scripts = HashSet::new();
        for script in &self.scripts {
            script.validate()?;
            if !seen_scripts.insert(script.name.as_str()) {
                return Err(ClientError::Config(format!(
                    "duplicate script name '{}'",
                    script.name
                )));
            }
            for rule in &script.rules {
                if let Some(RuleAction::Proxy { name }) = rule_action(rule) {
                    self.resolve_proxy_target(name).map_err(|_| {
                        ClientError::Config(format!(
                            "script '{}' references unknown proxy '{}'",
                            script.name, name
                        ))
                    })?;
                }
            }
        }

        let mut seen_modes = HashSet::new();
        for mode in &self.modes {
            if mode.name.trim().is_empty() {
                return Err(ClientError::Config("mode name must not be empty".into()));
            }
            if !seen_modes.insert(mode.name.as_str()) {
                return Err(ClientError::Config(format!(
                    "duplicate mode name '{}'",
                    mode.name
                )));
            }
            if let Some(name) = &mode.proxy {
                self.resolve_proxy_target(name).map_err(|_| {
                    ClientError::Config(format!(
                        "mode '{}' references unknown proxy '{}'",
                        mode.name, name
                    ))
                })?;
            }
            if let Some(script_name) = &mode.script {
                if !self.scripts.iter().any(|s| &s.name == script_name) {
                    return Err(ClientError::Config(format!(
                        "mode '{}' references unknown script '{}'",
                        mode.name, script_name
                    )));
                }
            }
            match mode.kind {
                ModeKind::User => {
                    if mode.proxy.is_none() {
                        return Err(ClientError::Config(format!(
                            "user mode '{}' must specify a proxy",
                            mode.name
                        )));
                    }
                }
                ModeKind::Global | ModeKind::Rule | ModeKind::Direct => {}
            }
        }

        if !self.modes.iter().any(|m| m.name == self.active_mode) {
            return Err(ClientError::Config(format!(
                "active_mode '{}' is not a defined mode",
                self.active_mode
            )));
        }

        Ok(())
    }

    pub fn find_endpoint(&self, name: &str) -> Result<&NamedEndpoint> {
        self.endpoints
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| ClientError::Config(format!("endpoint '{name}' is not defined")))
    }

    pub fn find_group(&self, name: &str) -> Result<&ProxyGroup> {
        self.groups
            .iter()
            .find(|g| g.name == name)
            .ok_or_else(|| ClientError::Config(format!("group '{name}' is not defined")))
    }

    pub fn resolve_active_endpoint(&self) -> Result<&NamedEndpoint> {
        match &self.active {
            ActiveSelection::Endpoint { name } => self.find_endpoint(name),
            ActiveSelection::Group { name } => {
                let group = self.find_group(name)?;
                let selected = group.selected.as_deref().ok_or_else(|| {
                    ClientError::Config(format!("group '{name}' has no selected member"))
                })?;
                self.find_endpoint(selected)
            }
        }
    }

    pub fn select_group_member(&mut self, group: &str, member: &str) -> Result<()> {
        if !self.endpoints.iter().any(|e| e.name == member) {
            return Err(ClientError::Config(format!(
                "endpoint '{member}' is not defined"
            )));
        }
        let group_ref = self
            .groups
            .iter_mut()
            .find(|g| g.name == group)
            .ok_or_else(|| ClientError::Config(format!("group '{group}' is not defined")))?;
        if group_ref.kind != ProxyGroupKind::Select {
            return Err(ClientError::Config(format!(
                "group '{group}' is a {} group and does not accept manual selection",
                group_ref.kind.as_str()
            )));
        }
        if !group_ref.members.iter().any(|m| m == member) {
            return Err(ClientError::Config(format!(
                "group '{group}' has no member '{member}'"
            )));
        }
        group_ref.selected = Some(member.to_string());
        Ok(())
    }

    pub fn set_active_mode(&mut self, name: &str) -> Result<()> {
        if !self.modes.iter().any(|m| m.name == name) {
            return Err(ClientError::Config(format!("mode '{name}' is not defined")));
        }
        self.active_mode = name.to_string();
        Ok(())
    }

    pub fn upsert_script(&mut self, script: Script) -> Result<()> {
        script.validate()?;
        for rule in &script.rules {
            if let Some(RuleAction::Proxy { name }) = rule_action(rule) {
                self.resolve_proxy_target(name).map_err(|_| {
                    ClientError::Config(format!(
                        "script '{}' references unknown proxy '{}'",
                        script.name, name
                    ))
                })?;
            }
        }
        if let Some(existing) = self.scripts.iter_mut().find(|s| s.name == script.name) {
            *existing = script;
        } else {
            self.scripts.push(script);
        }
        Ok(())
    }

    pub fn remove_script(&mut self, name: &str) -> Result<()> {
        if let Some(mode) = self
            .modes
            .iter()
            .find(|m| m.script.as_deref() == Some(name))
        {
            return Err(ClientError::Config(format!(
                "script '{name}' is still used by mode '{}'",
                mode.name
            )));
        }
        self.scripts.retain(|s| s.name != name);
        Ok(())
    }

    pub fn upsert_user_mode(&mut self, mode: Mode) -> Result<()> {
        if mode.kind != ModeKind::User {
            return Err(ClientError::Config(
                "upsert_user_mode only accepts user-kind modes".into(),
            ));
        }
        if mode.name.trim().is_empty() {
            return Err(ClientError::Config("mode name must not be empty".into()));
        }
        if matches!(
            mode.name.as_str(),
            BUILTIN_GLOBAL | BUILTIN_RULE | BUILTIN_DIRECT
        ) {
            return Err(ClientError::Config(format!(
                "'{}' is a built-in mode name and cannot be redefined",
                mode.name
            )));
        }
        if let Some(name) = &mode.proxy {
            self.resolve_proxy_target(name).map_err(|_| {
                ClientError::Config(format!(
                    "mode '{}' references unknown proxy '{}'",
                    mode.name, name
                ))
            })?;
        } else {
            return Err(ClientError::Config(format!(
                "user mode '{}' must specify a proxy",
                mode.name
            )));
        }
        if let Some(script_name) = &mode.script {
            if !self.scripts.iter().any(|s| &s.name == script_name) {
                return Err(ClientError::Config(format!(
                    "mode '{}' references unknown script '{}'",
                    mode.name, script_name
                )));
            }
        }
        if let Some(existing) = self.modes.iter_mut().find(|m| m.name == mode.name) {
            if existing.kind != ModeKind::User {
                return Err(ClientError::Config(format!(
                    "cannot overwrite built-in mode '{}'",
                    mode.name
                )));
            }
            *existing = mode;
        } else {
            self.modes.push(mode);
        }
        Ok(())
    }

    pub fn remove_user_mode(&mut self, name: &str) -> Result<()> {
        let pos = self
            .modes
            .iter()
            .position(|m| m.name == name)
            .ok_or_else(|| ClientError::Config(format!("mode '{name}' is not defined")))?;
        if self.modes[pos].kind != ModeKind::User {
            return Err(ClientError::Config(format!(
                "cannot remove built-in mode '{name}'"
            )));
        }
        if self.active_mode == name {
            self.active_mode = BUILTIN_GLOBAL.to_string();
        }
        self.modes.remove(pos);
        Ok(())
    }

    pub fn active_selection_name(&self) -> &str {
        match &self.active {
            ActiveSelection::Endpoint { name } => name,
            ActiveSelection::Group { name } => name,
        }
    }

    pub fn resolve_proxy_target(&self, name: &str) -> Result<&NamedEndpoint> {
        if let Some(ep) = self.endpoints.iter().find(|e| e.name == name) {
            return Ok(ep);
        }
        let group =
            self.groups.iter().find(|g| g.name == name).ok_or_else(|| {
                ClientError::Config(format!("proxy target '{name}' is not defined"))
            })?;
        let selected = group
            .selected
            .as_deref()
            .ok_or_else(|| ClientError::Config(format!("group '{name}' has no selected member")))?;
        self.find_endpoint(selected)
    }

    pub fn decide(&self, target_host: &str) -> Decision {
        let mode = match self.modes.iter().find(|m| m.name == self.active_mode) {
            Some(m) => m,
            None => return Decision::Proxy(self.active_selection_name().to_string()),
        };
        match mode.kind {
            ModeKind::Direct => Decision::Direct,
            ModeKind::Global => Decision::Proxy(
                mode.proxy
                    .clone()
                    .unwrap_or_else(|| self.active_selection_name().to_string()),
            ),
            ModeKind::Rule | ModeKind::User => {
                if let Some(script_name) = &mode.script {
                    if let Some(script) = self.scripts.iter().find(|s| &s.name == script_name) {
                        let resolved_ips = if router::has_ip_cidr_rule(&script.rules) {
                            match crate::dns::resolve(target_host, &self.dns) {
                                Ok(ips) => ips,
                                Err(error) => {
                                    tracing::debug!(
                                        ?error,
                                        target_host,
                                        script = %script.name,
                                        "DNS resolution for rule evaluation failed"
                                    );
                                    Vec::new()
                                }
                            }
                        } else {
                            Vec::new()
                        };
                        if let Some(action) =
                            router::evaluate_with_ips(&script.rules, target_host, &resolved_ips)
                        {
                            return action.to_decision();
                        }
                    }
                }
                Decision::Proxy(
                    mode.proxy
                        .clone()
                        .unwrap_or_else(|| self.active_selection_name().to_string()),
                )
            }
        }
    }

    pub fn with_local_port(mut self, port: u16) -> Result<Self> {
        self.local.port = port;
        self.validate()?;
        Ok(self)
    }
}

pub fn default_config() -> ClientConfig {
    ClientConfig::single_server(
        "default",
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 443,
            endpoint: Endpoint::default(),
        },
        LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 1080,
            allow_socks: true,
            allow_http: true,
        },
    )
}

pub fn config_example() -> String {
    r#"[[endpoints]]
name = "default"
host = "127.0.0.1"
port = 443

[endpoints.proxy]
type = "vless"
uuid = "12345678-1234-1234-1234-123456789abc"
flow = ""

[endpoints.transport]
type = "raw"

[endpoints.outer-security]
type = "none"

[active]
type = "endpoint"
name = "default"

[local]
host = "127.0.0.1"
port = 1080

# Sample alternates — uncomment one.
#
# [[endpoints.proxy]]
# type = "trojan"
# password = "change-this-password"
#
# [[endpoints.proxy]]
# type = "mixed"
# username = "admin"
# password = "change-this-password"
#
# [[endpoints.transport]]
# type = "websocket"
# path = "/ws"
# host = "example.com"
#
# [[endpoints.outer-security]]
# type = "tls"
# server-name = "example.com"
# insecure-skip-verify = false
# alpn = ["h2", "http/1.1"]
#
# Add a select group to route through it:
# [[groups]]
# name = "auto"
# kind = "select"
# members = ["default"]
# selected = "default"
#
# [active]
# type = "group"
# name = "auto"
"#
    .to_string()
}

pub(crate) fn validate_host(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ClientError::Config(format!("{name} is required")));
    }
    Ok(())
}

pub(crate) fn validate_port(value: u16, name: &str) -> Result<()> {
    if value == 0 {
        return Err(ClientError::Config(format!(
            "{name} must be greater than zero"
        )));
    }
    Ok(())
}

fn rule_action(rule: &Rule) -> Option<&RuleAction> {
    match rule {
        Rule::Domain { action, .. }
        | Rule::DomainSuffix { action, .. }
        | Rule::DomainKeyword { action, .. }
        | Rule::IpCidr { action, .. }
        | Rule::GeoIp { action, .. }
        | Rule::Match { action } => Some(action),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(contents: &str, extension: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wrongcl-config-test-{}-{}.{}",
            std::process::id(),
            rand::random::<u64>(),
            extension,
        ));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn loads_json_config_from_file() {
        let path = write_temp_config(
            r#"{
  "endpoints": [
    {
      "name": "default",
      "host": "127.0.0.1",
      "port": 443,
      "proxy": {
        "type": "vless",
        "uuid": "12345678-1234-1234-1234-123456789abc",
        "flow": ""
      },
      "transport": {"type": "raw"},
      "outer-security": {"type": "none"}
    }
  ],
  "active": {"type": "endpoint", "name": "default"},
  "local": {"host": "127.0.0.1", "port": 1080}
}"#,
            "json",
        );

        let config = ClientConfig::from_file(path).unwrap();
        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(config.endpoints[0].server.host, "127.0.0.1");
        assert_eq!(config.local.port, 1080);
        let active = config.resolve_active_endpoint().unwrap();
        assert_eq!(active.name, "default");
    }

    #[test]
    fn loads_legacy_document_json() {
        let json = r#"{
  "server": {
    "host": "127.0.0.1",
    "port": 443,
    "proxy": {"type": "vless", "uuid": "12345678-1234-1234-1234-123456789abc", "flow": ""},
    "transport": {"type": "raw"},
    "outer-security": {"type": "none"}
  },
  "local": {"host": "127.0.0.1", "port": 1080}
}"#;
        let config = ClientConfig::from_legacy_document_json(json).unwrap();
        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(config.endpoints[0].name, "default");
        assert_eq!(config.endpoints[0].server.port, 443);
        let active = config.resolve_active_endpoint().unwrap();
        assert_eq!(active.server.host, "127.0.0.1");
    }

    #[test]
    fn loads_fragment_transport_from_toml() {
        let path = write_temp_config(
            r#"
[[endpoints]]
name = "frag"
host = "127.0.0.1"
port = 443

[endpoints.proxy]
type = "vless"
uuid = "12345678-1234-1234-1234-123456789abc"

[endpoints.transport]
type = "fragment"
length-min = 1
length-max = 4
packets-from = 1
packets-to = 2

[endpoints.outer-security]
type = "none"

[active]
type = "endpoint"
name = "frag"

[local]
host = "127.0.0.1"
port = 1080
"#,
            "toml",
        );

        let config = ClientConfig::from_file(path).unwrap();
        let active = config.resolve_active_endpoint().unwrap();
        match &active.server.endpoint.transport {
            Transport::Fragment(opts) => {
                assert_eq!(opts.length_min, 1);
                assert_eq!(opts.length_max, 4);
                assert_eq!(opts.packets_to, 2);
            }
            other => panic!("expected Fragment transport, got {other:?}"),
        }
    }

    #[test]
    fn validates_duplicate_endpoint_name() {
        let config = ClientConfig {
            endpoints: vec![
                NamedEndpoint {
                    name: "a".into(),
                    server: ServerConfig {
                        host: "1.1.1.1".into(),
                        port: 443,
                        endpoint: Endpoint::default(),
                    },
                },
                NamedEndpoint {
                    name: "a".into(),
                    server: ServerConfig {
                        host: "2.2.2.2".into(),
                        port: 443,
                        endpoint: Endpoint::default(),
                    },
                },
            ],
            groups: vec![],
            scripts: vec![],
            modes: default_modes(),
            active_mode: default_active_mode(),
            active: ActiveSelection::Endpoint { name: "a".into() },
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 1080,
                allow_socks: true,
                allow_http: true,
            },
            dns: DnsSettings::default(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_group_with_unknown_member() {
        let config = ClientConfig {
            endpoints: vec![NamedEndpoint {
                name: "a".into(),
                server: ServerConfig {
                    host: "1.1.1.1".into(),
                    port: 443,
                    endpoint: Endpoint::default(),
                },
            }],
            groups: vec![ProxyGroup {
                name: "auto".into(),
                kind: ProxyGroupKind::Select,
                members: vec!["a".into(), "ghost".into()],
                selected: Some("a".into()),
            }],
            scripts: vec![],
            modes: default_modes(),
            active_mode: default_active_mode(),
            active: ActiveSelection::Endpoint { name: "a".into() },
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 1080,
                allow_socks: true,
                allow_http: true,
            },
            dns: DnsSettings::default(),
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ghost"), "unexpected message: {msg}");
    }

    #[test]
    fn resolves_active_via_group() {
        let mut config = ClientConfig {
            endpoints: vec![
                NamedEndpoint {
                    name: "a".into(),
                    server: ServerConfig {
                        host: "1.1.1.1".into(),
                        port: 443,
                        endpoint: Endpoint::default(),
                    },
                },
                NamedEndpoint {
                    name: "b".into(),
                    server: ServerConfig {
                        host: "2.2.2.2".into(),
                        port: 443,
                        endpoint: Endpoint::default(),
                    },
                },
            ],
            groups: vec![ProxyGroup {
                name: "auto".into(),
                kind: ProxyGroupKind::Select,
                members: vec!["a".into(), "b".into()],
                selected: Some("a".into()),
            }],
            scripts: vec![],
            modes: default_modes(),
            active_mode: default_active_mode(),
            active: ActiveSelection::Group {
                name: "auto".into(),
            },
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 1080,
                allow_socks: true,
                allow_http: true,
            },
            dns: DnsSettings::default(),
        };
        assert_eq!(config.resolve_active_endpoint().unwrap().name, "a");
        config.select_group_member("auto", "b").unwrap();
        assert_eq!(config.resolve_active_endpoint().unwrap().name, "b");
    }

    #[test]
    fn select_member_must_be_in_group() {
        let mut config = ClientConfig {
            endpoints: vec![
                NamedEndpoint {
                    name: "a".into(),
                    server: ServerConfig {
                        host: "1.1.1.1".into(),
                        port: 443,
                        endpoint: Endpoint::default(),
                    },
                },
                NamedEndpoint {
                    name: "b".into(),
                    server: ServerConfig {
                        host: "2.2.2.2".into(),
                        port: 443,
                        endpoint: Endpoint::default(),
                    },
                },
            ],
            groups: vec![ProxyGroup {
                name: "auto".into(),
                kind: ProxyGroupKind::Select,
                members: vec!["a".into()],
                selected: Some("a".into()),
            }],
            scripts: vec![],
            modes: default_modes(),
            active_mode: default_active_mode(),
            active: ActiveSelection::Group {
                name: "auto".into(),
            },
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 1080,
                allow_socks: true,
                allow_http: true,
            },
            dns: DnsSettings::default(),
        };
        assert!(config.select_group_member("auto", "b").is_err());
    }

    #[test]
    fn decide_resolves_domain_for_ip_cidr_rules_with_configured_dns() {
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = socket.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; 1500];
            for answer_count in [1u16, 0u16] {
                let (n, peer) = socket.recv_from(&mut buf).unwrap();
                let id = [buf[0], buf[1]];
                let mut response = Vec::new();
                response.extend_from_slice(&id);
                response.extend_from_slice(&0x8180u16.to_be_bytes());
                response.extend_from_slice(&1u16.to_be_bytes());
                response.extend_from_slice(&answer_count.to_be_bytes());
                response.extend_from_slice(&0u16.to_be_bytes());
                response.extend_from_slice(&0u16.to_be_bytes());
                response.extend_from_slice(&buf[12..n]);
                if answer_count == 1 {
                    response.extend_from_slice(&[0xc0, 0x0c]);
                    response.extend_from_slice(&1u16.to_be_bytes());
                    response.extend_from_slice(&1u16.to_be_bytes());
                    response.extend_from_slice(&60u32.to_be_bytes());
                    response.extend_from_slice(&4u16.to_be_bytes());
                    response.extend_from_slice(&[9, 9, 9, 9]);
                }
                socket.send_to(&response, peer).unwrap();
            }
        });

        let mut config = ClientConfig::raw_vless(
            "127.0.0.1",
            443,
            "12345678-1234-1234-1234-123456789abc",
            "127.0.0.1",
            1080,
        )
        .unwrap();
        config.dns = DnsSettings {
            backend: crate::dns::DnsBackend::Udp {
                server: server_addr,
            },
        };
        config
            .upsert_script(Script {
                name: "ip-split".into(),
                rules: vec![Rule::IpCidr {
                    cidr: "9.9.9.0/24".into(),
                    action: RuleAction::Direct,
                }],
            })
            .unwrap();
        config
            .upsert_user_mode(Mode {
                name: "resolved".into(),
                kind: ModeKind::User,
                proxy: Some("default".into()),
                script: Some("ip-split".into()),
            })
            .unwrap();
        config.set_active_mode("resolved").unwrap();

        assert_eq!(config.decide("dns.example"), Decision::Direct);
        handle.join().unwrap();
    }
}
