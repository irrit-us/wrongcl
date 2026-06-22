use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnState {
    Handshake,
    Active,
    Closing,
    Failed,
}

pub struct ConnInfo {
    pub id: u64,
    pub started_at_unix_ms: u64,
    pub peer_addr: SocketAddr,
    target: Mutex<Option<String>>,
    source_pid: Mutex<Option<u32>>,
    source_app: Mutex<Option<String>>,
    url: Mutex<Option<String>>,
    pub bytes_up: AtomicU64,
    pub bytes_down: AtomicU64,
    state: Mutex<ConnState>,
    close_handle: Mutex<Option<TcpStream>>,
}

impl ConnInfo {
    pub fn target(&self) -> Option<String> {
        self.target.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_target(&self, target: impl Into<String>) {
        if let Ok(mut g) = self.target.lock() {
            *g = Some(target.into());
        }
    }

    pub fn source_pid(&self) -> Option<u32> {
        self.source_pid.lock().ok().and_then(|g| *g)
    }

    pub fn set_source_pid(&self, pid: u32) {
        if let Ok(mut g) = self.source_pid.lock() {
            *g = Some(pid);
        }
    }

    pub fn source_app(&self) -> Option<String> {
        self.source_app.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_source_app(&self, app: impl Into<String>) {
        if let Ok(mut g) = self.source_app.lock() {
            *g = Some(app.into());
        }
    }

    pub fn url(&self) -> Option<String> {
        self.url.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_url(&self, url: impl Into<String>) {
        if let Ok(mut g) = self.url.lock() {
            *g = Some(url.into());
        }
    }

    pub fn state(&self) -> ConnState {
        self.state
            .lock()
            .map(|g| *g)
            .unwrap_or(ConnState::Handshake)
    }

    pub fn set_state(&self, state: ConnState) {
        if let Ok(mut g) = self.state.lock() {
            *g = state;
        }
    }

    pub fn bytes_up(&self) -> u64 {
        self.bytes_up.load(Ordering::Relaxed)
    }

    pub fn bytes_down(&self) -> u64 {
        self.bytes_down.load(Ordering::Relaxed)
    }

    /// Trigger an externally-requested shutdown of the client socket.
    /// Safe to call from any thread. Idempotent.
    pub fn close(&self) {
        let stream = match self.close_handle.lock() {
            Ok(mut g) => g.take(),
            Err(p) => p.into_inner().take(),
        };
        if let Some(stream) = stream {
            let _ = stream.shutdown(Shutdown::Both);
        }
        self.set_state(ConnState::Closing);
    }

    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "started_at_unix_ms": self.started_at_unix_ms,
            "peer_addr": self.peer_addr.to_string(),
            "target": self.target(),
            "source_pid": self.source_pid(),
            "source_app": self.source_app(),
            "url": self.url(),
            "bytes_up": self.bytes_up(),
            "bytes_down": self.bytes_down(),
            "state": self.state(),
        })
    }
}

pub struct ConnRegistry {
    next_id: AtomicU64,
    live: DashMap<u64, Arc<ConnInfo>>,
    total_connections: AtomicU64,
    failed_connections: AtomicU64,
    retired_bytes_up: AtomicU64,
    retired_bytes_down: AtomicU64,
}

impl ConnRegistry {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            live: DashMap::new(),
            total_connections: AtomicU64::new(0),
            failed_connections: AtomicU64::new(0),
            retired_bytes_up: AtomicU64::new(0),
            retired_bytes_down: AtomicU64::new(0),
        }
    }

    pub fn register(
        &self,
        peer_addr: SocketAddr,
        close_handle: Option<TcpStream>,
    ) -> Arc<ConnInfo> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let info = Arc::new(ConnInfo {
            id,
            started_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            peer_addr,
            target: Mutex::new(None),
            source_pid: Mutex::new(None),
            source_app: Mutex::new(None),
            url: Mutex::new(None),
            bytes_up: AtomicU64::new(0),
            bytes_down: AtomicU64::new(0),
            state: Mutex::new(ConnState::Handshake),
            close_handle: Mutex::new(close_handle),
        });
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.live.insert(id, Arc::clone(&info));
        info
    }

    pub fn retire(&self, id: u64, failed: bool) {
        if let Some((_, info)) = self.live.remove(&id) {
            self.retired_bytes_up
                .fetch_add(info.bytes_up(), Ordering::Relaxed);
            self.retired_bytes_down
                .fetch_add(info.bytes_down(), Ordering::Relaxed);
            if failed {
                self.failed_connections.fetch_add(1, Ordering::Relaxed);
                info.set_state(ConnState::Failed);
            }
        } else if failed {
            self.failed_connections.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn get(&self, id: u64) -> Option<Arc<ConnInfo>> {
        self.live.get(&id).map(|r| Arc::clone(r.value()))
    }

    pub fn snapshot(&self) -> RegistrySnapshot {
        let mut connections: Vec<Arc<ConnInfo>> =
            self.live.iter().map(|r| Arc::clone(r.value())).collect();
        connections.sort_by_key(|c| c.id);
        let live_up: u64 = connections.iter().map(|c| c.bytes_up()).sum();
        let live_down: u64 = connections.iter().map(|c| c.bytes_down()).sum();
        RegistrySnapshot {
            active_connections: connections.len(),
            total_connections: self.total_connections.load(Ordering::Relaxed),
            failed_connections: self.failed_connections.load(Ordering::Relaxed),
            bytes_uploaded: live_up + self.retired_bytes_up.load(Ordering::Relaxed),
            bytes_downloaded: live_down + self.retired_bytes_down.load(Ordering::Relaxed),
            connections,
        }
    }

    pub fn close(&self, id: u64) -> bool {
        if let Some(info) = self.get(id) {
            info.close();
            true
        } else {
            false
        }
    }

    pub fn close_matching(&self, filter: &ConnFilter) -> usize {
        let mut closed = 0usize;
        for info in self.live.iter().map(|r| Arc::clone(r.value())) {
            if filter.matches(&info) {
                info.close();
                closed += 1;
            }
        }
        closed
    }
}

impl Default for ConnRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RegistrySnapshot {
    pub active_connections: usize,
    pub total_connections: u64,
    pub failed_connections: u64,
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
    pub connections: Vec<Arc<ConnInfo>>,
}

impl RegistrySnapshot {
    pub fn connections_json(&self) -> Value {
        Value::Array(self.connections.iter().map(|c| c.to_json()).collect())
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConnFilter {
    pub target_contains: Option<String>,
    pub source_app_contains: Option<String>,
    pub state: Option<ConnState>,
}

impl ConnFilter {
    pub fn from_json(value: &Value) -> Self {
        let mut filter = Self::default();
        if let Some(obj) = value.as_object() {
            if let Some(t) = obj.get("target_contains").and_then(|v| v.as_str()) {
                filter.target_contains = Some(t.to_string());
            }
            if let Some(t) = obj.get("source_app_contains").and_then(|v| v.as_str()) {
                filter.source_app_contains = Some(t.to_string());
            }
            if let Some(t) = obj.get("state").and_then(|v| v.as_str()) {
                filter.state = match t {
                    "handshake" => Some(ConnState::Handshake),
                    "active" => Some(ConnState::Active),
                    "closing" => Some(ConnState::Closing),
                    "failed" => Some(ConnState::Failed),
                    _ => None,
                };
            }
        }
        filter
    }

    fn matches(&self, info: &ConnInfo) -> bool {
        if let Some(needle) = &self.target_contains {
            match info.target() {
                Some(t) if t.contains(needle) => {}
                _ => return false,
            }
        }
        if let Some(needle) = &self.source_app_contains {
            match info.source_app() {
                Some(app) if app.contains(needle) => {}
                _ => return false,
            }
        }
        if let Some(want) = self.state {
            if info.state() != want {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests;
