use super::*;
use std::fs;

fn write_wrongsv_config(text: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "wrongcl-ffi-test-{}-{}.toml",
        std::process::id(),
        rand::random::<u64>()
    ));
    fs::write(&path, text).unwrap();
    path
}

fn take_json(ptr: *mut c_char) -> serde_json::Value {
    assert!(!ptr.is_null());
    let text = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_string();
    unsafe { wrongcl_free_string(ptr) };
    serde_json::from_str(&text).unwrap()
}

#[test]
fn ffi_can_inspect_wrongsv_config() {
    let path = write_wrongsv_config(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[websocket]
path = "/ws"
"#,
    );
    let path_c = CString::new(path.to_string_lossy().as_bytes()).unwrap();

    let value = take_json(wrongcl_capabilities_json(path_c.as_ptr()));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["active_profile"], "websocket");
    assert_eq!(value["data"]["active_support"], "supported");
}

#[test]
fn ffi_can_adapt_wrongsv_config() {
    let path = write_wrongsv_config(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[tls]
server_name = "example.com"
"#,
    );
    let path_c = CString::new(path.to_string_lossy().as_bytes()).unwrap();
    let server_c = CString::new("127.0.0.1").unwrap();
    let listen_c = CString::new("127.0.0.1").unwrap();

    let value = take_json(wrongcl_adapt_wrongsv_config_json(
        path_c.as_ptr(),
        server_c.as_ptr(),
        listen_c.as_ptr(),
        1080,
    ));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["report"]["active_profile"], "tls");
    assert_eq!(value["data"]["config"]["server"]["host"], "127.0.0.1");
    assert_eq!(value["data"]["config"]["local"]["port"], 1080);
}

#[test]
fn ffi_returns_draft_config_when_missing_client_fields() {
    let path = write_wrongsv_config(
        r#"
listen = "0.0.0.0:443"

[[users]]
id = "12345678-1234-1234-1234-123456789abc"

[reality]
short_ids = ["aaaaaaaa"]
dest = "www.microsoft.com:443"
"#,
    );
    let path_c = CString::new(path.to_string_lossy().as_bytes()).unwrap();
    let server_c = CString::new("127.0.0.1").unwrap();
    let listen_c = CString::new("127.0.0.1").unwrap();

    let value = take_json(wrongcl_adapt_wrongsv_config_json(
        path_c.as_ptr(),
        server_c.as_ptr(),
        listen_c.as_ptr(),
        1080,
    ));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["config"], serde_json::Value::Null);
    assert_eq!(value["data"]["report"]["active_profile"], "reality");
    assert_eq!(
        value["data"]["report"]["missing_fields"][0]["field"],
        "reality.public-key"
    );
    assert_eq!(value["data"]["draft_config"]["server"]["host"], "127.0.0.1");
    assert_eq!(
        value["data"]["draft_config"]["server"]["outer-security"]["type"],
        "reality"
    );
}

#[test]
fn ffi_can_load_client_config_file() {
    let path = write_wrongsv_config(
        r#"{
  "server": {
    "host": "127.0.0.1",
    "port": 443,
    "proxy": {
      "type": "vless",
      "uuid": "12345678-1234-1234-1234-123456789abc",
      "flow": ""
    },
    "transport": {
      "type": "raw"
    },
    "outer-security": {
      "type": "none"
    }
  },
  "local": {
    "host": "127.0.0.1",
    "port": 1080
  }
}"#,
    );
    let path_c = CString::new(path.to_string_lossy().as_bytes()).unwrap();

    let value = take_json(wrongcl_load_config_file_json(path_c.as_ptr()));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["config"]["server"]["host"], "127.0.0.1");
    assert_eq!(value["data"]["stack"], "VLESS → raw → TCP");
}

#[test]
fn ffi_can_validate_client_config() {
    let config = r#"{
  "server": {
    "host": "127.0.0.1",
    "port": 443,
    "proxy": {
      "type": "vless",
      "uuid": "12345678-1234-1234-1234-123456789abc",
      "flow": ""
    },
    "transport": {
      "type": "raw"
    },
    "outer-security": {
      "type": "none"
    }
  },
  "local": {
    "host": "127.0.0.1",
    "port": 1080
  }
}"#;
    let config_c = CString::new(config).unwrap();

    let value = take_json(wrongcl_validate_config_json(config_c.as_ptr()));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["stack"], "VLESS → raw → TCP");
    assert_eq!(value["data"]["proxy"], "vless");
}

#[test]
fn ffi_can_export_config_as_toml() {
    let config = r#"{
  "server": {
    "host": "127.0.0.1",
    "port": 443,
    "proxy": {
      "type": "vless",
      "uuid": "12345678-1234-1234-1234-123456789abc",
      "flow": ""
    },
    "transport": {
      "type": "raw"
    },
    "outer-security": {
      "type": "none"
    }
  },
  "local": {
    "host": "127.0.0.1",
    "port": 1080
  }
}"#;
    let config_c = CString::new(config).unwrap();
    let value = take_json(wrongcl_export_config_toml_json(config_c.as_ptr()));
    assert_eq!(value["ok"], true);
    let toml = value["data"]["toml"].as_str().unwrap();
    assert!(toml.contains("[server]"));
    assert!(toml.contains("host = \"127.0.0.1\""));
}
