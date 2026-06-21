use super::*;

pub(super) enum LocalProxyRequest {
    Socks(SocksRequest),
    Http(HttpRequest),
}

pub(super) struct HttpRequest {
    pub(super) target: Target,
    pub(super) connect: bool,
    pub(super) initial_bytes: Vec<u8>,
}

pub(super) enum SocksRequest {
    Connect(Target),
    UdpAssociate,
}

pub(super) fn detect_local_proxy_request(client: &mut TcpStream) -> io::Result<LocalProxyRequest> {
    let mut first = [0u8; 1];
    let n = client.peek(&mut first)?;
    if n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "client closed before proxy handshake",
        ));
    }
    if first[0] == 0x05 {
        return read_socks5_request(client).map(LocalProxyRequest::Socks);
    }
    read_http_proxy_request(client).map(LocalProxyRequest::Http)
}

fn read_socks5_request(client: &mut TcpStream) -> io::Result<SocksRequest> {
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
    let port = u16::from_be_bytes(port);
    match request[1] {
        0x01 => Target::new(host, port)
            .map(SocksRequest::Connect)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string())),
        0x03 => Ok(SocksRequest::UdpAssociate),
        other => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported SOCKS5 command {other:#04x}"),
        )),
    }
}

fn read_http_proxy_request(client: &mut TcpStream) -> io::Result<HttpRequest> {
    let mut buf = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        client.read_exact(&mut byte)?;
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if buf.len() > 8 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP CONNECT request headers too large",
            ));
        }
    }

    let text = String::from_utf8(buf)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP request"))?;
    let mut lines = text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let request_target = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or("HTTP/1.1");
    if method != "CONNECT" {
        let (target, head) = rewrite_http_forward_request(method, request_target, version, lines)?;
        return Ok(HttpRequest {
            target,
            connect: false,
            initial_bytes: head,
        });
    }
    Ok(HttpRequest {
        target: parse_connect_authority(request_target)?,
        connect: true,
        initial_bytes: Vec::new(),
    })
}

fn rewrite_http_forward_request<'a>(
    method: &str,
    request_target: &str,
    version: &str,
    lines: impl Iterator<Item = &'a str>,
) -> io::Result<(Target, Vec<u8>)> {
    let mut header_lines = Vec::new();
    let mut host_header: Option<String> = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("proxy-connection:") {
            continue;
        }
        if lower.starts_with("host:") {
            host_header = Some(line[5..].trim().to_string());
        }
        header_lines.push(line.to_string());
    }

    let (target, path) =
        if request_target.starts_with("http://") || request_target.starts_with("https://") {
            let uri: http::Uri = request_target.parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid absolute-form URI")
            })?;
            let host = uri
                .host()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "URI missing host"))?;
            let port = uri.port_u16().unwrap_or(80);
            let target = Target::new(host, port)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
            let path = uri
                .path_and_query()
                .map(|value| value.as_str().to_string())
                .unwrap_or_else(|| "/".to_string());
            (target, path)
        } else if request_target.starts_with('/') {
            let host = host_header.clone().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "HTTP request missing Host header",
                )
            })?;
            (parse_host_header(&host)?, request_target.to_string())
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported HTTP proxy request-target",
            ));
        };

    let mut out = Vec::new();
    out.extend_from_slice(format!("{method} {path} {version}\r\n").as_bytes());
    let has_host_header = header_lines
        .iter()
        .any(|line| line.to_ascii_lowercase().starts_with("host:"));
    if !has_host_header {
        out.extend_from_slice(format!("Host: {}\r\n", target.host).as_bytes());
    }
    for line in header_lines {
        out.extend_from_slice(line.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    Ok((target, out))
}

fn parse_connect_authority(authority: &str) -> io::Result<Target> {
    if authority.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing CONNECT authority",
        ));
    }
    let (host, port) = if authority.starts_with('[') {
        let end = authority
            .find("]:")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid IPv6 authority"))?;
        let host = &authority[1..end];
        let port = &authority[end + 2..];
        (host, port)
    } else {
        authority.rsplit_once(':').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "CONNECT authority must include host:port",
            )
        })?
    };
    let port = port
        .parse::<u16>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid CONNECT port"))?;
    Target::new(host, port).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
}

fn parse_host_header(host: &str) -> io::Result<Target> {
    let host = host.trim();
    if host.starts_with('[') {
        let end = host.find(']').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid IPv6 Host header")
        })?;
        let addr = &host[1..end];
        let port = host[end + 1..]
            .strip_prefix(':')
            .map(|value| value.parse::<u16>())
            .transpose()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid Host header port"))?
            .unwrap_or(80);
        return Target::new(addr, port)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()));
    }
    if let Some((host_only, port)) = host.rsplit_once(':') {
        if !host_only.contains(':') {
            let port = port.parse::<u16>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Host header port")
            })?;
            return Target::new(host_only, port)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()));
        }
    }
    Target::new(host, 80).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
}

pub(super) fn write_socks5_reply(client: &mut TcpStream, reply: u8) -> io::Result<()> {
    client.write_all(&[0x05, reply, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
}

pub(super) fn write_http_connect_ok(client: &mut TcpStream) -> io::Result<()> {
    client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
}

pub(super) fn write_http_error(client: &mut TcpStream, status: &str) -> io::Result<()> {
    let response = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    client.write_all(response.as_bytes())
}

pub(super) fn write_socks5_reply_addr(
    client: &mut TcpStream,
    reply: u8,
    addr: SocketAddr,
) -> io::Result<()> {
    let mut out = vec![0x05, reply, 0x00];
    match addr.ip() {
        IpAddr::V4(ip) => {
            out.push(0x01);
            out.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            out.push(0x04);
            out.extend_from_slice(&ip.octets());
        }
    }
    out.extend_from_slice(&addr.port().to_be_bytes());
    client.write_all(&out)
}
