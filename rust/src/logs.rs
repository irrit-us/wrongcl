use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Map, Value};
use tracing::Event;
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::Registry;

pub const RING_CAPACITY: usize = 2000;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LogEntry {
    pub seq: u64,
    pub ts_unix_ms: u64,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Value,
}

pub struct LogRing {
    capacity: usize,
    buf: Mutex<VecDeque<LogEntry>>,
    next_seq: AtomicU64,
}

impl LogRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buf: Mutex::new(VecDeque::with_capacity(capacity)),
            next_seq: AtomicU64::new(1),
        }
    }

    pub fn push(&self, level: &str, target: &str, message: String, fields: Value) {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let ts_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let entry = LogEntry {
            seq,
            ts_unix_ms,
            level: level.to_string(),
            target: target.to_string(),
            message,
            fields,
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

    pub fn since(&self, cursor: u64) -> (Vec<LogEntry>, u64) {
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

struct LogLayer {
    ring: Arc<LogRing>,
}

struct FieldVisitor {
    message: Option<String>,
    fields: Map<String, Value>,
}

impl FieldVisitor {
    fn new() -> Self {
        Self {
            message: None,
            fields: Map::new(),
        }
    }
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields
                .insert(field.name().to_string(), Value::String(value.to_string()));
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), Value::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), Value::from(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let formatted = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(formatted);
        } else {
            self.fields
                .insert(field.name().to_string(), Value::String(formatted));
        }
    }
}

impl<S> Layer<S> for LogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);
        let message = visitor.message.unwrap_or_default();
        self.ring.push(
            metadata.level().as_str(),
            metadata.target(),
            message,
            Value::Object(visitor.fields),
        );
    }
}

static GLOBAL_RING: OnceLock<Arc<LogRing>> = OnceLock::new();
static INSTALL: OnceLock<()> = OnceLock::new();

pub fn global_ring() -> &'static Arc<LogRing> {
    GLOBAL_RING.get_or_init(|| Arc::new(LogRing::new(RING_CAPACITY)))
}

/// Install the tracing layer that feeds [`global_ring`]. Idempotent.
pub fn install_global() {
    INSTALL.get_or_init(|| {
        let ring = Arc::clone(global_ring());
        let layer = LogLayer { ring };
        let subscriber = Registry::default().with(layer);
        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}

#[cfg(test)]
mod tests;
