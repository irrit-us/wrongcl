use std::sync::{Mutex, OnceLock};

use crate::config::ClientConfig;
use crate::error::{ClientError, Result};
use crate::proxy::{ProxyHandle, ProxySnapshot};

#[derive(Default)]
pub struct ConnectionManager {
    proxy: Mutex<Option<ProxyHandle>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_proxy(&self, config: ClientConfig) -> Result<ProxySnapshot> {
        let mut guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        if guard.is_some() {
            return Err(ClientError::Config("proxy is already running".into()));
        }

        let handle = ProxyHandle::start(config)?;
        let snapshot = handle.snapshot();
        *guard = Some(handle);
        Ok(snapshot)
    }

    pub fn stop_proxy(&self) -> Result<ProxySnapshot> {
        let mut handle = {
            let mut guard = self
                .proxy
                .lock()
                .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
            guard.take()
        }
        .ok_or_else(|| ClientError::Config("proxy is not running".into()))?;

        handle.stop()?;
        Ok(ProxySnapshot::stopped())
    }

    pub fn status(&self) -> Result<ProxySnapshot> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        Ok(guard
            .as_ref()
            .map(ProxyHandle::snapshot)
            .unwrap_or_else(ProxySnapshot::stopped))
    }
}

static GLOBAL_MANAGER: OnceLock<ConnectionManager> = OnceLock::new();

pub fn global_manager() -> &'static ConnectionManager {
    GLOBAL_MANAGER.get_or_init(ConnectionManager::new)
}
