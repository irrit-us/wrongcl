use super::super::udp::parse_socks5_target;
use super::*;

pub(super) fn wait_for_inactive(proxy: &ProxyHandle) -> ProxySnapshot {
    for _ in 0..40 {
        let snapshot = proxy.snapshot();
        if snapshot.active_connections == 0 {
            return snapshot;
        }
        thread::sleep(Duration::from_millis(25));
    }
    proxy.snapshot()
}

pub(super) struct FakeServer {
    pub(super) port: u16,
}

pub(super) fn shadowsocks_client_config(port: u16, method: &str, password: &str) -> ClientConfig {
    ClientConfig {
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port,
            endpoint: Endpoint {
                proxy: ProxyProtocol::Shadowsocks(ShadowsocksOptions {
                    method: method.into(),
                    password: password.into(),
                }),
                transport: Transport::Raw,
                outer_security: OuterSecurity::None,
            },
        },
        local: LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 0,
        },
    }
}

pub(super) fn spawn_fake_vless_server() -> FakeServer {
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

pub(super) fn spawn_fake_vless_udp_server() -> FakeServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        ready_tx.send(()).unwrap();
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let _ = handle_fake_vless_udp(stream);
            });
        }
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    FakeServer { port }
}

pub(super) fn spawn_fake_shadowsocks_udp_server(method: String, password: String) -> FakeServer {
    use wrongsv_shadowsocks::{
        decrypt_aead_2022_udp_request, decrypt_udp_packet, encrypt_aead_2022_udp_response,
        encrypt_udp_packet, ServerConfig as SsServerConfig,
    };

    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        ready_tx.send(()).unwrap();
        let config = SsServerConfig::new(&method, password).unwrap();
        let mut buf = [0u8; 65535];
        loop {
            let Ok((n, peer)) = socket.recv_from(&mut buf) else {
                return;
            };
            let response = if config.method.is_aead_2022() {
                let request = decrypt_aead_2022_udp_request(&buf[..n], &config).unwrap();
                encrypt_aead_2022_udp_response(
                    &config,
                    [0x11; 8],
                    request.packet_id,
                    request.client_session_id,
                    &request.address,
                    request.port,
                    &request.payload,
                )
                .unwrap()
            } else {
                let plaintext = decrypt_udp_packet(&buf[..n], &config).unwrap();
                encrypt_udp_packet(&plaintext, &config).unwrap()
            };
            let _ = socket.send_to(&response, peer);
        }
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    FakeServer { port }
}

pub(super) fn spawn_fake_http_connect_backend(
    username: Option<&str>,
    password: Option<&str>,
) -> FakeServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let username = username.map(str::to_string);
    let password = password.map(str::to_string);
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        ready_tx.send(()).unwrap();
        for stream in listener.incoming().flatten() {
            let username = username.clone();
            let password = password.clone();
            thread::spawn(move || {
                let _ = handle_fake_http_connect_backend(
                    stream,
                    username.as_deref(),
                    password.as_deref(),
                );
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

fn handle_fake_vless_udp(mut stream: TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let mut fixed = [0u8; 19];
    stream.read_exact(&mut fixed)?;
    let addons_len = fixed[17] as usize;
    if addons_len > 0 {
        let mut addons = vec![0u8; addons_len];
        stream.read_exact(&mut addons)?;
    }
    assert_eq!(fixed[18], 0x02, "expected VLESS UDP command");

    let mut port = [0u8; 2];
    stream.read_exact(&mut port)?;
    let mut atyp = [0u8; 1];
    stream.read_exact(&mut atyp)?;
    read_fake_address(&mut stream, atyp[0])?;

    stream.write_all(&[0x00, 0x00])?;
    loop {
        let mut len_buf = [0u8; 2];
        if stream.read_exact(&mut len_buf).is_err() {
            return Ok(());
        }
        let len = u16::from_be_bytes(len_buf) as usize;
        let mut packet = vec![0u8; len];
        if stream.read_exact(&mut packet).is_err() {
            return Ok(());
        }
        stream.write_all(&len_buf)?;
        stream.write_all(&packet)?;
    }
}

fn read_fake_address(reader: &mut impl Read, atyp: u8) -> io::Result<()> {
    match atyp {
        0x01 => {
            let mut addr = [0u8; 4];
            reader.read_exact(&mut addr)?;
        }
        0x02 => {
            let mut len = [0u8; 1];
            reader.read_exact(&mut len)?;
            let mut domain = vec![0u8; len[0] as usize];
            reader.read_exact(&mut domain)?;
        }
        0x03 => {
            let mut addr = [0u8; 16];
            reader.read_exact(&mut addr)?;
        }
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected address type {other}"),
            ));
        }
    }
    Ok(())
}

fn handle_fake_http_connect_backend(
    mut stream: TcpStream,
    username: Option<&str>,
    password: Option<&str>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let request = read_http_headers_from_stream(&mut stream)?;
    let mut lines = request.split("\r\n");
    assert_eq!(
        lines.next().unwrap_or_default(),
        "CONNECT example.com:80 HTTP/1.1"
    );
    let auth_header = lines.find(|line| {
        line.to_ascii_lowercase()
            .starts_with("proxy-authorization:")
    });
    if username.is_some() || password.is_some() {
        let expected = base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            username.unwrap_or(""),
            password.unwrap_or("")
        ));
        let expected_line = format!("Proxy-Authorization: Basic {expected}");
        assert_eq!(auth_header, Some(expected_line.as_str()));
    } else {
        assert!(auth_header.is_none());
    }
    stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => stream.write_all(&buf[..n])?,
            Err(e) => return Err(e),
        }
    }
}

fn read_http_headers_from_stream(stream: &mut impl Read) -> io::Result<String> {
    let mut buf = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte)?;
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            return String::from_utf8(buf)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP request"));
        }
        if buf.len() > 8 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP request headers too large",
            ));
        }
    }
}

pub(super) fn run_socks_echo(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
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

pub(super) fn run_http_connect_echo(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(b"CONNECT example.com:80 HTTP/1.1\r\nHost: example.com:80\r\n\r\n")?;

    let mut response = Vec::with_capacity(128);
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte)?;
        response.push(byte[0]);
        if response.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    assert!(std::str::from_utf8(&response)
        .unwrap()
        .starts_with("HTTP/1.1 200 Connection Established"),);

    stream.write_all(b"hello")?;
    let mut echoed = [0u8; 5];
    stream.read_exact(&mut echoed)?;
    Ok(echoed.to_vec())
}

pub(super) fn run_http_get_rejected(local_addr: SocketAddr) -> io::Result<String> {
    let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(b"GET ftp://example.com/ HTTP/1.1\r\nHost: example.com\r\n\r\n")?;

    let mut response = Vec::with_capacity(128);
    let mut byte = [0u8; 1];
    loop {
        match stream.read_exact(&mut byte) {
            Ok(()) => {
                response.push(byte[0]);
                if response.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
    }
    Ok(String::from_utf8_lossy(&response).to_string())
}

pub(super) fn run_http_absolute_form(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(
        b"GET http://example.com/hello?x=1 HTTP/1.1\r\nHost: example.com\r\nProxy-Connection: keep-alive\r\n\r\n",
    )?;

    let mut response = vec![0u8; 256];
    let n = stream.read(&mut response)?;
    response.truncate(n);
    Ok(response)
}

pub(super) fn run_socks_udp_echo(local_addr: SocketAddr) -> io::Result<Vec<u8>> {
    let mut control = TcpStream::connect_timeout(&local_addr, Duration::from_secs(2))?;
    control.set_read_timeout(Some(Duration::from_secs(3)))?;
    control.write_all(&[0x05, 0x01, 0x00])?;

    let mut greeting = [0u8; 2];
    control.read_exact(&mut greeting)?;
    assert_eq!(greeting, [0x05, 0x00]);

    control.write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])?;
    let mut reply = [0u8; 10];
    control.read_exact(&mut reply)?;
    assert_eq!(reply[1], 0x00);
    let relay_addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7])),
        u16::from_be_bytes([reply[8], reply[9]]),
    );

    let udp = UdpSocket::bind("127.0.0.1:0")?;
    udp.set_read_timeout(Some(Duration::from_secs(3)))?;

    let payload = b"ping-udp";
    let mut packet = vec![0x00, 0x00, 0x00, 0x03, 11];
    packet.extend_from_slice(b"example.com");
    packet.extend_from_slice(&53u16.to_be_bytes());
    packet.extend_from_slice(payload);
    udp.send_to(&packet, relay_addr)?;

    let mut buf = [0u8; 1024];
    let (n, _) = udp.recv_from(&mut buf)?;
    assert_eq!(&buf[..3], &[0x00, 0x00, 0x00]);
    let (_, header_len) = parse_socks5_target(&buf[3..])?;
    Ok(buf[3 + header_len..n].to_vec())
}
