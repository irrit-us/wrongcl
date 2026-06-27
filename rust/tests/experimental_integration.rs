use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn reserve_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    format!("127.0.0.1:{port}")
}

#[test]
fn e2e_brook_proxy_works() {
    // 1. Spawn a local TCP echo target
    let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let target_addr = target_listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((mut s, _)) = target_listener.accept() {
            let mut buf = [0u8; 100];
            if let Ok(n) = s.read(&mut buf) {
                s.write_all(&buf[..n]).unwrap();
            }
        }
    });

    // 2. Start a real wrongsv Brook server running in the background
    let listen = reserve_addr();
    let config_toml = format!(
        r#"
listen = "{listen}"

[brook]
password = "test-password"
"#
    );
    let config: wrongsv_server::Config = toml::from_str(&config_toml).unwrap();
    let server = wrongsv_server::InboundServer::new(config).unwrap();
    let _handle = server.spawn();
    thread::sleep(Duration::from_millis(50));

    // 3. Connect client stream manually to verify
    let mut stream = TcpStream::connect(&listen).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    // Verify manually
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(b"test-password");
    let pass_hash = hasher.finalize();
    stream.write_all(pass_hash.as_slice()).unwrap();
    stream.write_all(&[0x01, 0x01, 127, 0, 0, 1]).unwrap();
    stream.write_all(&target_addr.port().to_be_bytes()).unwrap();

    let mut status = [0u8; 1];
    stream.read_exact(&mut status).unwrap();
    assert_eq!(status[0], 0x00);

    let msg = b"hello client server brook E2E";
    stream.write_all(msg).unwrap();
    let mut buf = [0u8; 100];
    let n = stream.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], msg);
}
