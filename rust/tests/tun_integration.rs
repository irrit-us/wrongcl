#![cfg(target_os = "linux")]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use wrongcl_native::config::ClientConfig;
use wrongcl_native::proxy::ProxyHandle;
use wrongcl_native::tun;

const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";
static ECHO_COUNT: AtomicUsize = AtomicUsize::new(0);

fn ensure_loopback_up() {
    let status = Command::new("ip")
        .args(["link", "set", "lo", "up"])
        .status()
        .unwrap();
    assert!(status.success(), "failed to bring loopback up");
}

fn spawn_fake_vless_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let _ = handle_fake_vless(stream);
            });
        }
    });
    port
}

fn handle_fake_vless(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let mut fixed = [0u8; 19];
    stream.read_exact(&mut fixed)?;
    let addons_len = fixed[17] as usize;
    if addons_len > 0 {
        let mut addons = vec![0u8; addons_len];
        stream.read_exact(&mut addons)?;
    }
    let mut port = [0u8; 2];
    stream.read_exact(&mut port)?;
    let mut atyp = [0u8; 1];
    stream.read_exact(&mut atyp)?;
    match atyp[0] {
        0x01 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr)?;
        }
        0x02 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len)?;
            let mut domain = vec![0u8; len[0] as usize];
            stream.read_exact(&mut domain)?;
        }
        0x03 => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr)?;
        }
        _ => {}
    }
    stream.write_all(&[0x00, 0x00])?;
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                ECHO_COUNT.fetch_add(1, Ordering::Relaxed);
                stream.write_all(&buf[..n])?
            }
            Err(e) => return Err(e),
        }
    }
}

/// Run manually or through `scripts/verify-local.sh linux`.
#[test]
#[ignore = "requires unshare -Urn or CAP_NET_ADMIN"]
fn tun_runtime_prepares_interface_and_route() {
    ensure_loopback_up();
    let server_port = spawn_fake_vless_server();
    let mut proxy = ProxyHandle::start(
        ClientConfig::raw_vless("127.0.0.1", server_port, TEST_UUID, "127.0.0.1", 0).unwrap(),
    )
    .unwrap();
    let snapshot = proxy.snapshot();
    let local_addr = SocketAddr::new(snapshot.local_host.parse().unwrap(), snapshot.local_port);

    let status = tun::enable(&format!(
        r#"{{
  "interface_name": "wctun0",
  "address_cidr": "198.18.0.1/15",
  "routes": ["198.18.0.0/15"],
  "proxy_host": "127.0.0.1",
  "proxy_port": {}
}}"#,
        local_addr.port()
    ))
    .unwrap();
    assert!(status.enabled, "TUN runtime did not report enabled");
    assert!(status.supported, "TUN runtime did not report supported");
    let current = tun::current_status();
    assert!(current.enabled);
    assert!(current.supported);

    let route_output = Command::new("ip")
        .args(["route", "show", "dev", "wctun0"])
        .output()
        .unwrap();
    assert!(route_output.status.success());
    let route_text = String::from_utf8_lossy(&route_output.stdout);
    assert!(route_text.contains("198.18.0.0/15"));

    let _ = tun::disable();
    proxy.stop().unwrap();
    assert!(!tun::current_status().enabled);
}

#[test]
#[ignore = "requires unshare -Urn or CAP_NET_ADMIN"]
fn tun_runtime_routes_targeted_tcp_via_local_proxy() {
    ensure_loopback_up();
    let server_port = spawn_fake_vless_server();
    let mut proxy = ProxyHandle::start(
        ClientConfig::raw_vless("127.0.0.1", server_port, TEST_UUID, "127.0.0.1", 0).unwrap(),
    )
    .unwrap();
    let snapshot = proxy.snapshot();
    let local_addr = SocketAddr::new(snapshot.local_host.parse().unwrap(), snapshot.local_port);

    let status = tun::enable(&format!(
        r#"{{
  "interface_name": "wctun1",
  "address_cidr": "198.18.0.1/15",
  "routes": ["198.19.0.2/32"],
  "proxy_host": "127.0.0.1",
  "proxy_port": {}
}}"#,
        local_addr.port()
    ))
    .unwrap();
    assert!(status.enabled);
    assert!(status.supported);

    let result = (|| -> std::io::Result<Vec<u8>> {
        let mut stream =
            TcpStream::connect_timeout(&"198.19.0.2:80".parse().unwrap(), Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.write_all(b"hello")?;
        let mut echoed = [0u8; 5];
        stream.read_exact(&mut echoed)?;
        Ok(echoed.to_vec())
    })();

    let _ = tun::disable();
    proxy.stop().unwrap();

    eprintln!("echo count: {}", ECHO_COUNT.load(Ordering::Relaxed));
    assert_eq!(result.unwrap(), b"hello".to_vec());
}
