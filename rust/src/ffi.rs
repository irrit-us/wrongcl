use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::Serialize;
use serde_json::{Value, json};

use crate::client::WrongsvClient;
use crate::config::{ClientConfig, Mode};
use crate::dns::DnsSettings;
use crate::error::{ClientError, Result};
use crate::logs::{global_ring, install_global as install_log_layer};
use crate::manager::global_manager;
use crate::protocol::Target;
use crate::proxy::{ConnFilter, global_request_log};
use crate::router::Script;
use crate::tun;
use crate::{adapt_wrongsv_config, inspect_wrongsv_config};

fn c_string_arg(ptr: *const c_char, name: &str) -> Result<String> {
    if ptr.is_null() {
        return Err(ClientError::Config(format!("{name} is required")));
    }
    let value = unsafe { CStr::from_ptr(ptr) };
    Ok(value.to_str()?.to_string())
}

fn raw_vless_config(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    local_host: *const c_char,
    local_port: u16,
) -> Result<ClientConfig> {
    let host = c_string_arg(server_host, "server host")?;
    let uuid = c_string_arg(uuid, "UUID")?;
    let local_host = c_string_arg(local_host, "local host")?;
    ClientConfig::raw_vless(host, server_port, uuid, local_host, local_port)
}

fn parse_json_config(ptr: *const c_char) -> Result<ClientConfig> {
    let text = c_string_arg(ptr, "config JSON")?;
    ClientConfig::from_json(&text)
}

fn json_ptr(value: Value) -> *mut c_char {
    let text = value.to_string();
    CString::new(text)
        .unwrap_or_else(|_| {
            CString::new(r#"{"ok":false,"message":"invalid JSON response","data":{}}"#).unwrap()
        })
        .into_raw()
}

fn ok(message: &str, data: impl Serialize) -> *mut c_char {
    json_ptr(json!({
        "ok": true,
        "message": message,
        "data": data,
    }))
}

fn err(message: impl Into<String>) -> *mut c_char {
    json_ptr(json!({
        "ok": false,
        "message": message.into(),
        "data": {},
    }))
}

fn guarded<F>(f: F) -> *mut c_char
where
    F: FnOnce() -> *mut c_char,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(ptr) => ptr,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&'static str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "non-string panic".to_string()
            };
            err(format!("internal panic: {msg}"))
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_native_version() -> *mut c_char {
    guarded(|| {
        ok(
            "wrongcl native ready",
            json!({
                "version": env!("CARGO_PKG_VERSION"),
                "headless": true,
                "proxies": ["vless", "naive", "hysteria2", "tuic", "trojan", "mixed", "shadowsocks", "wireguard"],
                "transports": ["raw", "kcp", "meek", "gdocsviewer", "quic", "webtransport", "websocket", "httpupgrade", "xhttp", "grpc"],
                "outer_security": ["none", "tls", "reality", "anytls", "shadowtls"],
                "vless_flows": ["", "xtls-rprx-vision"],
            }),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_start_proxy(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    local_host: *const c_char,
    local_port: u16,
) -> *mut c_char {
    guarded(|| {
        let config = match raw_vless_config(server_host, server_port, uuid, local_host, local_port)
        {
            Ok(config) => config,
            Err(e) => return err(e.to_string()),
        };
        start_proxy_with_config(config)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_start_proxy_json(config_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let config = match parse_json_config(config_json) {
            Ok(config) => config,
            Err(e) => return err(e.to_string()),
        };
        start_proxy_with_config(config)
    })
}

fn start_proxy_with_config(config: ClientConfig) -> *mut c_char {
    let stack = match config.resolve_active_endpoint() {
        Ok(ep) => ep.server.endpoint.stack_summary(),
        Err(e) => return err(e.to_string()),
    };
    match global_manager().start_proxy(config) {
        Ok(snapshot) => ok(
            "local proxy started",
            json!({
                "stack": stack,
                "proxy": snapshot,
            }),
        ),
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_stop_proxy() -> *mut c_char {
    guarded(|| match global_manager().stop_proxy() {
        Ok(snapshot) => ok("local proxy stopped", snapshot),
        Err(e) => err(e.to_string()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_proxy_status() -> *mut c_char {
    guarded(|| match global_manager().status() {
        Ok(snapshot) if snapshot.running => ok("local proxy is running", snapshot),
        Ok(snapshot) => ok("local proxy is stopped", snapshot),
        Err(e) => err(e.to_string()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_probe(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    target_host: *const c_char,
    target_port: u16,
    payload: *const c_char,
) -> *mut c_char {
    guarded(|| {
        let config =
            match raw_vless_config(server_host, server_port, uuid, c"127.0.0.1".as_ptr(), 0) {
                Ok(config) => config,
                Err(e) => return err(e.to_string()),
            };
        let target_host = match c_string_arg(target_host, "target host") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        let payload = match c_string_arg(payload, "payload") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        probe_with_config(config, target_host, target_port, payload)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_probe_json(
    config_json: *const c_char,
    target_host: *const c_char,
    target_port: u16,
    payload: *const c_char,
) -> *mut c_char {
    guarded(|| {
        let config = match parse_json_config(config_json) {
            Ok(config) => config,
            Err(e) => return err(e.to_string()),
        };
        let target_host = match c_string_arg(target_host, "target host") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        let payload = match c_string_arg(payload, "payload") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        probe_with_config(config, target_host, target_port, payload)
    })
}

fn probe_with_config(
    config: ClientConfig,
    target_host: String,
    target_port: u16,
    payload: String,
) -> *mut c_char {
    let target = match Target::new(target_host, target_port) {
        Ok(target) => target,
        Err(e) => return err(e.to_string()),
    };
    let server = match config.resolve_active_endpoint() {
        Ok(ep) => ep.server.clone(),
        Err(e) => return err(e.to_string()),
    };
    let client = match WrongsvClient::new(server) {
        Ok(client) => client,
        Err(e) => return err(e.to_string()),
    };
    let stack = client.stack_summary();
    match client.probe(&target, &payload) {
        Ok(data) => ok(
            "probe succeeded",
            json!({
                "stack": stack,
                "probe": data,
            }),
        ),
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_stack_summary_json(config_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let config = match parse_json_config(config_json) {
            Ok(config) => config,
            Err(e) => return err(e.to_string()),
        };
        let ep = match config.resolve_active_endpoint() {
            Ok(ep) => ep,
            Err(e) => return err(e.to_string()),
        };
        ok(
            "stack resolved",
            json!({
                "stack": ep.server.endpoint.stack_summary(),
                "proxy": ep.server.endpoint.proxy.id(),
                "transport": ep.server.endpoint.transport.id(),
                "outer_security": ep.server.endpoint.outer_security.id(),
            }),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_validate_config_json(config_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let config = match parse_json_config(config_json) {
            Ok(config) => config,
            Err(e) => return err(e.to_string()),
        };
        let ep = match config.resolve_active_endpoint() {
            Ok(ep) => ep,
            Err(e) => return err(e.to_string()),
        };
        let stack = ep.server.endpoint.stack_summary();
        let proxy_id = ep.server.endpoint.proxy.id();
        let transport_id = ep.server.endpoint.transport.id();
        let outer_security_id = ep.server.endpoint.outer_security.id();
        ok(
            "client config validated",
            json!({
                "config": config,
                "stack": stack,
                "proxy": proxy_id,
                "transport": transport_id,
                "outer_security": outer_security_id,
            }),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_load_config_file_json(config_path: *const c_char) -> *mut c_char {
    guarded(|| {
        let path = match c_string_arg(config_path, "config path") {
            Ok(path) => path,
            Err(e) => return err(e.to_string()),
        };
        match ClientConfig::from_file(path) {
            Ok(config) => {
                let ep = match config.resolve_active_endpoint() {
                    Ok(ep) => ep,
                    Err(e) => return err(e.to_string()),
                };
                let stack = ep.server.endpoint.stack_summary();
                let proxy = ep.server.endpoint.proxy.id();
                let transport = ep.server.endpoint.transport.id();
                let outer_security = ep.server.endpoint.outer_security.id();
                ok(
                    "client config loaded",
                    json!({
                        "config": config,
                        "stack": stack,
                        "proxy": proxy,
                        "transport": transport,
                        "outer_security": outer_security,
                    }),
                )
            }
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_export_config_toml_json(config_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let config = match parse_json_config(config_json) {
            Ok(config) => config,
            Err(e) => return err(e.to_string()),
        };
        match config.to_toml_string() {
            Ok(toml) => ok(
                "client config exported as TOML",
                json!({
                    "toml": toml,
                }),
            ),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_capabilities_json(wrongsv_config_path: *const c_char) -> *mut c_char {
    guarded(|| {
        let path = match c_string_arg(wrongsv_config_path, "wrongsv config path") {
            Ok(path) => path,
            Err(e) => return err(e.to_string()),
        };
        match inspect_wrongsv_config(path) {
            Ok(report) => ok("wrongsv capabilities inspected", report),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_adapt_wrongsv_config_json(
    wrongsv_config_path: *const c_char,
    server_host: *const c_char,
    listen_host: *const c_char,
    listen_port: u16,
) -> *mut c_char {
    guarded(|| {
        let path = match c_string_arg(wrongsv_config_path, "wrongsv config path") {
            Ok(path) => path,
            Err(e) => return err(e.to_string()),
        };
        let server_host = match c_string_arg(server_host, "server host") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        let listen_host = match c_string_arg(listen_host, "listen host") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        match adapt_wrongsv_config(path, server_host, listen_host, listen_port) {
            Ok(adapted) => ok("wrongsv config adapted", adapted),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_connections_list_json() -> *mut c_char {
    guarded(|| match global_manager().connections_snapshot() {
        Ok(Some(snap)) => ok(
            "connections snapshot",
            json!({
                "connections": snap.connections_json(),
                "active": snap.active_connections,
                "total": snap.total_connections,
                "failed": snap.failed_connections,
                "bytes_uploaded": snap.bytes_uploaded,
                "bytes_downloaded": snap.bytes_downloaded,
            }),
        ),
        Ok(None) => ok(
            "proxy is stopped",
            json!({
                "connections": [],
                "active": 0,
                "total": 0,
                "failed": 0,
                "bytes_uploaded": 0,
                "bytes_downloaded": 0,
            }),
        ),
        Err(e) => err(e.to_string()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_connection_close(id: u64) -> *mut c_char {
    guarded(|| match global_manager().close_connection(id) {
        Ok(true) => ok(
            "connection close requested",
            json!({ "id": id, "closed": true }),
        ),
        Ok(false) => ok("connection not found", json!({ "id": id, "closed": false })),
        Err(e) => err(e.to_string()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_connections_close_matching(filter_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let filter = if filter_json.is_null() {
            ConnFilter::default()
        } else {
            let text = match c_string_arg(filter_json, "filter JSON") {
                Ok(text) => text,
                Err(e) => return err(e.to_string()),
            };
            match serde_json::from_str::<Value>(&text) {
                Ok(value) => ConnFilter::from_json(&value),
                Err(e) => return err(format!("invalid filter JSON: {e}")),
            }
        };
        match global_manager().close_matching(&filter) {
            Ok(count) => ok("connections close requested", json!({ "closed": count })),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_logs_since(cursor: u64) -> *mut c_char {
    guarded(|| {
        install_log_layer();
        let ring = global_ring();
        let (entries, next_cursor) = ring.since(cursor);
        ok(
            "logs snapshot",
            json!({
                "entries": entries,
                "cursor": next_cursor,
                "capacity": ring.capacity(),
            }),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_requests_since(cursor: u64) -> *mut c_char {
    guarded(|| {
        let log = global_request_log();
        let (entries, next_cursor) = log.since(cursor);
        let entries: Vec<Value> = entries.iter().map(|e| e.to_json()).collect();
        ok(
            "requests snapshot",
            json!({
                "entries": entries,
                "cursor": next_cursor,
                "capacity": log.capacity(),
            }),
        )
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_proxy_groups_json() -> *mut c_char {
    guarded(|| match global_manager().proxy_groups_snapshot() {
        Ok(Some(snapshot)) => ok("proxy groups snapshot", snapshot),
        Ok(None) => ok(
            "proxy is stopped",
            json!({
                "endpoints": [],
                "groups": [],
                "active": null,
            }),
        ),
        Err(e) => err(e.to_string()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_proxy_group_select(
    group: *const c_char,
    member: *const c_char,
) -> *mut c_char {
    guarded(|| {
        let group = match c_string_arg(group, "group name") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        let member = match c_string_arg(member, "member name") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        match global_manager().select_group_member(&group, &member) {
            Ok(()) => ok(
                "group member selected",
                json!({ "group": group, "member": member }),
            ),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_dns_settings_json() -> *mut c_char {
    guarded(|| match global_manager().dns_settings_snapshot() {
        Ok(Some(snapshot)) => ok("dns settings snapshot", snapshot),
        Ok(None) => ok("proxy is stopped", json!(DnsSettings::default())),
        Err(e) => err(e.to_string()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_dns_settings_set_json(settings_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let text = match c_string_arg(settings_json, "dns settings JSON") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        let settings: DnsSettings = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(e) => return err(format!("invalid DNS settings JSON: {e}")),
        };
        match global_manager().set_dns_settings(settings.clone()) {
            Ok(()) => ok("DNS settings saved", json!(settings)),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_tun_status_json() -> *mut c_char {
    guarded(|| ok("TUN status snapshot", json!(tun::current_status())))
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_tun_enable_json(config_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let config_json = match c_string_arg(config_json, "tun config JSON") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        match tun::enable(&config_json) {
            Ok(status) => ok("TUN enabled", json!(status)),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_tun_disable() -> *mut c_char {
    guarded(|| ok("TUN disabled", json!(tun::disable())))
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_router_snapshot_json() -> *mut c_char {
    guarded(|| match global_manager().router_snapshot() {
        Ok(Some(snapshot)) => ok("router snapshot", snapshot),
        Ok(None) => ok(
            "proxy is stopped",
            json!({
                "modes": [],
                "scripts": [],
                "active_mode": null,
            }),
        ),
        Err(e) => err(e.to_string()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_router_set_active_mode(name: *const c_char) -> *mut c_char {
    guarded(|| {
        let name = match c_string_arg(name, "mode name") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        match global_manager().set_active_mode(&name) {
            Ok(()) => ok("active mode set", json!({ "active_mode": name })),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_router_set_script_json(script_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let text = match c_string_arg(script_json, "script JSON") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        let script: Script = match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => return err(format!("invalid script JSON: {e}")),
        };
        let name = script.name.clone();
        match global_manager().upsert_script(script) {
            Ok(()) => ok("script saved", json!({ "name": name })),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_router_remove_script(name: *const c_char) -> *mut c_char {
    guarded(|| {
        let name = match c_string_arg(name, "script name") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        match global_manager().remove_script(&name) {
            Ok(()) => ok("script removed", json!({ "name": name })),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_router_upsert_user_mode_json(mode_json: *const c_char) -> *mut c_char {
    guarded(|| {
        let text = match c_string_arg(mode_json, "mode JSON") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        let mode: Mode = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => return err(format!("invalid mode JSON: {e}")),
        };
        let name = mode.name.clone();
        match global_manager().upsert_user_mode(mode) {
            Ok(()) => ok("mode saved", json!({ "name": name })),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_router_remove_user_mode(name: *const c_char) -> *mut c_char {
    guarded(|| {
        let name = match c_string_arg(name, "mode name") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        };
        match global_manager().remove_user_mode(&name) {
            Ok(()) => ok("mode removed", json!({ "name": name })),
            Err(e) => err(e.to_string()),
        }
    })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `ptr` must be a pointer previously returned by one of this library's
/// exported string-producing functions, and it must not be freed more than
/// once.
pub unsafe extern "C" fn wrongcl_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let _ = unsafe { CString::from_raw(ptr) };
}

#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn wrongcl_debug_trigger_panic() -> *mut c_char {
    guarded(|| panic!("debug-triggered panic"))
}

#[cfg(test)]
mod tests;
