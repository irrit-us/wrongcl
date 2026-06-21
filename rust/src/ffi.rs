use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use serde::Serialize;
use serde_json::{json, Value};

use crate::client::WrongsvClient;
use crate::config::{ClientConfig, LocalProxyConfig, ServerConfig};
use crate::endpoint::{Endpoint, OuterSecurity, ProxyProtocol, Transport, VlessOptions};
use crate::error::{ClientError, Result};
use crate::manager::global_manager;
use crate::protocol::Target;
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
    let config = ClientConfig {
        server: ServerConfig {
            host,
            port: server_port,
            endpoint: Endpoint {
                proxy: ProxyProtocol::Vless(VlessOptions {
                    uuid,
                    flow: String::new(),
                }),
                transport: Transport::Raw,
                outer_security: OuterSecurity::None,
            },
        },
        local: LocalProxyConfig {
            host: local_host,
            port: local_port,
        },
    };
    config.validate()?;
    Ok(config)
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

#[no_mangle]
pub extern "C" fn wrongcl_native_version() -> *mut c_char {
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
}

#[no_mangle]
pub extern "C" fn wrongcl_start_proxy(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    local_host: *const c_char,
    local_port: u16,
) -> *mut c_char {
    let config = match raw_vless_config(server_host, server_port, uuid, local_host, local_port) {
        Ok(config) => config,
        Err(e) => return err(e.to_string()),
    };
    start_proxy_with_config(config)
}

#[no_mangle]
pub extern "C" fn wrongcl_start_proxy_json(config_json: *const c_char) -> *mut c_char {
    let config = match parse_json_config(config_json) {
        Ok(config) => config,
        Err(e) => return err(e.to_string()),
    };
    start_proxy_with_config(config)
}

fn start_proxy_with_config(config: ClientConfig) -> *mut c_char {
    let stack = config.server.endpoint.stack_summary();
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

#[no_mangle]
pub extern "C" fn wrongcl_stop_proxy() -> *mut c_char {
    match global_manager().stop_proxy() {
        Ok(snapshot) => ok("local proxy stopped", snapshot),
        Err(e) => err(e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_proxy_status() -> *mut c_char {
    match global_manager().status() {
        Ok(snapshot) if snapshot.running => ok("local proxy is running", snapshot),
        Ok(snapshot) => ok("local proxy is stopped", snapshot),
        Err(e) => err(e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_probe(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    target_host: *const c_char,
    target_port: u16,
    payload: *const c_char,
) -> *mut c_char {
    let config = match raw_vless_config(server_host, server_port, uuid, c"127.0.0.1".as_ptr(), 0) {
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
}

#[no_mangle]
pub extern "C" fn wrongcl_probe_json(
    config_json: *const c_char,
    target_host: *const c_char,
    target_port: u16,
    payload: *const c_char,
) -> *mut c_char {
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
    let client = match WrongsvClient::new(config.server) {
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

#[no_mangle]
pub extern "C" fn wrongcl_stack_summary_json(config_json: *const c_char) -> *mut c_char {
    let config = match parse_json_config(config_json) {
        Ok(config) => config,
        Err(e) => return err(e.to_string()),
    };
    ok(
        "stack resolved",
        json!({
            "stack": config.server.endpoint.stack_summary(),
            "proxy": config.server.endpoint.proxy.id(),
            "transport": config.server.endpoint.transport.id(),
            "outer_security": config.server.endpoint.outer_security.id(),
        }),
    )
}

#[no_mangle]
pub extern "C" fn wrongcl_validate_config_json(config_json: *const c_char) -> *mut c_char {
    let config = match parse_json_config(config_json) {
        Ok(config) => config,
        Err(e) => return err(e.to_string()),
    };
    ok(
        "client config validated",
        json!({
            "config": config,
            "stack": config.server.endpoint.stack_summary(),
            "proxy": config.server.endpoint.proxy.id(),
            "transport": config.server.endpoint.transport.id(),
            "outer_security": config.server.endpoint.outer_security.id(),
        }),
    )
}

#[no_mangle]
pub extern "C" fn wrongcl_load_config_file_json(config_path: *const c_char) -> *mut c_char {
    let path = match c_string_arg(config_path, "config path") {
        Ok(path) => path,
        Err(e) => return err(e.to_string()),
    };
    match ClientConfig::from_file(path) {
        Ok(config) => {
            let stack = config.server.endpoint.stack_summary();
            let proxy = config.server.endpoint.proxy.id();
            let transport = config.server.endpoint.transport.id();
            let outer_security = config.server.endpoint.outer_security.id();
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
}

#[no_mangle]
pub extern "C" fn wrongcl_export_config_toml_json(config_json: *const c_char) -> *mut c_char {
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
}

#[no_mangle]
pub extern "C" fn wrongcl_capabilities_json(wrongsv_config_path: *const c_char) -> *mut c_char {
    let path = match c_string_arg(wrongsv_config_path, "wrongsv config path") {
        Ok(path) => path,
        Err(e) => return err(e.to_string()),
    };
    match inspect_wrongsv_config(path) {
        Ok(report) => ok("wrongsv capabilities inspected", report),
        Err(e) => err(e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_adapt_wrongsv_config_json(
    wrongsv_config_path: *const c_char,
    server_host: *const c_char,
    listen_host: *const c_char,
    listen_port: u16,
) -> *mut c_char {
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
}

#[no_mangle]
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
mod tests;
