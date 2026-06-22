use super::*;
use std::time::Instant;

const MKCP_ORIGINAL_OVERHEAD: usize = 6;

#[allow(clippy::duplicate_mod)]
#[path = "../../kcp/mask.rs"]
mod kcp_mask;
#[allow(clippy::duplicate_mod)]
#[path = "../../../../../wrongsv/crates/server/src/handler/kcp/xray_session.rs"]
mod test_xray_session;

use kcp_mask::KcpPacketMask;
use test_xray_session::{SessionConfig as XraySessionConfig, XrayKcpSession, peek_conv};

pub(super) const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

pub(super) fn vless_server(
    host: &str,
    port: u16,
    uuid: &str,
    transport: Transport,
) -> ServerConfig {
    ServerConfig {
        host: host.into(),
        port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Vless(VlessOptions {
                uuid: uuid.into(),
                flow: String::new(),
            }),
            transport,
            outer_security: OuterSecurity::None,
        },
    }
}

pub(super) fn mixed_server(host: &str, port: u16, opts: MixedOptions) -> ServerConfig {
    ServerConfig {
        host: host.into(),
        port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Mixed(opts),
            transport: Transport::Raw,
            outer_security: OuterSecurity::None,
        },
    }
}

pub(super) fn shadowsocks_server(host: &str, port: u16, opts: ShadowsocksOptions) -> ServerConfig {
    ServerConfig {
        host: host.into(),
        port,
        endpoint: Endpoint {
            proxy: ProxyProtocol::Shadowsocks(opts),
            transport: Transport::Raw,
            outer_security: OuterSecurity::None,
        },
    }
}

pub(super) fn run_socks_echo(local_addr: SocketAddr, payload: &[u8]) -> io::Result<Vec<u8>> {
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

    stream.write_all(payload)?;
    let mut response = vec![0u8; payload.len()];
    stream.read_exact(&mut response)?;
    Ok(response)
}

pub(super) fn spawn_fake_shadowsocks_server(method: String, password: String) -> FakeServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        ready_tx.send(()).unwrap();
        for stream in listener.incoming().flatten() {
            let _ = handle_fake_shadowsocks(stream, &method, &password);
        }
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    FakeServer { port }
}

fn handle_fake_shadowsocks(stream: TcpStream, method: &str, password: &str) -> io::Result<()> {
    use wrongsv_shadowsocks::{
        ServerConfig as SsServerConfig, ShadowsocksReader, ShadowsocksWriter, parse_request_header,
    };

    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let config = SsServerConfig::new(method, password.to_string())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let read_stream = stream.try_clone()?;
    let mut reader = ShadowsocksReader::new(read_stream, &config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let first = reader
        .read_chunk()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let (_addr, _port, consumed) = parse_request_header(&first)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let initial_payload = first[consumed..].to_vec();

    let mut writer = ShadowsocksWriter::new_response(stream, &config, reader.request_salt())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    if !initial_payload.is_empty() {
        writer
            .write_chunk(&initial_payload)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    }

    loop {
        match reader.read_chunk() {
            Ok(chunk) if chunk.is_empty() => return Ok(()),
            Ok(chunk) => writer
                .write_chunk(&chunk)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?,
            Err(_) => return Ok(()),
        }
    }
}

pub(super) enum FakeCarrier {
    Raw,
    HttpUpgrade,
    WebSocket,
}

pub(super) struct FakeServer {
    pub(super) port: u16,
}

pub(super) fn spawn_fake_server(carrier: FakeCarrier) -> FakeServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        ready_tx.send(()).unwrap();
        for stream in listener.incoming().flatten() {
            let _ = handle_fake_connection(stream, &carrier);
        }
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    FakeServer { port }
}

pub(super) fn spawn_fake_kcp_server(opts: KcpOptions) -> FakeServer {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        ready_tx.send(()).unwrap();
        let _ = handle_fake_kcp(socket, opts);
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    FakeServer { port }
}

pub(super) fn spawn_fake_socks5_server(
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
            let _ = handle_fake_socks5(stream, username.as_deref(), password.as_deref());
        }
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    FakeServer { port }
}

pub(super) fn spawn_fake_http_connect_server(
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
            let _ = handle_fake_http_connect(stream, username.as_deref(), password.as_deref());
        }
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    FakeServer { port }
}

fn handle_fake_socks5(
    mut stream: TcpStream,
    username: Option<&str>,
    password: Option<&str>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let mut greeting = [0u8; 2];
    stream.read_exact(&mut greeting)?;
    let mut methods = vec![0u8; greeting[1] as usize];
    stream.read_exact(&mut methods)?;
    if username.is_some() || password.is_some() {
        assert!(methods.contains(&0x02));
        stream.write_all(&[0x05, 0x02])?;
        let mut version = [0u8; 1];
        stream.read_exact(&mut version)?;
        assert_eq!(version[0], 0x01);
        let mut ulen = [0u8; 1];
        stream.read_exact(&mut ulen)?;
        let mut uname = vec![0u8; ulen[0] as usize];
        stream.read_exact(&mut uname)?;
        let mut plen = [0u8; 1];
        stream.read_exact(&mut plen)?;
        let mut pass = vec![0u8; plen[0] as usize];
        stream.read_exact(&mut pass)?;
        assert_eq!(std::str::from_utf8(&uname).unwrap(), username.unwrap_or(""));
        assert_eq!(std::str::from_utf8(&pass).unwrap(), password.unwrap_or(""));
        stream.write_all(&[0x01, 0x00])?;
    } else {
        assert!(methods.contains(&0x00));
        stream.write_all(&[0x05, 0x00])?;
    }

    let mut request = [0u8; 4];
    stream.read_exact(&mut request)?;
    assert_eq!(request[0], 0x05);
    match request[1] {
        0x01 => {
            match request[3] {
                0x01 => {
                    let mut buf = [0u8; 6];
                    stream.read_exact(&mut buf)?;
                }
                0x03 => {
                    let mut len = [0u8; 1];
                    stream.read_exact(&mut len)?;
                    let mut buf = vec![0u8; len[0] as usize + 2];
                    stream.read_exact(&mut buf)?;
                }
                0x04 => {
                    let mut buf = [0u8; 18];
                    stream.read_exact(&mut buf)?;
                }
                other => panic!("unexpected socks atyp {other}"),
            }
            stream.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])?;

            let mut buf = [0u8; 1024];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => return Ok(()),
                    Ok(n) => stream.write_all(&buf[..n])?,
                    Err(e) => return Err(e),
                }
            }
        }
        0x03 => {
            match request[3] {
                0x01 => {
                    let mut buf = [0u8; 6];
                    stream.read_exact(&mut buf)?;
                }
                0x03 => {
                    let mut len = [0u8; 1];
                    stream.read_exact(&mut len)?;
                    let mut buf = vec![0u8; len[0] as usize + 2];
                    stream.read_exact(&mut buf)?;
                }
                0x04 => {
                    let mut buf = [0u8; 18];
                    stream.read_exact(&mut buf)?;
                }
                other => panic!("unexpected socks atyp {other}"),
            }

            let udp = UdpSocket::bind("127.0.0.1:0")?;
            let addr = udp.local_addr()?;
            let mut reply = vec![0x05, 0x00, 0x00];
            reply.push(0x01);
            reply.extend_from_slice(&match addr.ip() {
                std::net::IpAddr::V4(ip) => ip.octets().to_vec(),
                _ => panic!("expected IPv4 relay address"),
            });
            reply.extend_from_slice(&addr.port().to_be_bytes());
            stream.write_all(&reply)?;
            udp.set_read_timeout(Some(Duration::from_millis(100)))?;
            stream.set_read_timeout(Some(Duration::from_millis(100)))?;

            let mut buf = [0u8; 65535];
            loop {
                match udp.recv_from(&mut buf) {
                    Ok((n, peer)) => {
                        let packet = parse_socks5_udp_packet(&buf[..n])
                            .map_err(|e| io::Error::other(e.to_string()))?;
                        let response = encode_socks5_udp_packet(&packet.target, &packet.payload)
                            .map_err(|e| io::Error::other(e.to_string()))?;
                        udp.send_to(&response, peer)?;
                    }
                    Err(ref e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) =>
                    {
                        if !control_connection_alive_for_test(&stream)? {
                            return Ok(());
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        other => panic!("unexpected SOCKS5 command {other:#04x}"),
    }
}

fn handle_fake_http_connect(
    mut stream: TcpStream,
    username: Option<&str>,
    password: Option<&str>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let request = read_http_headers(&mut stream, "fake-http-connect")?;
    let mut lines = request.split("\r\n");
    let status = lines.next().unwrap_or_default();
    assert_eq!(status, "CONNECT example.com:80 HTTP/1.1");

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
        std::net::IpAddr::V4(Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7])),
        u16::from_be_bytes([reply[8], reply[9]]),
    );

    let udp = UdpSocket::bind("127.0.0.1:0")?;
    udp.set_read_timeout(Some(Duration::from_secs(3)))?;

    let payload = b"ping-udp";
    let packet = encode_socks5_udp_packet(&Target::new("example.com", 53).unwrap(), payload)
        .map_err(|e| io::Error::other(e.to_string()))?;
    udp.send_to(&packet, relay_addr)?;

    let mut buf = [0u8; 1024];
    let (n, _) = udp.recv_from(&mut buf)?;
    let packet = parse_socks5_udp_packet(&buf[..n]).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(packet.payload)
}

fn control_connection_alive_for_test(client: &TcpStream) -> io::Result<bool> {
    let mut byte = [0u8; 1];
    match client.peek(&mut byte) {
        Ok(0) => Ok(false),
        Ok(_) => Ok(true),
        Err(ref e)
            if matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) =>
        {
            Ok(true)
        }
        Err(e) => Err(e),
    }
}

fn handle_fake_connection(mut stream: TcpStream, carrier: &FakeCarrier) -> io::Result<()> {
    match carrier {
        FakeCarrier::Raw => handle_fake_vless(stream),
        FakeCarrier::HttpUpgrade => {
            let _ = read_http_headers(&mut stream, "fake-httpupgrade")?;
            stream.write_all(
                b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
            )?;
            handle_fake_vless(stream)
        }
        FakeCarrier::WebSocket => {
            let _ = read_http_headers(&mut stream, "fake-websocket")?;
            stream.write_all(
                b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
            )?;
            let (_opcode, header) = read_ws_frame(&mut stream)?;
            let response = fake_vless_response(&header)?;
            write_ws_frame(&mut stream, &response, OpCode::Binary, false)?;
            loop {
                let (_opcode, payload) = read_ws_frame(&mut stream)?;
                write_ws_frame(&mut stream, &payload, OpCode::Binary, false)?;
            }
        }
    }
}

fn handle_fake_kcp(socket: UdpSocket, opts: KcpOptions) -> io::Result<()> {
    socket.set_read_timeout(Some(Duration::from_millis(20)))?;
    let packet_mask = KcpPacketMask::from_seed(&opts.seed);
    let started = Instant::now();
    let mut peer = None;
    let mut session = None;
    let mut handshake_done = false;
    let mut buf = [0u8; 4096];

    loop {
        let current = started.elapsed().as_millis() as u32;
        match socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                peer.get_or_insert(src);
                if Some(src) != peer {
                    continue;
                }
                let Some(packet) = packet_mask.unwrap(&buf[..n]) else {
                    continue;
                };
                let conv = peek_conv(&packet).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "KCP packet missing conv")
                })?;
                let session_ref = session.get_or_insert_with(|| {
                    XrayKcpSession::new(XraySessionConfig {
                        conv,
                        mtu: opts.mtu as usize,
                        tti: opts.tti,
                        uplink_capacity: 20,
                        downlink_capacity: 5,
                        write_buffer_size: 2 * 1024 * 1024,
                        packet_overhead: packet_mask.overhead(),
                    })
                });
                session_ref.input(&packet, current);
                while let Some(frame) = session_ref.take_received() {
                    if !handshake_done {
                        let response = fake_vless_response(&frame)?;
                        session_ref.enqueue_application_data(&response);
                        handshake_done = true;
                    } else {
                        session_ref.enqueue_application_data(&frame);
                    }
                }
            }
            Err(ref err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(err) => return Err(err),
        }

        if let (Some(session_ref), Some(peer_addr)) = (session.as_mut(), peer) {
            for packet in session_ref.flush(current) {
                let wrapped = packet_mask.wrap(&packet)?;
                socket.send_to(&wrapped, peer_addr)?;
            }
        }
    }
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

    let mut target = [0u8; 2];
    stream.read_exact(&mut target)?;
    let mut atyp = [0u8; 1];
    stream.read_exact(&mut atyp)?;
    read_fake_address(&mut stream, atyp[0])?;

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

fn fake_vless_response(header: &[u8]) -> io::Result<Vec<u8>> {
    let mut cursor = io::Cursor::new(header);
    let mut fixed = [0u8; 19];
    cursor.read_exact(&mut fixed)?;
    let addons_len = fixed[17] as usize;
    if addons_len > 0 {
        let mut addons = vec![0u8; addons_len];
        cursor.read_exact(&mut addons)?;
    }
    let mut target = [0u8; 2];
    cursor.read_exact(&mut target)?;
    let mut atyp = [0u8; 1];
    cursor.read_exact(&mut atyp)?;
    read_fake_address(&mut cursor, atyp[0])?;
    Ok(vec![0x00, 0x00])
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
