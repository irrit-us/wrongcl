use std::sync::{Mutex, OnceLock};

use serde_json::Value;

use crate::config::{ClientConfig, Mode};
use crate::dns::DnsSettings;
use crate::error::{ClientError, Result};
use crate::proxy::{ConnFilter, ProxyHandle, ProxySnapshot, RegistrySnapshot};
use crate::router::Script;

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

    pub fn connections_snapshot(&self) -> Result<Option<RegistrySnapshot>> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        Ok(guard.as_ref().map(|h| h.registry().snapshot()))
    }

    pub fn close_connection(&self, id: u64) -> Result<bool> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        Ok(guard
            .as_ref()
            .map(|h| h.registry().close(id))
            .unwrap_or(false))
    }

    pub fn close_matching(&self, filter: &ConnFilter) -> Result<usize> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        Ok(guard
            .as_ref()
            .map(|h| h.registry().close_matching(filter))
            .unwrap_or(0))
    }

    pub fn proxy_groups_snapshot(&self) -> Result<Option<Value>> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        Ok(guard.as_ref().map(|h| h.groups_snapshot()))
    }

    pub fn select_group_member(&self, group: &str, member: &str) -> Result<()> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| ClientError::Config("proxy is not running".into()))?;
        handle.select_group_member(group, member)
    }

    pub fn dns_settings_snapshot(&self) -> Result<Option<Value>> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        Ok(guard.as_ref().map(|h| h.dns_settings_json()))
    }

    pub fn set_dns_settings(&self, settings: DnsSettings) -> Result<()> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| ClientError::Config("proxy is not running".into()))?;
        handle.set_dns_settings(settings)
    }

    pub fn router_snapshot(&self) -> Result<Option<Value>> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        Ok(guard.as_ref().map(|h| h.router_snapshot()))
    }

    pub fn set_active_mode(&self, name: &str) -> Result<()> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| ClientError::Config("proxy is not running".into()))?;
        handle.set_active_mode(name)
    }

    pub fn upsert_script(&self, script: Script) -> Result<()> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| ClientError::Config("proxy is not running".into()))?;
        handle.upsert_script(script)
    }

    pub fn remove_script(&self, name: &str) -> Result<()> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| ClientError::Config("proxy is not running".into()))?;
        handle.remove_script(name)
    }

    pub fn upsert_user_mode(&self, mode: Mode) -> Result<()> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| ClientError::Config("proxy is not running".into()))?;
        handle.upsert_user_mode(mode)
    }

    pub fn remove_user_mode(&self, name: &str) -> Result<()> {
        let guard = self
            .proxy
            .lock()
            .map_err(|_| ClientError::Config("connection manager lock is poisoned".into()))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| ClientError::Config("proxy is not running".into()))?;
        handle.remove_user_mode(name)
    }
}

static GLOBAL_MANAGER: OnceLock<ConnectionManager> = OnceLock::new();

pub fn global_manager() -> &'static ConnectionManager {
    GLOBAL_MANAGER.get_or_init(ConnectionManager::new)
}
