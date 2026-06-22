#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::sync::{Mutex, OnceLock};

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
use serde::Deserialize;
use serde::Serialize;

use crate::error::{ClientError, Result};

#[cfg(target_os = "linux")]
mod linux_runtime;
#[cfg(target_os = "macos")]
mod macos_runtime;
#[cfg(target_os = "windows")]
mod windows_runtime;

#[cfg(target_os = "linux")]
const CAP_NET_ADMIN_BIT: u32 = 12;
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
const DEFAULT_TUN_NAME: &str = "wrongcl-tun0";
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
const DEFAULT_TUN_MTU: u32 = 1400;
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
const DEFAULT_TUN_CIDR: &str = "198.18.0.1/15";
#[cfg(target_os = "linux")]
const ENV_TUN_DEVICE: &str = "WRONGCL_TUN_DEVICE";
#[cfg(target_os = "linux")]
const ENV_IP_BIN: &str = "WRONGCL_IP_BIN";
#[cfg(target_os = "linux")]
const ENV_FORCE_CAP: &str = "WRONGCL_FORCE_CAP_NET_ADMIN";
#[cfg(target_os = "windows")]
const WINDOWS_TUN_SERVICE_NAME: &str = "wintun";
#[cfg(target_os = "windows")]
const WINDOWS_TUN_DLL_NAME: &str = "wintun.dll";
#[cfg(target_os = "windows")]
const ENV_WINTUN_DLL: &str = "WRONGCL_WINTUN_DLL";

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
    #[cfg(not(target_os = "windows"))]
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

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
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

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn default_tun_name() -> String {
    DEFAULT_TUN_NAME.to_string()
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn default_tun_mtu() -> u32 {
    DEFAULT_TUN_MTU
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn default_tun_cidr() -> String {
    DEFAULT_TUN_CIDR.to_string()
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn default_proxy_host() -> String {
    "127.0.0.1".to_string()
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
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

#[cfg(target_os = "windows")]
#[derive(Default)]
struct TunState {
    interface_name: Option<String>,
    runtime: Option<windows_runtime::WindowsTunRuntimeHandle>,
}

#[cfg(target_os = "windows")]
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
    windows_status()
}

#[cfg(target_os = "macos")]
pub fn current_status() -> TunStatus {
    macos_status()
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

#[cfg(target_os = "windows")]
pub fn enable(config_json: &str) -> Result<TunStatus> {
    windows_enable(config_json)
}

#[cfg(target_os = "macos")]
pub fn enable(config_json: &str) -> Result<TunStatus> {
    macos_enable(config_json)
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn enable(_config_json: &str) -> Result<TunStatus> {
    Err(ClientError::Config(current_status().disabled_reason))
}

#[cfg(target_os = "linux")]
pub fn disable() -> TunStatus {
    linux_disable()
}

#[cfg(target_os = "windows")]
pub fn disable() -> TunStatus {
    windows_disable()
}

#[cfg(target_os = "macos")]
pub fn disable() -> TunStatus {
    macos_disable()
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn disable() -> TunStatus {
    current_status()
}

#[cfg(target_os = "macos")]
fn macos_status() -> TunStatus {
    TunStatus::unsupported(
        "macos",
        "macOS TUN host wiring is planned but not implemented in wrongcl yet. Finish the native utun path and validate it on a real macOS host before enabling this control.",
        false,
    )
}

#[cfg(target_os = "macos")]
fn macos_enable(config_json: &str) -> Result<TunStatus> {
    let config = parse_enable_config(config_json)?;
    let _ = macos_runtime::MacosTunRuntimeHandle::start(config)?;
    Ok(macos_status())
}

#[cfg(target_os = "macos")]
fn macos_disable() -> TunStatus {
    macos_status()
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

#[cfg(target_os = "windows")]
fn windows_status() -> TunStatus {
    if runtime_running_windows() {
        return TunStatus {
            supported: true,
            enabled: true,
            disabled_reason: "Windows TUN interface and routed bridge are active.".into(),
            needs_privileges: false,
            preparable: true,
            platform: "windows",
        };
    }

    let dll_present = windows_wintun_dll_path().is_some();
    let driver_present = windows_wintun_service_present().unwrap_or(false);
    let is_admin = windows_is_elevated().unwrap_or(false);

    let (supported, disabled_reason, needs_privileges, preparable) =
        windows_tun_readiness(dll_present, driver_present, is_admin);

    TunStatus {
        supported,
        enabled: false,
        disabled_reason,
        needs_privileges,
        preparable,
        platform: "windows",
    }
}

#[cfg(target_os = "windows")]
fn windows_enable(config_json: &str) -> Result<TunStatus> {
    let config = parse_enable_config(config_json)?;
    let status = windows_status();
    if status.enabled {
        return Ok(status);
    }
    if !status.supported || status.needs_privileges {
        return Err(ClientError::Config(status.disabled_reason));
    }

    let runtime = windows_runtime::WindowsTunRuntimeHandle::start(config)?;
    let interface_name = runtime.interface_name().to_string();
    set_tun_state_windows(interface_name, runtime);
    Ok(windows_status())
}

#[cfg(target_os = "windows")]
fn windows_disable() -> TunStatus {
    clear_current_interface_name_windows();
    windows_status()
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

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
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

#[cfg(target_os = "windows")]
fn windows_tun_readiness(
    dll_present: bool,
    driver_present: bool,
    is_admin: bool,
) -> (bool, String, bool, bool) {
    if !dll_present && !driver_present {
        return (
            false,
            "Windows TUN prerequisites are incomplete: `wintun.dll` was not found and the Wintun driver service is not installed.".into(),
            false,
            false,
        );
    }
    if !dll_present {
        return (
            false,
            "Windows TUN is blocked: `wintun.dll` was not found near the app or in the current workspace.".into(),
            false,
            false,
        );
    }
    if !driver_present {
        return (
            false,
            "Windows TUN is blocked: the Wintun driver service is not installed or not visible to the current host.".into(),
            false,
            false,
        );
    }
    if !is_admin {
        return (
            false,
            "Needs privileges: run wrongcl with Administrator rights before preparing the Windows TUN interface.".into(),
            true,
            false,
        );
    }
    (
        true,
        "Windows TUN runtime is ready to start on this host.".into(),
        false,
        true,
    )
}

#[cfg(target_os = "windows")]
fn set_tun_state_windows(name: String, runtime: windows_runtime::WindowsTunRuntimeHandle) {
    if let Ok(mut guard) = tun_state().lock() {
        guard.interface_name = Some(name);
        guard.runtime = Some(runtime);
    }
}

#[cfg(target_os = "windows")]
fn clear_current_interface_name_windows() {
    if let Ok(mut guard) = tun_state().lock() {
        guard.interface_name = None;
        if let Some(mut runtime) = guard.runtime.take() {
            runtime.stop();
        }
    }
}

#[cfg(target_os = "windows")]
fn runtime_running_windows() -> bool {
    let Ok(guard) = tun_state().lock() else {
        return false;
    };
    guard
        .runtime
        .as_ref()
        .is_some_and(|runtime| runtime.is_running())
}

#[cfg(target_os = "windows")]
fn windows_wintun_dll_path() -> Option<PathBuf> {
    windows_wintun_candidate_paths()
        .into_iter()
        .find(|path| path.exists())
}

#[cfg(target_os = "windows")]
fn windows_wintun_candidate_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var(ENV_WINTUN_DLL) {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            candidates.push(dir.join(WINDOWS_TUN_DLL_NAME));
        }
    }
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join(WINDOWS_TUN_DLL_NAME));
        candidates.push(current_dir.join("windows").join(WINDOWS_TUN_DLL_NAME));
        candidates.push(current_dir.join("runner").join(WINDOWS_TUN_DLL_NAME));
    }
    dedupe_paths(candidates)
}

#[cfg(target_os = "windows")]
fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !deduped.iter().any(|candidate| same_path(candidate, &path)) {
            deduped.push(path);
        }
    }
    deduped
}

#[cfg(target_os = "windows")]
fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(target_os = "windows")]
fn windows_wintun_service_present() -> Result<bool> {
    let output = Command::new("sc.exe")
        .args(["query", WINDOWS_TUN_SERVICE_NAME])
        .output();
    let output = output
        .map_err(|error| ClientError::Io(std::io::Error::new(error.kind(), error.to_string())))?;
    Ok(parse_windows_sc_query_service_present(
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    ))
}

#[cfg(target_os = "windows")]
fn parse_windows_sc_query_service_present(stdout: &str, stderr: &str) -> bool {
    let stdout_lower = stdout.to_ascii_lowercase();
    if stdout_lower.contains("service_name:") || stdout_lower.contains("state") {
        return true;
    }
    let stderr_lower = stderr.to_ascii_lowercase();
    !stderr_lower.contains("does not exist")
        && !stderr_lower.contains("openservice failed 1060")
        && !stdout_lower.contains("does not exist")
        && !stdout_lower.contains("openservice failed 1060")
}

#[cfg(target_os = "windows")]
fn windows_is_elevated() -> Result<bool> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
        ])
        .output();
    let output = output
        .map_err(|error| ClientError::Io(std::io::Error::new(error.kind(), error.to_string())))?;
    Ok(parse_windows_admin_check_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

#[cfg(target_os = "windows")]
fn parse_windows_admin_check_output(output: &str) -> bool {
    output
        .lines()
        .find_map(|line| {
            let value = line.trim();
            if value.eq_ignore_ascii_case("true") {
                Some(true)
            } else if value.eq_ignore_ascii_case("false") {
                Some(false)
            } else {
                None
            }
        })
        .unwrap_or(false)
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

    #[cfg(target_os = "windows")]
    mod windows {
        use super::*;

        #[test]
        fn parses_windows_admin_check_output() {
            assert!(parse_windows_admin_check_output("True\r\n"));
            assert!(!parse_windows_admin_check_output("False\r\n"));
            assert!(!parse_windows_admin_check_output("unexpected\r\n"));
        }

        #[test]
        fn parses_windows_sc_query_missing_service() {
            let stderr = "[SC] OpenService FAILED 1060:\r\nThe specified service does not exist as an installed service.\r\n";
            assert!(!parse_windows_sc_query_service_present("", stderr));
        }

        #[test]
        fn parses_windows_sc_query_present_service() {
            let stdout = "SERVICE_NAME: wintun\r\n        TYPE               : 1  KERNEL_DRIVER\r\n        STATE              : 4  RUNNING\r\n";
            assert!(parse_windows_sc_query_service_present(stdout, ""));
        }

        #[test]
        fn windows_tun_readiness_reports_missing_prerequisites() {
            let (supported, message, needs_privileges, preparable) =
                windows_tun_readiness(false, false, false);
            assert!(!supported);
            assert!(message.contains("prerequisites are incomplete"));
            assert!(!needs_privileges);
            assert!(!preparable);
        }

        #[test]
        fn windows_tun_readiness_reports_admin_requirement() {
            let (supported, message, needs_privileges, preparable) =
                windows_tun_readiness(true, true, false);
            assert!(!supported);
            assert!(message.contains("Administrator rights"));
            assert!(needs_privileges);
            assert!(!preparable);
        }

        #[test]
        fn windows_tun_readiness_marks_ready_host_as_prepairable() {
            let (supported, message, needs_privileges, preparable) =
                windows_tun_readiness(true, true, true);
            assert!(supported);
            assert!(message.contains("ready to start"));
            assert!(!needs_privileges);
            assert!(preparable);
        }
    }
}
