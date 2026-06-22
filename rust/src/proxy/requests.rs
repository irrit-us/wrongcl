use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};

pub const REQUEST_RING_CAPACITY: usize = 500;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RequestEntry {
    pub seq: u64,
    pub ts_unix_ms: u64,
    pub conn_id: u64,
    pub target: String,
    pub method: String,
    pub url: Option<String>,
    pub host: Option<String>,
    pub source_pid: Option<u32>,
    pub source_app: Option<String>,
}

impl RequestEntry {
    pub fn to_json(&self) -> Value {
        json!({
            "seq": self.seq,
            "ts_unix_ms": self.ts_unix_ms,
            "conn_id": self.conn_id,
            "target": self.target,
            "method": self.method,
            "url": self.url,
            "host": self.host,
            "source_pid": self.source_pid,
            "source_app": self.source_app,
        })
    }
}

pub struct RequestRecord {
    pub conn_id: u64,
    pub target: String,
    pub method: String,
    pub url: Option<String>,
    pub host: Option<String>,
    pub source_pid: Option<u32>,
    pub source_app: Option<String>,
}

pub struct RequestLog {
    capacity: usize,
    buf: Mutex<VecDeque<RequestEntry>>,
    next_seq: AtomicU64,
}

impl RequestLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buf: Mutex::new(VecDeque::with_capacity(capacity)),
            next_seq: AtomicU64::new(1),
        }
    }

    pub fn record(&self, record: RequestRecord) {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let ts_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let entry = RequestEntry {
            seq,
            ts_unix_ms,
            conn_id: record.conn_id,
            target: record.target,
            method: record.method,
            url: record.url,
            host: record.host,
            source_pid: record.source_pid,
            source_app: record.source_app,
        };
        let mut buf = match self.buf.lock() {
            Ok(buf) => buf,
            Err(p) => p.into_inner(),
        };
        if buf.len() == self.capacity {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    pub fn since(&self, cursor: u64) -> (Vec<RequestEntry>, u64) {
        let buf = match self.buf.lock() {
            Ok(buf) => buf,
            Err(p) => p.into_inner(),
        };
        let mut entries = Vec::new();
        let mut max_seq = cursor;
        for entry in buf.iter() {
            if entry.seq > cursor {
                if entry.seq > max_seq {
                    max_seq = entry.seq;
                }
                entries.push(entry.clone());
            }
        }
        (entries, max_seq)
    }

    pub fn len(&self) -> usize {
        let buf = match self.buf.lock() {
            Ok(buf) => buf,
            Err(p) => p.into_inner(),
        };
        buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

static GLOBAL_REQUEST_LOG: OnceLock<Arc<RequestLog>> = OnceLock::new();

pub fn global_request_log() -> &'static Arc<RequestLog> {
    GLOBAL_REQUEST_LOG.get_or_init(|| Arc::new(RequestLog::new(REQUEST_RING_CAPACITY)))
}

#[cfg(test)]
mod tests;
