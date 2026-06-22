use super::*;
use std::net::{IpAddr, Ipv4Addr};

fn peer() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)
}

#[test]
fn register_assigns_monotonic_ids_and_counts_total() {
    let registry = ConnRegistry::new();
    let a = registry.register(peer(), None);
    let b = registry.register(peer(), None);
    assert!(b.id > a.id);
    let snap = registry.snapshot();
    assert_eq!(snap.active_connections, 2);
    assert_eq!(snap.total_connections, 2);
    assert_eq!(snap.failed_connections, 0);
}

#[test]
fn retire_removes_from_live_and_accumulates_bytes() {
    let registry = ConnRegistry::new();
    let conn = registry.register(peer(), None);
    conn.bytes_up.fetch_add(100, Ordering::Relaxed);
    conn.bytes_down.fetch_add(250, Ordering::Relaxed);
    registry.retire(conn.id, false);

    let snap = registry.snapshot();
    assert_eq!(snap.active_connections, 0);
    assert_eq!(snap.total_connections, 1);
    assert_eq!(snap.failed_connections, 0);
    assert_eq!(snap.bytes_uploaded, 100);
    assert_eq!(snap.bytes_downloaded, 250);
}

#[test]
fn retire_with_failed_increments_failed_counter() {
    let registry = ConnRegistry::new();
    let conn = registry.register(peer(), None);
    registry.retire(conn.id, true);

    let snap = registry.snapshot();
    assert_eq!(snap.failed_connections, 1);
}

#[test]
fn snapshot_includes_target_and_state() {
    let registry = ConnRegistry::new();
    let conn = registry.register(peer(), None);
    conn.set_target("example.com:443");
    conn.set_state(ConnState::Active);
    conn.bytes_up.store(7, Ordering::Relaxed);

    let snap = registry.snapshot();
    let json = snap.connections_json();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["target"], "example.com:443");
    assert_eq!(arr[0]["state"], "active");
    assert_eq!(arr[0]["bytes_up"], 7);
}

#[test]
fn close_matching_by_target_substring() {
    let registry = ConnRegistry::new();
    let a = registry.register(peer(), None);
    let b = registry.register(peer(), None);
    a.set_target("foo.example.com:443");
    b.set_target("bar.example.org:80");

    let filter = ConnFilter::from_json(&serde_json::json!({
        "target_contains": "example.com",
    }));
    let closed = registry.close_matching(&filter);
    assert_eq!(closed, 1);
    assert_eq!(a.state(), ConnState::Closing);
    assert_eq!(b.state(), ConnState::Handshake);
}

#[test]
fn close_returns_false_for_unknown_id() {
    let registry = ConnRegistry::new();
    assert!(!registry.close(999));
}
