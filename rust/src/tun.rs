use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{ClientError, Result};

const CAP_NET_ADMIN_BIT: u32 = 12;
const DEFAULT_TUN_NAME: &str = "wrongcl-tun0";
const DEFAULT_TUN_MTU: u32 = 1400;
const DEFAULT_TUN_CIDR: &str = "198.18.0.1/15";
const ENV_TUN_DEVICE: &str = "WRONGCL_TUN_DEVICE";
const ENV_IP_BIN: &str = "WRONGCL_IP_BIN";
const ENV_FORCE_CAP: &str = "WRONGCL_FORCE_CAP_NET_ADMIN";
const ENV_TUN_HELPER_BIN: &str = "WRONGCL_TUN_HELPER_BIN";

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

fn default_tun_name() -> String {
    DEFAULT_TUN_NAME.to_string()
}

fn default_tun_mtu() -> u32 {
    DEFAULT_TUN_MTU
}

fn default_tun_cidr() -> String {
    DEFAULT_TUN_CIDR.to_string()
}

fn default_proxy_host() -> String {
    "127.0.0.1".to_string()
}

fn default_proxy_port() -> u16 {
    1080
}

#[derive(Default)]
struct TunState {
    interface_name: Option<String>,
    helper_child: Option<Child>,
    helper_config_path: Option<PathBuf>,
}

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
    let helper_running = helper_running_linux();

    if !driver_present {
        return TunStatus::unsupported(
            "linux",
            "Linux TUN device is unavailable: /dev/net/tun was not found.",
            false,
        );
    }
    if helper_running && interface_exists {
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
    let helper_config = write_helper_config(&config)?;
    let helper_binary = build_tun_helper()?;
    let mut child = spawn_tun_helper(&helper_binary, &helper_config)?;
    if let Some(status) = wait_for_helper_start(&mut child, Duration::from_secs(3))? {
        let _ = run_ip_command(&["link", "delete", "dev", &config.interface_name]);
        let _ = fs::remove_file(&helper_config);
        return Err(ClientError::Config(format!(
            "tun helper exited during startup: {status}"
        )));
    }
    set_tun_state(config.interface_name, child, helper_config);
    Ok(linux_status())
}

#[cfg(target_os = "linux")]
fn linux_disable() -> TunStatus {
    let Some(interface_name) = current_interface_name() else {
        return linux_status();
    };
    terminate_helper_linux();
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
fn set_tun_state(name: String, child: Child, config_path: PathBuf) {
    if let Ok(mut guard) = tun_state().lock() {
        guard.interface_name = Some(name);
        guard.helper_child = Some(child);
        guard.helper_config_path = Some(config_path);
    }
}

#[cfg(target_os = "linux")]
fn clear_current_interface_name() {
    if let Ok(mut guard) = tun_state().lock() {
        guard.interface_name = None;
        if let Some(path) = guard.helper_config_path.take() {
            let _ = fs::remove_file(path);
        }
        guard.helper_child = None;
    }
}

#[cfg(target_os = "linux")]
fn helper_running_linux() -> bool {
    let Ok(mut guard) = tun_state().lock() else {
        return false;
    };
    let Some(child) = guard.helper_child.as_mut() else {
        return false;
    };
    match child.try_wait() {
        Ok(None) => true,
        Ok(Some(_)) | Err(_) => {
            if let Some(path) = guard.helper_config_path.take() {
                let _ = fs::remove_file(path);
            }
            guard.helper_child = None;
            false
        }
    }
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
fn helper_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../helpers/tun-proxy-bridge")
}

#[cfg(target_os = "linux")]
fn helper_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tun-proxy-bridge")
}

#[cfg(target_os = "linux")]
fn helper_binary_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("tun-proxy-bridge"));
            candidates.push(parent.join("lib").join("tun-proxy-bridge"));
        }
    }
    candidates.push(helper_binary_path());
    candidates
}

#[cfg(target_os = "linux")]
fn build_tun_helper() -> Result<PathBuf> {
    if let Ok(path) = env::var(ENV_TUN_HELPER_BIN) {
        return Ok(PathBuf::from(path));
    }
    for candidate in helper_binary_candidates() {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    let helper_dir = helper_directory();
    let output = helper_binary_path();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let status = Command::new("go")
        .arg("build")
        .arg("-o")
        .arg(&output)
        .arg(".")
        .current_dir(&helper_dir)
        .status()
        .map_err(|error| ClientError::Io(std::io::Error::new(error.kind(), error.to_string())))?;
    if !status.success() {
        return Err(ClientError::Config(format!(
            "go build failed for {}",
            helper_dir.display()
        )));
    }
    Ok(output)
}

#[cfg(target_os = "linux")]
fn write_helper_config(config: &TunEnableConfig) -> Result<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!(
            "tun-proxy-bridge-{}-{}.json",
            std::process::id(),
            rand::random::<u64>()
        ));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::json!({
        "interface_name": config.interface_name,
        "mtu": config.mtu,
        "address_cidr": config.address_cidr,
        "stack_cidr": helper_stack_cidr(&config.address_cidr),
        "routes": helper_routes_for(config),
        "proxy_host": config.proxy_host,
        "proxy_port": config.proxy_port,
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).map_err(|error| {
            ClientError::Config(format!("serialize tun helper config: {error}"))
        })?,
    )?;
    Ok(path)
}

#[cfg(target_os = "linux")]
fn helper_routes_for(config: &TunEnableConfig) -> Vec<String> {
    let mut routes = vec!["0.0.0.0/0".into()];
    for route in &config.routes {
        if !routes.contains(route) {
            routes.push(route.clone());
        }
    }
    routes
}

#[cfg(target_os = "linux")]
fn helper_stack_cidr(address_cidr: &str) -> String {
    let Some((ip, prefix)) = address_cidr.split_once('/') else {
        return address_cidr.to_string();
    };
    let Ok(ipv4) = ip.parse::<std::net::Ipv4Addr>() else {
        return address_cidr.to_string();
    };
    let octets = ipv4.octets();
    let candidate = std::net::Ipv4Addr::new(octets[0], octets[1], octets[2], 2);
    format!("{candidate}/{prefix}")
}

#[cfg(target_os = "linux")]
fn spawn_tun_helper(binary: &PathBuf, config_path: &PathBuf) -> Result<Child> {
    Command::new(binary)
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| ClientError::Io(std::io::Error::new(error.kind(), error.to_string())))
}

#[cfg(target_os = "linux")]
fn wait_for_helper_start(
    child: &mut Child,
    timeout: Duration,
) -> Result<Option<std::process::ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {
                if Instant::now() >= deadline {
                    return Ok(None);
                }
            }
            Err(error) => {
                return Err(ClientError::Io(std::io::Error::new(
                    error.kind(),
                    error.to_string(),
                )));
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "linux")]
fn terminate_helper_linux() {
    let Ok(mut guard) = tun_state().lock() else {
        return;
    };
    let Some(child) = guard.helper_child.as_mut() else {
        return;
    };
    let _ = child.stdin.take();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            Err(_) => break,
        }
    }
    if let Some(path) = guard.helper_config_path.take() {
        let _ = fs::remove_file(path);
    }
    guard.helper_child = None;
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
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: impl Into<String>) -> Self {
            let previous = env::var(key).ok();
            env::set_var(key, value.into());
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                env::set_var(self.key, previous);
            } else {
                env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn disable_returns_current_status() {
        assert_eq!(disable(), current_status());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_cap_net_admin_from_proc_status() {
        let status = "Name:\twrongcl\nCapEff:\t0000000000001000\n";
        assert_eq!(parse_cap_net_admin_from_proc_status(status), Some(true));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_missing_cap_net_admin_from_proc_status() {
        let status = "Name:\twrongcl\nCapEff:\t0000000000000000\n";
        assert_eq!(parse_cap_net_admin_from_proc_status(status), Some(false));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn enable_and_disable_use_ip_tuntap_commands() {
        let _guard = env_lock().lock().unwrap();
        clear_current_interface_name();

        let temp = std::env::temp_dir().join(format!(
            "wrongcl-tun-test-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        fs::create_dir_all(&temp).unwrap();
        let state_path = temp.join("iface-state");
        let log_path = temp.join("ip.log");
        let ip_path = temp.join("ip");
        let helper_log_path = temp.join("helper.log");
        let helper_path = temp.join("helper");
        let tun_path = temp.join("tun");
        fs::write(&tun_path, b"").unwrap();

        let mut script = fs::File::create(&ip_path).unwrap();
        writeln!(
            script,
            "#!/usr/bin/env bash\nset -euo pipefail\nLOG=\"{}\"\nSTATE=\"{}\"\nprintf '%s\\n' \"$*\" >> \"$LOG\"\nif [[ \"$1\" == \"link\" && \"$2\" == \"show\" ]]; then\n  [[ -f \"$STATE\" ]]\n  exit $?\nfi\nif [[ \"$1\" == \"tuntap\" && \"$2\" == \"add\" ]]; then\n  touch \"$STATE\"\n  exit 0\nfi\nif [[ \"$1\" == \"addr\" && \"$2\" == \"replace\" ]]; then\n  exit 0\nfi\nif [[ \"$1\" == \"link\" && \"$2\" == \"set\" ]]; then\n  exit 0\nfi\nif [[ \"$1\" == \"link\" && \"$2\" == \"delete\" ]]; then\n  rm -f \"$STATE\"\n  exit 0\nfi\nexit 1\n",
            log_path.display(),
            state_path.display(),
        )
        .unwrap();
        drop(script);
        let mut helper = fs::File::create(&helper_path).unwrap();
        writeln!(
            helper,
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf '%s\\n' \"$*\" > \"{}\"\ncat >/dev/null\n",
            helper_log_path.display(),
        )
        .unwrap();
        drop(helper);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&ip_path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&ip_path, permissions).unwrap();
            let mut helper_permissions = fs::metadata(&helper_path).unwrap().permissions();
            helper_permissions.set_mode(0o755);
            fs::set_permissions(&helper_path, helper_permissions).unwrap();
        }

        let _ip_guard = EnvVarGuard::set(ENV_IP_BIN, ip_path.display().to_string());
        let _tun_guard = EnvVarGuard::set(ENV_TUN_DEVICE, tun_path.display().to_string());
        let _cap_guard = EnvVarGuard::set(ENV_FORCE_CAP, "1");
        let _helper_guard = EnvVarGuard::set(ENV_TUN_HELPER_BIN, helper_path.display().to_string());

        let enabled = enable("{}").unwrap();
        assert!(enabled.enabled);
        assert!(enabled.supported);
        let log = fs::read_to_string(&log_path).unwrap();
        assert!(log.contains("tuntap add mode tun name"));
        assert!(log.contains("addr replace 198.18.0.1/15 dev"));
        assert!(fs::read_to_string(&helper_log_path)
            .unwrap()
            .contains("--config"));

        let disabled = disable();
        assert!(!disabled.enabled);
        assert!(fs::read_to_string(&log_path)
            .unwrap()
            .contains("link delete dev"));
    }
}
