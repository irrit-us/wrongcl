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
            "proxies": ["vless", "trojan", "mixed", "shadowsocks"],
            "transports": ["raw", "websocket", "httpupgrade", "xhttp", "grpc"],
            "outer_security": ["none", "tls", "reality", "anytls"],
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
            "SOCKS5 proxy started",
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
        Ok(snapshot) => ok("SOCKS5 proxy stopped", snapshot),
        Err(e) => err(e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_proxy_status() -> *mut c_char {
    match global_manager().status() {
        Ok(snapshot) if snapshot.running => ok("SOCKS5 proxy is running", snapshot),
        Ok(snapshot) => ok("SOCKS5 proxy is stopped", snapshot),
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
pub extern "C" fn wrongcl_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}
