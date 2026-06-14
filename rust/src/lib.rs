use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use uuid::Uuid;

const READ_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug)]
struct ClientConfig {
    server_host: String,
    server_port: u16,
    uuid: String,
    local_host: String,
    local_port: u16,
}

impl ClientConfig {
    fn new(
        server_host: String,
        server_port: u16,
        uuid: String,
        local_host: String,
        local_port: u16,
    ) -> Result<Self, String> {
        if server_host.trim().is_empty() {
            return Err("server host is required".into());
        }
        if server_port == 0 {
            return Err("server port must be greater than zero".into());
        }
        if local_host.trim().is_empty() {
            return Err("local listen host is required".into());
        }
        Uuid::parse_str(uuid.trim()).map_err(|e| format!("invalid UUID: {e}"))?;

        Ok(Self {
            server_host: server_host.trim().to_string(),
            server_port,
            uuid: uuid.trim().to_string(),
            local_host: local_host.trim().to_string(),
            local_port,
        })
    }
}

struct ProxyState {
    stop: Arc<AtomicBool>,
    local_addr: SocketAddr,
    join: Option<JoinHandle<()>>,
}

static PROXY: OnceLock<Mutex<Option<ProxyState>>> = OnceLock::new();

fn proxy_slot() -> &'static Mutex<Option<ProxyState>> {
    PROXY.get_or_init(|| Mutex::new(None))
}

fn start_proxy_internal(config: ClientConfig) -> Result<SocketAddr, String> {
    let mut guard = proxy_slot()
        .lock()
        .map_err(|_| "proxy state lock is poisoned".to_string())?;
    if guard.is_some() {
        return Err("proxy is already running".into());
    }

    let listener = TcpListener::bind((config.local_host.as_str(), config.local_port))
        .map_err(|e| format!("bind local SOCKS5 listener: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("set listener nonblocking: {e}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| format!("read local listener address: {e}"))?;

    let stop = Arc::new(AtomicBool::new(false));
    let accept_stop = Arc::clone(&stop);
    let accept_config = config.clone();
    let join = thread::Builder::new()
        .name("wrongcl-socks5".into())
        .spawn(move || accept_loop(listener, accept_config, accept_stop))
        .map_err(|e| format!("start proxy thread: {e}"))?;

    *guard = Some(ProxyState {
        stop,
        local_addr,
        join: Some(join),
    });

    Ok(local_addr)
}

fn stop_proxy_internal() -> Result<(), String> {
    let mut state = {
        let mut guard = proxy_slot()
            .lock()
            .map_err(|_| "proxy state lock is poisoned".to_string())?;
        guard.take()
    }
    .ok_or_else(|| "proxy is not running".to_string())?;

    state.stop.store(true, Ordering::SeqCst);
    let _ = TcpStream::connect_timeout(&state.local_addr, Duration::from_millis(250));
    if let Some(join) = state.join.take() {
        join.join()
            .map_err(|_| "proxy thread panicked while stopping".to_string())?;
    }
    Ok(())
}

fn proxy_status_internal() -> Result<Option<SocketAddr>, String> {
    let guard = proxy_slot()
        .lock()
        .map_err(|_| "proxy state lock is poisoned".to_string())?;
    Ok(guard.as_ref().map(|state| state.local_addr))
}

fn accept_loop(listener: TcpListener, config: ClientConfig, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _peer)) => {
                let connection_config = config.clone();
                let _ = thread::Builder::new()
                    .name("wrongcl-socks5-conn".into())
                    .spawn(move || {
                        let _ = handle_socks_client(stream, connection_config);
                    });
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn handle_socks_client(mut client: TcpStream, config: ClientConfig) -> io::Result<()> {
    client.set_read_timeout(Some(READ_TIMEOUT))?;
    client.set_write_timeout(Some(WRITE_TIMEOUT))?;

    let target = match read_socks5_connect(&mut client) {
        Ok(target) => target,
        Err(e) => {
            let _ = write_socks5_reply(&mut client, 0x01);
            return Err(e);
        }
    };

    match connect_vless(&config, &target.host, target.port) {
        Ok(upstream) => {
            write_socks5_reply(&mut client, 0x00)?;
            relay(client, upstream)
        }
        Err(e) => {
            let _ = write_socks5_reply(&mut client, 0x05);
            Err(e)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    host: String,
    port: u16,
}

fn read_socks5_connect(client: &mut TcpStream) -> io::Result<Target> {
    let mut greeting = [0u8; 2];
    client.read_exact(&mut greeting)?;
    if greeting[0] != 0x05 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported SOCKS version",
        ));
    }

    let method_count = greeting[1] as usize;
    let mut methods = vec![0u8; method_count];
    client.read_exact(&mut methods)?;
    if !methods.contains(&0x00) {
        client.write_all(&[0x05, 0xff])?;
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "SOCKS client did not offer no-auth method",
        ));
    }
    client.write_all(&[0x05, 0x00])?;

    let mut request = [0u8; 4];
    client.read_exact(&mut request)?;
    if request[0] != 0x05 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid SOCKS request version",
        ));
    }
    if request[1] != 0x01 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "only SOCKS5 CONNECT is supported",
        ));
    }

    let host = match request[3] {
        0x01 => {
            let mut octets = [0u8; 4];
            client.read_exact(&mut octets)?;
            Ipv4Addr::from(octets).to_string()
        }
        0x03 => {
            let mut len = [0u8; 1];
            client.read_exact(&mut len)?;
            let mut domain = vec![0u8; len[0] as usize];
            client.read_exact(&mut domain)?;
            String::from_utf8(domain)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid domain name"))?
        }
        0x04 => {
            let mut octets = [0u8; 16];
            client.read_exact(&mut octets)?;
            Ipv6Addr::from(octets).to_string()
        }
        other => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported SOCKS address type: {other}"),
            ));
        }
    };

    let mut port = [0u8; 2];
    client.read_exact(&mut port)?;
    Ok(Target {
        host,
        port: u16::from_be_bytes(port),
    })
}

fn write_socks5_reply(client: &mut TcpStream, reply: u8) -> io::Result<()> {
    client.write_all(&[0x05, reply, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
}

fn relay(mut client: TcpStream, mut upstream: TcpStream) -> io::Result<()> {
    let mut upstream_reader = upstream.try_clone()?;
    let mut client_writer = client.try_clone()?;
    let downstream = thread::spawn(move || {
        let _ = io::copy(&mut upstream_reader, &mut client_writer);
        let _ = client_writer.shutdown(Shutdown::Write);
    });

    let result = io::copy(&mut client, &mut upstream).map(|_| ());
    let _ = upstream.shutdown(Shutdown::Write);
    let _ = downstream.join();
    result
}

fn connect_vless(
    config: &ClientConfig,
    target_host: &str,
    target_port: u16,
) -> io::Result<TcpStream> {
    let mut stream = connect_tcp(&config.server_host, config.server_port)?;
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;

    let header = build_vless_header(&config.uuid, target_host, target_port)?;
    stream.write_all(&header)?;
    read_vless_response(&mut stream)?;
    Ok(stream)
}

fn connect_tcp(host: &str, port: u16) -> io::Result<TcpStream> {
    let addrs = (host, port).to_socket_addrs().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("resolve {host}:{port}: {e}"),
        )
    })?;

    let mut last_error = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(stream) => return Ok(stream),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no addresses resolved for {host}:{port}"),
        )
    }))
}

fn build_vless_header(uuid: &str, target_host: &str, target_port: u16) -> io::Result<Vec<u8>> {
    let parsed_uuid = Uuid::parse_str(uuid.trim()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid UUID '{uuid}': {e}"),
        )
    })?;

    let mut header = Vec::with_capacity(64 + target_host.len());
    header.push(0x00);
    header.extend_from_slice(parsed_uuid.as_bytes());
    header.push(0x00);
    header.push(0x01);
    write_vless_address(&mut header, target_host, target_port)?;
    Ok(header)
}

fn write_vless_address(buf: &mut Vec<u8>, host: &str, port: u16) -> io::Result<()> {
    buf.extend_from_slice(&port.to_be_bytes());

    let bracketless = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);

    if let Ok(ip) = bracketless.parse::<Ipv4Addr>() {
        buf.push(0x01);
        buf.extend_from_slice(&ip.octets());
        return Ok(());
    }

    if let Ok(ip) = bracketless.parse::<Ipv6Addr>() {
        buf.push(0x03);
        buf.extend_from_slice(&ip.octets());
        return Ok(());
    }

    let domain = host.as_bytes();
    if domain.is_empty() || domain.len() > u8::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "domain must be 1..255 bytes",
        ));
    }
    buf.push(0x02);
    buf.push(domain.len() as u8);
    buf.extend_from_slice(domain);
    Ok(())
}

fn read_vless_response(stream: &mut TcpStream) -> io::Result<()> {
    let mut response = [0u8; 2];
    stream.read_exact(&mut response)?;
    if response[0] != 0x00 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid VLESS response version: {}", response[0]),
        ));
    }
    if response[1] > 0 {
        let mut addons = vec![0u8; response[1] as usize];
        stream.read_exact(&mut addons)?;
    }
    Ok(())
}

fn probe_internal(
    config: ClientConfig,
    target_host: String,
    target_port: u16,
    payload: String,
) -> Result<Value, String> {
    if target_host.trim().is_empty() {
        return Err("probe target host is required".into());
    }
    if target_port == 0 {
        return Err("probe target port must be greater than zero".into());
    }

    let payload = if payload.is_empty() {
        format!(
            "HEAD / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            target_host
        )
    } else {
        payload
    };

    let mut tunnel = connect_vless(&config, target_host.trim(), target_port)
        .map_err(|e| format!("connect through wrongsv: {e}"))?;
    tunnel
        .write_all(payload.as_bytes())
        .map_err(|e| format!("write probe payload: {e}"))?;
    tunnel.flush().ok();

    let mut buf = [0u8; 2048];
    let n = tunnel
        .read(&mut buf)
        .map_err(|e| format!("read probe response: {e}"))?;
    if n == 0 {
        return Err("probe target closed without response".into());
    }

    let preview = String::from_utf8_lossy(&buf[..n]).to_string();
    Ok(json!({
        "bytes_read": n,
        "preview": preview,
    }))
}

fn c_string_arg(ptr: *const c_char, name: &str) -> Result<String, String> {
    if ptr.is_null() {
        return Err(format!("{name} is required"));
    }
    let value = unsafe { CStr::from_ptr(ptr) };
    value
        .to_str()
        .map(|s| s.to_string())
        .map_err(|e| format!("{name} must be valid UTF-8: {e}"))
}

fn json_ptr(value: Value) -> *mut c_char {
    let text = value.to_string();
    CString::new(text)
        .unwrap_or_else(|_| {
            CString::new(r#"{"ok":false,"message":"invalid JSON response"}"#).unwrap()
        })
        .into_raw()
}

fn ok(message: &str, data: Value) -> *mut c_char {
    json_ptr(json!({
        "ok": true,
        "message": message,
        "data": data,
    }))
}

fn err(message: impl Into<String>) -> *mut c_char {
    json_ptr(json!({
        "ok": false,
        "message": message.into(),
        "data": {},
    }))
}

fn ffi_config(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    local_host: *const c_char,
    local_port: u16,
) -> Result<ClientConfig, String> {
    ClientConfig::new(
        c_string_arg(server_host, "server host")?,
        server_port,
        c_string_arg(uuid, "UUID")?,
        c_string_arg(local_host, "local host")?,
        local_port,
    )
}

#[no_mangle]
pub extern "C" fn wrongcl_native_version() -> *mut c_char {
    ok(
        "wrongcl native ready",
        json!({ "version": env!("CARGO_PKG_VERSION") }),
    )
}

#[no_mangle]
pub extern "C" fn wrongcl_start_proxy(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    local_host: *const c_char,
    local_port: u16,
) -> *mut c_char {
    let config = match ffi_config(server_host, server_port, uuid, local_host, local_port) {
        Ok(config) => config,
        Err(e) => return err(e),
    };

    match start_proxy_internal(config) {
        Ok(addr) => ok(
            "SOCKS5 proxy started",
            json!({
                "running": true,
                "local_host": addr.ip().to_string(),
                "local_port": addr.port(),
            }),
        ),
        Err(e) => err(e),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_stop_proxy() -> *mut c_char {
    match stop_proxy_internal() {
        Ok(()) => ok("SOCKS5 proxy stopped", json!({ "running": false })),
        Err(e) => err(e),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_proxy_status() -> *mut c_char {
    match proxy_status_internal() {
        Ok(Some(addr)) => ok(
            "SOCKS5 proxy is running",
            json!({
                "running": true,
                "local_host": addr.ip().to_string(),
                "local_port": addr.port(),
            }),
        ),
        Ok(None) => ok("SOCKS5 proxy is stopped", json!({ "running": false })),
        Err(e) => err(e),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_probe(
    server_host: *const c_char,
    server_port: u16,
    uuid: *const c_char,
    target_host: *const c_char,
    target_port: u16,
    payload: *const c_char,
) -> *mut c_char {
    let config = match ClientConfig::new(
        match c_string_arg(server_host, "server host") {
            Ok(value) => value,
            Err(e) => return err(e),
        },
        server_port,
        match c_string_arg(uuid, "UUID") {
            Ok(value) => value,
            Err(e) => return err(e),
        },
        "127.0.0.1".into(),
        1080,
    ) {
        Ok(config) => config,
        Err(e) => return err(e),
    };
    let target_host = match c_string_arg(target_host, "target host") {
        Ok(value) => value,
        Err(e) => return err(e),
    };
    let payload = match c_string_arg(payload, "payload") {
        Ok(value) => value,
        Err(e) => return err(e),
    };

    match probe_internal(config, target_host, target_port, payload) {
        Ok(data) => ok("probe succeeded", data),
        Err(e) => err(e),
    }
}

#[no_mangle]
pub extern "C" fn wrongcl_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

    #[test]
    fn builds_vless_header_for_domain() {
        let header = build_vless_header(TEST_UUID, "example.com", 443).unwrap();

        assert_eq!(header[0], 0x00);
        assert_eq!(
            &header[1..17],
            Uuid::parse_str(TEST_UUID).unwrap().as_bytes()
        );
        assert_eq!(header[17], 0x00);
        assert_eq!(header[18], 0x01);
        assert_eq!(&header[19..21], &443u16.to_be_bytes());
        assert_eq!(header[21], 0x02);
        assert_eq!(header[22], "example.com".len() as u8);
        assert_eq!(&header[23..], b"example.com");
    }

    #[test]
    fn builds_vless_header_for_ipv4() {
        let header = build_vless_header(TEST_UUID, "127.0.0.1", 80).unwrap();

        assert_eq!(header[21], 0x01);
        assert_eq!(&header[22..26], &[127, 0, 0, 1]);
    }

    #[test]
    fn rejects_invalid_uuid() {
        let err = build_vless_header("not-a-uuid", "127.0.0.1", 80).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn probe_works_against_fake_vless_server() {
        let server = spawn_fake_vless_server();
        let config = ClientConfig::new(
            "127.0.0.1".into(),
            server.port,
            TEST_UUID.into(),
            "127.0.0.1".into(),
            0,
        )
        .unwrap();

        let result = probe_internal(config, "example.com".into(), 80, "ping".into()).unwrap();

        assert_eq!(result["bytes_read"], 4);
        assert_eq!(result["preview"], "ping");
    }

    #[test]
    fn socks_proxy_relays_through_fake_vless_server() {
        let server = spawn_fake_vless_server();
        let config = ClientConfig::new(
            "127.0.0.1".into(),
            server.port,
            TEST_UUID.into(),
            "127.0.0.1".into(),
            0,
        )
        .unwrap();
        let local_addr = start_proxy_internal(config).unwrap();

        let result = run_socks_echo(local_addr);
        let _ = stop_proxy_internal();

        assert_eq!(result.unwrap(), b"hello".to_vec());
    }

    struct FakeServer {
        port: u16,
    }

    fn spawn_fake_vless_server() -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            ready_tx.send(()).unwrap();
            for stream in listener.incoming().flatten() {
                thread::spawn(move || {
                    let _ = handle_fake_vless(stream);
                });
            }
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        FakeServer { port }
    }

    fn handle_fake_vless(mut stream: TcpStream) -> io::Result<()> {
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
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected address type {other}"),
                ));
            }
        }

        stream.write_all(&[0x00, 0x00])?;
        let mut buf = [0u8; 1024];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => return Ok(()),
                Ok(n) => stream.write_all(&buf[..n])?,
                Err(e) => return Err(e),
            }
        }
    }

    fn run_socks_echo(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
        let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.write_all(&[0x05, 0x01, 0x00])?;

        let mut greeting = [0u8; 2];
        stream.read_exact(&mut greeting)?;
        assert_eq!(greeting, [0x05, 0x00]);

        let host = b"example.com";
        let mut request = vec![0x05, 0x01, 0x00, 0x03, host.len() as u8];
        request.extend_from_slice(host);
        request.extend_from_slice(&80u16.to_be_bytes());
        stream.write_all(&request)?;

        let mut reply = [0u8; 10];
        stream.read_exact(&mut reply)?;
        assert_eq!(reply[1], 0x00);

        stream.write_all(b"hello")?;
        let mut response = [0u8; 5];
        stream.read_exact(&mut response)?;
        Ok(response.to_vec())
    }
}
