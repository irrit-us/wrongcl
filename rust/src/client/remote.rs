use super::*;

pub(crate) fn connect_tcp(host: &str, port: u16) -> Result<TcpStream> {
    let addrs = (host, port).to_socket_addrs().map_err(|e| {
        ClientError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("resolve {host}:{port}: {e}"),
        ))
    })?;

    let mut last_error = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(e) => last_error = Some(e),
        }
    }

    Err(ClientError::Io(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no addresses resolved for {host}:{port}"),
        )
    })))
}

pub(crate) fn clear_timeouts<T: Tunnel + ?Sized>(stream: &T) -> io::Result<()> {
    stream.set_socket_timeouts(None, None)
}

pub(super) fn normalized_path(value: &str, default: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() {
        return default.to_string();
    }
    if raw.starts_with('/') {
        raw.to_string()
    } else {
        format!("/{raw}")
    }
}

pub(super) fn host_header(explicit: Option<&str>, server_host: &str, server_port: u16) -> String {
    explicit
        .map(|value| value.to_string())
        .unwrap_or_else(|| format!("{server_host}:{server_port}"))
}

pub(super) fn read_http_headers(stream: &mut dyn Read, context: &str) -> io::Result<String> {
    let mut buf = vec![0u8; 4096];
    let mut total = 0usize;
    loop {
        match stream.read(&mut buf[total..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("{context}: connection closed"),
                ));
            }
            Ok(n) => total += n,
            Err(e) => return Err(e),
        }
        if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
            return Ok(String::from_utf8_lossy(&buf[..total]).to_string());
        }
        if total == buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{context}: response headers too large"),
            ));
        }
    }
}

pub(super) fn http_upgrade_handshake(
    stream: &mut dyn Tunnel,
    path: &str,
    host: String,
) -> Result<()> {
    let req = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Upgrade: websocket\r\n\
         Connection: keep-alive, Upgrade\r\n\
         \r\n"
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;

    let response = read_http_headers(stream, "HTTPUpgrade")?;
    if !response.starts_with("HTTP/1.1 101 ") {
        return Err(ClientError::Io(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("expected HTTP 101, got: {response}"),
        )));
    }
    Ok(())
}

pub(super) fn websocket_handshake(stream: &mut dyn Tunnel, path: &str, host: String) -> Result<()> {
    let mut random_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut random_bytes);
    let key = base64::engine::general_purpose::STANDARD.encode(random_bytes);
    let req = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\n\
         Sec-WebSocket-Version: 13\r\n\
         \r\n"
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;

    let response = read_http_headers(stream, "WebSocket")?;
    if !response.starts_with("HTTP/1.1 101 ") {
        return Err(ClientError::Io(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("expected WebSocket HTTP 101, got: {response}"),
        )));
    }
    Ok(())
}

pub(super) fn remote_socks5_connect(
    stream: &mut TcpStream,
    opts: &MixedOptions,
    target: &Target,
) -> Result<()> {
    remote_socks5_negotiate(stream, opts)?;

    let mut request = vec![0x05, 0x01, 0x00];
    write_socks_address(&mut request, &target.host, target.port)?;
    stream.write_all(&request)?;

    let mut reply = [0u8; 4];
    stream.read_exact(&mut reply)?;
    if reply[0] != 0x05 {
        return Err(ClientError::Config(
            "remote SOCKS5 reply bad version".into(),
        ));
    }
    if reply[1] != 0x00 {
        return Err(ClientError::Config(format!(
            "remote SOCKS5 CONNECT failed with reply {:#04x}",
            reply[1]
        )));
    }
    let _ = read_socks_bound_address(stream, reply[3])?;
    Ok(())
}

pub(super) fn remote_socks5_udp_associate(
    stream: &mut TcpStream,
    opts: &MixedOptions,
) -> Result<Target> {
    remote_socks5_negotiate(stream, opts)?;

    let request = [0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    stream.write_all(&request)?;

    let mut reply = [0u8; 4];
    stream.read_exact(&mut reply)?;
    if reply[0] != 0x05 {
        return Err(ClientError::Config(
            "remote SOCKS5 reply bad version".into(),
        ));
    }
    if reply[1] != 0x00 {
        return Err(ClientError::Config(format!(
            "remote SOCKS5 UDP ASSOCIATE failed with reply {:#04x}",
            reply[1]
        )));
    }
    let (host, port) = read_socks_bound_address(stream, reply[3])?;
    normalize_socks_udp_relay_target(stream, host, port)
}

fn remote_socks5_negotiate(stream: &mut TcpStream, opts: &MixedOptions) -> Result<()> {
    let use_auth = opts
        .username
        .as_deref()
        .is_some_and(|value| !value.is_empty())
        || opts
            .password
            .as_deref()
            .is_some_and(|value| !value.is_empty());

    if use_auth {
        stream.write_all(&[0x05, 0x02, 0x00, 0x02])?;
    } else {
        stream.write_all(&[0x05, 0x01, 0x00])?;
    }
    let mut method = [0u8; 2];
    stream.read_exact(&mut method)?;
    if method[0] != 0x05 {
        return Err(ClientError::Config(
            "remote SOCKS5 returned bad version".into(),
        ));
    }
    match method[1] {
        0x00 => {}
        0x02 => {
            let username = opts.username.as_deref().unwrap_or("");
            let password = opts.password.as_deref().unwrap_or("");
            write_socks5_userpass(stream, username, password)?;
        }
        0xff => {
            return Err(ClientError::Config(
                "remote SOCKS5 rejected offered auth methods".into(),
            ));
        }
        other => {
            return Err(ClientError::Config(format!(
                "remote SOCKS5 selected unsupported auth method {other:#04x}"
            )));
        }
    }
    Ok(())
}

fn write_socks5_userpass(stream: &mut TcpStream, username: &str, password: &str) -> Result<()> {
    if username.len() > u8::MAX as usize || password.len() > u8::MAX as usize {
        return Err(ClientError::Config(
            "SOCKS5 username/password must be <=255 bytes".into(),
        ));
    }
    let mut request = Vec::with_capacity(3 + username.len() + password.len());
    request.push(0x01);
    request.push(username.len() as u8);
    request.extend_from_slice(username.as_bytes());
    request.push(password.len() as u8);
    request.extend_from_slice(password.as_bytes());
    stream.write_all(&request)?;

    let mut response = [0u8; 2];
    stream.read_exact(&mut response)?;
    if response != [0x01, 0x00] {
        return Err(ClientError::Config(
            "remote SOCKS5 username/password authentication failed".into(),
        ));
    }
    Ok(())
}

fn write_socks_address(buf: &mut Vec<u8>, host: &str, port: u16) -> Result<()> {
    let bracketless = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);

    if let Ok(ip) = bracketless.parse::<Ipv4Addr>() {
        buf.push(0x01);
        buf.extend_from_slice(&ip.octets());
    } else if let Ok(ip) = bracketless.parse::<Ipv6Addr>() {
        buf.push(0x04);
        buf.extend_from_slice(&ip.octets());
    } else {
        let domain = host.as_bytes();
        if domain.is_empty() || domain.len() > u8::MAX as usize {
            return Err(ClientError::Config(
                "SOCKS5 domain must be 1..255 bytes".into(),
            ));
        }
        buf.push(0x03);
        buf.push(domain.len() as u8);
        buf.extend_from_slice(domain);
    }
    buf.extend_from_slice(&port.to_be_bytes());
    Ok(())
}

fn read_socks_bound_address(stream: &mut TcpStream, atyp: u8) -> Result<(String, u16)> {
    match atyp {
        0x01 => {
            let mut buf = [0u8; 6];
            stream.read_exact(&mut buf)?;
            let host = Ipv4Addr::from([buf[0], buf[1], buf[2], buf[3]]).to_string();
            let port = u16::from_be_bytes([buf[4], buf[5]]);
            Ok((host, port))
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len)?;
            let mut buf = vec![0u8; len[0] as usize + 2];
            stream.read_exact(&mut buf)?;
            let host = String::from_utf8(buf[..len[0] as usize].to_vec()).map_err(|_| {
                ClientError::Config("remote SOCKS5 reply contained invalid domain name".into())
            })?;
            let port = u16::from_be_bytes([buf[len[0] as usize], buf[len[0] as usize + 1]]);
            Ok((host, port))
        }
        0x04 => {
            let mut buf = [0u8; 18];
            stream.read_exact(&mut buf)?;
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&buf[..16]);
            let host = Ipv6Addr::from(octets).to_string();
            let port = u16::from_be_bytes([buf[16], buf[17]]);
            Ok((host, port))
        }
        other => Err(ClientError::Config(format!(
            "remote SOCKS5 reply used unsupported address type {other:#04x}"
        ))),
    }
}

fn normalize_socks_udp_relay_target(stream: &TcpStream, host: String, port: u16) -> Result<Target> {
    let host = if host == "0.0.0.0" || host == "::" {
        stream.peer_addr()?.ip().to_string()
    } else {
        host
    };
    Target::new(host, port)
}

pub(super) fn parse_socks5_udp_packet(packet: &[u8]) -> Result<UdpPacket> {
    if packet.len() < 4 {
        return Err(ClientError::Config(
            "remote SOCKS5 UDP packet too short".into(),
        ));
    }
    if packet[0] != 0 || packet[1] != 0 {
        return Err(ClientError::Config(
            "remote SOCKS5 UDP reserved bytes must be zero".into(),
        ));
    }
    if packet[2] != 0 {
        return Err(ClientError::UnsupportedProtocol(
            "remote SOCKS5 UDP fragmentation is not supported".into(),
        ));
    }
    let (target, header_len) = parse_socks5_udp_target(&packet[3..])?;
    Ok(UdpPacket {
        target,
        payload: packet[3 + header_len..].to_vec(),
    })
}

pub(super) fn encode_socks5_udp_packet(target: &Target, payload: &[u8]) -> Result<Vec<u8>> {
    let mut out = vec![0x00, 0x00, 0x00];
    write_socks_address(&mut out, &target.host, target.port)?;
    out.extend_from_slice(payload);
    Ok(out)
}

fn parse_socks5_udp_target(data: &[u8]) -> Result<(Target, usize)> {
    let atyp = *data
        .first()
        .ok_or_else(|| ClientError::Config("missing SOCKS5 UDP address type".into()))?;
    match atyp {
        0x01 => {
            if data.len() < 1 + 4 + 2 {
                return Err(ClientError::Config("short SOCKS5 UDP IPv4 address".into()));
            }
            let host = Ipv4Addr::from([data[1], data[2], data[3], data[4]]).to_string();
            let port = u16::from_be_bytes([data[5], data[6]]);
            Ok((Target::new(host, port)?, 7))
        }
        0x03 => {
            let len = *data
                .get(1)
                .ok_or_else(|| ClientError::Config("short SOCKS5 UDP domain length".into()))?
                as usize;
            if data.len() < 2 + len + 2 {
                return Err(ClientError::Config(
                    "short SOCKS5 UDP domain address".into(),
                ));
            }
            let host = String::from_utf8(data[2..2 + len].to_vec())
                .map_err(|_| ClientError::Config("invalid SOCKS5 UDP domain name".into()))?;
            let port = u16::from_be_bytes([data[2 + len], data[3 + len]]);
            Ok((Target::new(host, port)?, 4 + len))
        }
        0x04 => {
            if data.len() < 1 + 16 + 2 {
                return Err(ClientError::Config("short SOCKS5 UDP IPv6 address".into()));
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&data[1..17]);
            let host = Ipv6Addr::from(octets).to_string();
            let port = u16::from_be_bytes([data[17], data[18]]);
            Ok((Target::new(host, port)?, 19))
        }
        other => Err(ClientError::Config(format!(
            "unsupported SOCKS5 UDP address type {other:#04x}"
        ))),
    }
}

pub(super) fn remote_http_connect(
    stream: &mut TcpStream,
    opts: &MixedOptions,
    target: &Target,
) -> Result<()> {
    let authority = http_connect_authority(&target.host, target.port);
    let mut request =
        format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\nConnection: keep-alive\r\n");

    let username = opts.username.as_deref().unwrap_or("");
    let password = opts.password.as_deref().unwrap_or("");
    if !username.is_empty() || !password.is_empty() {
        let basic =
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
        request.push_str(&format!("Proxy-Authorization: Basic {basic}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let response = read_http_headers(stream, "remote HTTP CONNECT")?;
    let mut lines = response.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| ClientError::Config("remote HTTP CONNECT missing status line".into()))?;
    if status_line.starts_with("HTTP/1.1 200 ") || status_line.starts_with("HTTP/1.0 200 ") {
        return Ok(());
    }
    Err(ClientError::Config(format!(
        "remote HTTP CONNECT failed with status: {status_line}"
    )))
}

fn http_connect_authority(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
