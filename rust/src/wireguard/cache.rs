use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;

use crate::endpoint::WireGuardOptions;
use crate::error::{ClientError, Result};
use crate::wireguard_runtime::{WireGuardRuntime, WireGuardRuntimeConfig};

static RUNTIME_CACHE: OnceLock<Mutex<HashMap<String, Arc<WireGuardRuntime>>>> = OnceLock::new();

pub(super) fn acquire_runtime(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
) -> Result<Arc<WireGuardRuntime>> {
    let key = runtime_key(server_host, server_port, opts)?;
    let cache = RUNTIME_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache
        .lock()
        .map_err(|_| ClientError::Config("wireguard runtime cache lock is poisoned".into()))?;

    if let Some(runtime) = guard.get(&key) {
        return Ok(Arc::clone(runtime));
    }

    let config = WireGuardRuntimeConfig::from_options(server_host, server_port, opts)?;
    let runtime = Arc::new(WireGuardRuntime::start(config).map_err(super::map_runtime_error)?);
    guard.insert(key, Arc::clone(&runtime));
    Ok(runtime)
}

fn runtime_key(server_host: &str, server_port: u16, opts: &WireGuardOptions) -> Result<String> {
    #[derive(Serialize)]
    struct RuntimeKey<'a> {
        server_host: &'a str,
        server_port: u16,
        options: &'a WireGuardOptions,
    }

    serde_json::to_string(&RuntimeKey {
        server_host,
        server_port,
        options: opts,
    })
    .map_err(ClientError::Json)
}
