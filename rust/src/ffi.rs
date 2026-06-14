use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use serde::Serialize;
use serde_json::{json, Value};

use crate::client::RawVlessTcpClient;
use crate::config::{ClientConfig, LocalProxyConfig, Protocol, ServerConfig};
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

fn ffi_config(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    local_host: *const c_char,
    local_port: u16,
) -> Result<ClientConfig> {
    ClientConfig::new(
        c_string_arg(server_host, "server host")?,
        server_port,
        c_string_arg(uuid, "UUID")?,
        c_string_arg(local_host, "local host")?,
        local_port,
    )
}

fn ffi_config_ex(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    local_host: *const c_char,
    local_port: u16,
    protocol: *const c_char,
    path: *const c_char,
    host_header: *const c_char,
) -> Result<ClientConfig> {
    let protocol = parse_protocol(&c_string_arg(protocol, "protocol")?)?;
    let path = optional_string_arg(path, "path")?;
    let host_header = optional_string_arg(host_header, "host header")?;
    let config = ClientConfig {
        server: ServerConfig {
            host: c_string_arg(server_host, "server host")?,
            port: server_port,
            uuid: c_string_arg(uuid, "UUID")?,
            protocol,
            path,
            host_header,
            flow: String::new(),
        },
        local: LocalProxyConfig {
            host: c_string_arg(local_host, "local host")?,
            port: local_port,
        },
    };
    config.validate()?;
    Ok(config)
}

fn parse_protocol(value: &str) -> Result<Protocol> {
    match value {
        "raw-vless-tcp" => Ok(Protocol::RawVlessTcp),
        "vless-websocket" => Ok(Protocol::VlessWebsocket),
        "vless-httpupgrade" => Ok(Protocol::VlessHttpupgrade),
        other => Err(ClientError::UnsupportedProtocol(other.into())),
    }
}

fn optional_string_arg(ptr: *const c_char, name: &str) -> Result<Option<String>> {
    let value = c_string_arg(ptr, name)?;
    let trimmed = value.trim();
    Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
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
            "protocols": ["raw-vless-tcp", "vless-websocket", "vless-httpupgrade"],
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
    let config = match ffi_config(server_host, server_port, uuid, local_host, local_port) {
        Ok(config) => config,
        Err(e) => return err(e.to_string()),
    };

    match global_manager().start_proxy(config) {
        Ok(snapshot) => ok("SOCKS5 proxy started", snapshot),
        Err(e) => err(e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_start_proxy_ex(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    local_host: *const c_char,
    local_port: u16,
    protocol: *const c_char,
    path: *const c_char,
    host_header: *const c_char,
) -> *mut c_char {
    let config = match ffi_config_ex(
        server_host,
        server_port,
        uuid,
        local_host,
        local_port,
        protocol,
        path,
        host_header,
    ) {
        Ok(config) => config,
        Err(e) => return err(e.to_string()),
    };

    match global_manager().start_proxy(config) {
        Ok(snapshot) => ok("SOCKS5 proxy started", snapshot),
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
    let config = match ClientConfig::new(
        match c_string_arg(server_host, "server host") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        },
        server_port,
        match c_string_arg(uuid, "UUID") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        },
        "127.0.0.1",
        0,
    ) {
        Ok(config) => config,
        Err(e) => return err(e.to_string()),
    };
    let target = match Target::new(
        match c_string_arg(target_host, "target host") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        },
        target_port,
    ) {
        Ok(target) => target,
        Err(e) => return err(e.to_string()),
    };
    let payload = match c_string_arg(payload, "payload") {
        Ok(value) => value,
        Err(e) => return err(e.to_string()),
    };

    let client = match RawVlessTcpClient::new(config.server) {
        Ok(client) => client,
        Err(e) => return err(e.to_string()),
    };
    match client.probe(&target, &payload) {
        Ok(data) => ok("probe succeeded", data),
        Err(e) => err(e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_probe_ex(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    protocol: *const c_char,
    path: *const c_char,
    host_header: *const c_char,
    target_host: *const c_char,
    target_port: u16,
    payload: *const c_char,
) -> *mut c_char {
    let config = match ffi_config_ex(
        server_host,
        server_port,
        uuid,
        c"127.0.0.1".as_ptr(),
        0,
        protocol,
        path,
        host_header,
    ) {
        Ok(config) => config,
        Err(e) => return err(e.to_string()),
    };
    let target = match Target::new(
        match c_string_arg(target_host, "target host") {
            Ok(value) => value,
            Err(e) => return err(e.to_string()),
        },
        target_port,
    ) {
        Ok(target) => target,
        Err(e) => return err(e.to_string()),
    };
    let payload = match c_string_arg(payload, "payload") {
        Ok(value) => value,
        Err(e) => return err(e.to_string()),
    };

    let client = match RawVlessTcpClient::new(config.server) {
        Ok(client) => client,
        Err(e) => return err(e.to_string()),
    };
    match client.probe(&target, &payload) {
        Ok(data) => ok("probe succeeded", data),
        Err(e) => err(e.to_string()),
    }
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
