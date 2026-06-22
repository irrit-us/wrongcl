#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "linux")]
use serde::Deserialize;
use serde::Serialize;

use crate::error::{ClientError, Result};

#[cfg(target_os = "linux")]
mod linux_runtime;

#[cfg(target_os = "linux")]
const CAP_NET_ADMIN_BIT: u32 = 12;
#[cfg(target_os = "linux")]
const DEFAULT_TUN_NAME: &str = "wrongcl-tun0";
#[cfg(target_os = "linux")]
const DEFAULT_TUN_MTU: u32 = 1400;
#[cfg(target_os = "linux")]
const DEFAULT_TUN_CIDR: &str = "198.18.0.1/15";
#[cfg(target_os = "linux")]
const ENV_TUN_DEVICE: &str = "WRONGCL_TUN_DEVICE";
#[cfg(target_os = "linux")]
const ENV_IP_BIN: &str = "WRONGCL_IP_BIN";
#[cfg(target_os = "linux")]
const ENV_FORCE_CAP: &str = "WRONGCL_FORCE_CAP_NET_ADMIN";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct TunStatus {
    pub supported: bool,
    pub enabled: bool,
    pub disabled_reason: String,
    pub needs_privileges: bool,
    pub preparable: bool,
    pub platform: &'static str,
}

impl TunStatus {
    fn unsupported(
        platform: &'static str,
        disabled_reason: impl Into<String>,
        needs_privileges: bool,
    ) -> Self {
        Self {
            supported: false,
            enabled: false,
            disabled_reason: disabled_reason.into(),
            needs_privileges,
            preparable: false,
            platform,
        }
    }

    #[cfg(target_os = "linux")]
    fn prepared(platform: &'static str, disabled_reason: impl Into<String>) -> Self {
        Self {
            supported: true,
            enabled: true,
            disabled_reason: disabled_reason.into(),
            needs_privileges: false,
            preparable: true,
            platform,
        }
    }

    #[cfg(target_os = "linux")]
    fn setup_available(platform: &'static str, disabled_reason: impl Into<String>) -> Self {
        Self {
            supported: true,
            enabled: false,
            disabled_reason: disabled_reason.into(),
            needs_privileges: false,
            preparable: true,
            platform,
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug, Deserialize)]
struct TunEnableConfig {
    #[serde(default = "default_tun_name")]
    interface_name: String,
    #[serde(default = "default_tun_mtu")]
    mtu: u32,
    #[serde(default = "default_tun_cidr")]
    address_cidr: String,
    #[serde(default)]
    routes: Vec<String>,
    #[serde(default = "default_proxy_host")]
    proxy_host: String,
    #[serde(default = "default_proxy_port")]
    proxy_port: u16,
}

#[cfg(target_os = "linux")]
fn default_tun_name() -> String {
    DEFAULT_TUN_NAME.to_string()
}

#[cfg(target_os = "linux")]
fn default_tun_mtu() -> u32 {
    DEFAULT_TUN_MTU
}

#[cfg(target_os = "linux")]
fn default_tun_cidr() -> String {
    DEFAULT_TUN_CIDR.to_string()
}

#[cfg(target_os = "linux")]
fn default_proxy_host() -> String {
    "127.0.0.1".to_string()
}

#[cfg(target_os = "linux")]
fn default_proxy_port() -> u16 {
    1080
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct TunState {
    interface_name: Option<String>,
    runtime: Option<linux_runtime::LinuxTunRuntimeHandle>,
}

#[cfg(target_os = "linux")]
fn tun_state() -> &'static Mutex<TunState> {
    static STATE: OnceLock<Mutex<TunState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TunState::default()))
}

#[cfg(target_os = "linux")]
pub fn current_status() -> TunStatus {
    linux_status()
}

#[cfg(target_os = "windows")]
pub fn current_status() -> TunStatus {
    TunStatus::unsupported(
        "windows",
        "Windows TUN backend is not implemented in wrongcl yet.",
        false,
    )
}

#[cfg(target_os = "macos")]
pub fn current_status() -> TunStatus {
    TunStatus::unsupported(
        "macos",
        "macOS TUN backend is not implemented in wrongcl yet.",
        false,
    )
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn current_status() -> TunStatus {
    TunStatus::unsupported(
        std::env::consts::OS,
        "TUN backend is not implemented for this platform.",
        false,
    )
}

#[cfg(target_os = "linux")]
pub fn enable(config_json: &str) -> Result<TunStatus> {
    linux_enable(config_json)
}

#[cfg(not(target_os = "linux"))]
pub fn enable(_config_json: &str) -> Result<TunStatus> {
    Err(ClientError::Config(current_status().disabled_reason))
}

#[cfg(target_os = "linux")]
pub fn disable() -> TunStatus {
    linux_disable()
}

#[cfg(not(target_os = "linux"))]
pub fn disable() -> TunStatus {
    current_status()
}

#[cfg(target_os = "linux")]
fn linux_status() -> TunStatus {
    let interface_name = current_interface_name();
    let driver_present = tun_device_path().exists();
    let has_privileges = read_linux_cap_net_admin().unwrap_or(false);
    let interface_exists = interface_name
        .as_deref()
        .is_some_and(interface_exists_linux);
    let runtime_running = runtime_running_linux();

    if !driver_present {
        return TunStatus::unsupported(
            "linux",
            "Linux TUN device is unavailable: /dev/net/tun was not found.",
            false,
        );
    }
    if runtime_running && interface_exists {
        return TunStatus {
            supported: true,
            enabled: true,
            disabled_reason: "Linux TUN interface and routed bridge are active.".into(),
            needs_privileges: false,
            preparable: true,
            platform: "linux",
        };
    }
    if interface_exists {
        return TunStatus::prepared(
            "linux",
            "Linux TUN interface is prepared, but the routed bridge is not attached yet.",
        );
    }
    if !has_privileges {
        return TunStatus::unsupported(
            "linux",
            "Needs privileges: CAP_NET_ADMIN is required for TUN setup.",
            true,
        );
    }

    TunStatus::setup_available(
        "linux",
        "Linux TUN interface setup is available, but the routed dataplane is not attached yet.",
    )
}

#[cfg(target_os = "linux")]
fn linux_enable(config_json: &str) -> Result<TunStatus> {
    let config = parse_enable_config(config_json)?;
    let status = linux_status();
    if status.enabled {
        return Ok(status);
    }
    if status.needs_privileges || !tun_device_path().exists() {
        return Err(ClientError::Config(status.disabled_reason));
    }

    run_ip_command(&[
        "tuntap",
        "add",
        "mode",
        "tun",
        "name",
        &config.interface_name,
    ])?;
    if let Err(error) = run_ip_command(&[
        "addr",
        "replace",
        &config.address_cidr,
        "dev",
        &config.interface_name,
    ]) {
        let _ = run_ip_command(&["link", "delete", "dev", &config.interface_name]);
        return Err(error);
    }
    if let Err(error) = run_ip_command(&[
        "link",
        "set",
        "dev",
        &config.interface_name,
        "mtu",
        &config.mtu.to_string(),
        "up",
    ]) {
        let _ = run_ip_command(&["link", "delete", "dev", &config.interface_name]);
        return Err(error);
    }
    for route in &config.routes {
        if let Err(error) =
            run_ip_command(&["route", "replace", route, "dev", &config.interface_name])
        {
            let _ = run_ip_command(&["link", "delete", "dev", &config.interface_name]);
            return Err(error);
        }
    }
    let runtime = match linux_runtime::LinuxTunRuntimeHandle::start(config.clone()) {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = run_ip_command(&["link", "delete", "dev", &config.interface_name]);
            return Err(error);
        }
    };
    if !runtime.is_running() {
        let _ = run_ip_command(&["link", "delete", "dev", &config.interface_name]);
        return Err(ClientError::Config(
            "Linux TUN runtime exited during startup".into(),
        ));
    }
    set_tun_state(config.interface_name, runtime);
    Ok(linux_status())
}

#[cfg(target_os = "linux")]
fn linux_disable() -> TunStatus {
    let Some(interface_name) = current_interface_name() else {
        return linux_status();
    };
    terminate_runtime_linux();
    if interface_exists_linux(&interface_name) {
        let _ = run_ip_command(&["link", "delete", "dev", &interface_name]);
    }
    clear_current_interface_name();
    linux_status()
}

#[cfg(target_os = "linux")]
fn parse_enable_config(config_json: &str) -> Result<TunEnableConfig> {
    if config_json.trim().is_empty() || config_json.trim() == "{}" {
        return Ok(TunEnableConfig {
            interface_name: default_tun_name(),
            mtu: default_tun_mtu(),
            address_cidr: default_tun_cidr(),
            routes: Vec::new(),
            proxy_host: default_proxy_host(),
            proxy_port: default_proxy_port(),
        });
    }
    serde_json::from_str(config_json)
        .map_err(|error| ClientError::Config(format!("invalid TUN config JSON: {error}")))
}

#[cfg(target_os = "linux")]
fn current_interface_name() -> Option<String> {
    tun_state()
        .lock()
        .ok()
        .and_then(|guard| guard.interface_name.clone())
}

#[cfg(target_os = "linux")]
fn set_tun_state(name: String, runtime: linux_runtime::LinuxTunRuntimeHandle) {
    if let Ok(mut guard) = tun_state().lock() {
        guard.interface_name = Some(name);
        guard.runtime = Some(runtime);
    }
}

#[cfg(target_os = "linux")]
fn clear_current_interface_name() {
    if let Ok(mut guard) = tun_state().lock() {
        guard.interface_name = None;
        if let Some(mut runtime) = guard.runtime.take() {
            runtime.stop();
        }
    }
}

#[cfg(target_os = "linux")]
fn runtime_running_linux() -> bool {
    let Ok(guard) = tun_state().lock() else {
        return false;
    };
    guard
        .runtime
        .as_ref()
        .is_some_and(|runtime| runtime.is_running())
}

#[cfg(target_os = "linux")]
fn interface_exists_linux(name: &str) -> bool {
    run_ip_command(&["link", "show", "dev", name]).is_ok()
}

#[cfg(target_os = "linux")]
fn ip_binary() -> String {
    env::var(ENV_IP_BIN).unwrap_or_else(|_| "ip".into())
}

#[cfg(target_os = "linux")]
fn tun_device_path() -> PathBuf {
    env::var(ENV_TUN_DEVICE)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/dev/net/tun"))
}

#[cfg(target_os = "linux")]
fn run_ip_command(args: &[&str]) -> Result<()> {
    let output = Command::new(ip_binary())
        .args(args)
        .output()
        .map_err(|error| ClientError::Io(std::io::Error::new(error.kind(), error.to_string())))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit status {}", output.status)
    };
    Err(ClientError::Config(format!(
        "ip {} failed: {}",
        args.join(" "),
        detail
    )))
}

#[cfg(target_os = "linux")]
fn terminate_runtime_linux() {
    if let Ok(mut guard) = tun_state().lock() {
        if let Some(mut runtime) = guard.runtime.take() {
            runtime.stop();
        }
    }
}

#[cfg(target_os = "linux")]
fn read_linux_cap_net_admin() -> Result<bool> {
    if let Ok(value) = env::var(ENV_FORCE_CAP) {
        return Ok(matches!(value.as_str(), "1" | "true" | "TRUE"));
    }
    let status = fs::read_to_string("/proc/self/status")?;
    Ok(parse_cap_net_admin_from_proc_status(&status).unwrap_or(false))
}

#[cfg(target_os = "linux")]
fn parse_cap_net_admin_from_proc_status(status: &str) -> Option<bool> {
    let line = status.lines().find(|line| line.starts_with("CapEff:"))?;
    let raw = line.split_whitespace().nth(1)?;
    let caps = u64::from_str_radix(raw, 16).ok()?;
    Some((caps & (1u64 << CAP_NET_ADMIN_BIT)) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disable_returns_current_status() {
        assert_eq!(disable(), current_status());
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use super::*;

        #[test]
        fn parses_cap_net_admin_from_proc_status() {
            let status = "Name:\twrongcl\nCapEff:\t0000000000001000\n";
            assert_eq!(parse_cap_net_admin_from_proc_status(status), Some(true));
        }

        #[test]
        fn parses_missing_cap_net_admin_from_proc_status() {
            let status = "Name:\twrongcl\nCapEff:\t0000000000000000\n";
            assert_eq!(parse_cap_net_admin_from_proc_status(status), Some(false));
        }
    }
}
