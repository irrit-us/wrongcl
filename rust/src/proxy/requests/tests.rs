use super::*;

#[test]
fn buffers_entries_up_to_capacity() {
    let log = RequestLog::new(3);
    for i in 0..7 {
        log.record(RequestRecord {
            conn_id: i,
            target: format!("host{i}.example:443"),
            method: "CONNECT".into(),
            url: None,
            host: Some(format!("host{i}.example:443")),
            source_pid: None,
            source_app: None,
        });
    }
    let (entries, _) = log.since(0);
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].target, "host4.example:443");
    assert_eq!(entries[2].target, "host6.example:443");
}

#[test]
fn since_returns_only_new_entries_and_advances_cursor() {
    let log = RequestLog::new(16);
    log.record(RequestRecord {
        conn_id: 1,
        target: "a:80".into(),
        method: "GET".into(),
        url: Some("http://a/x".into()),
        host: Some("a".into()),
        source_pid: None,
        source_app: None,
    });
    log.record(RequestRecord {
        conn_id: 2,
        target: "b:443".into(),
        method: "CONNECT".into(),
        url: None,
        host: Some("b:443".into()),
        source_pid: None,
        source_app: None,
    });

    let (entries, cursor) = log.since(0);
    assert_eq!(entries.len(), 2);
    assert_eq!(cursor, 2);

    log.record(RequestRecord {
        conn_id: 3,
        target: "c:80".into(),
        method: "POST".into(),
        url: Some("http://c/y".into()),
        host: Some("c".into()),
        source_pid: Some(123),
        source_app: Some("curl".into()),
    });
    let (entries, cursor) = log.since(cursor);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].method, "POST");
    assert_eq!(entries[0].source_pid, Some(123));
    assert_eq!(entries[0].source_app.as_deref(), Some("curl"));
    assert_eq!(cursor, 3);

    let (entries, cursor2) = log.since(cursor);
    assert!(entries.is_empty());
    assert_eq!(cursor2, cursor);
}

#[test]
fn json_serializes_optional_fields_as_null() {
    let entry = RequestEntry {
        seq: 1,
        ts_unix_ms: 100,
        conn_id: 5,
        target: "x.example:443".into(),
        method: "CONNECT".into(),
        url: None,
        host: Some("x.example:443".into()),
        source_pid: None,
        source_app: None,
    };
    let value = entry.to_json();
    assert_eq!(value["target"], "x.example:443");
    assert_eq!(value["method"], "CONNECT");
    assert_eq!(value["url"], serde_json::Value::Null);
    assert_eq!(value["source_pid"], serde_json::Value::Null);
}
