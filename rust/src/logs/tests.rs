use super::*;

#[test]
fn ring_buffers_entries_up_to_capacity() {
    let ring = LogRing::new(4);
    for i in 0..10 {
        ring.push("INFO", "test", format!("msg-{i}"), Value::Null);
    }
    assert_eq!(ring.len(), 4);
    let (entries, _) = ring.since(0);
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].message, "msg-6");
    assert_eq!(entries[3].message, "msg-9");
}

#[test]
fn since_returns_only_new_entries_and_advances_cursor() {
    let ring = LogRing::new(16);
    for i in 0..3 {
        ring.push("INFO", "test", format!("msg-{i}"), Value::Null);
    }
    let (entries, cursor) = ring.since(0);
    assert_eq!(entries.len(), 3);
    assert_eq!(cursor, 3);

    ring.push("WARN", "test", "msg-3".into(), Value::Null);
    let (entries, cursor) = ring.since(cursor);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message, "msg-3");
    assert_eq!(entries[0].level, "WARN");
    assert_eq!(cursor, 4);

    let (entries, cursor2) = ring.since(cursor);
    assert!(entries.is_empty());
    assert_eq!(cursor2, cursor);
}

#[test]
fn fields_are_recorded_into_json() {
    let ring = LogRing::new(8);
    let mut fields = Map::new();
    fields.insert("conn_id".into(), Value::from(42u64));
    fields.insert("target".into(), Value::String("example.com:443".into()));
    ring.push(
        "INFO",
        "wrongcl::proxy",
        "connecting".into(),
        Value::Object(fields),
    );

    let (entries, _) = ring.since(0);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].target, "wrongcl::proxy");
    assert_eq!(entries[0].fields["conn_id"], Value::from(42u64));
    assert_eq!(
        entries[0].fields["target"],
        Value::String("example.com:443".into())
    );
}
