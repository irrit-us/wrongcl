use super::*;

pub(super) fn relay_udp_associate(
    mut client: TcpStream,
    tunnel_client: WrongsvClient,
    metrics: &ProxyMetrics,
) -> Result<()> {
    let bind_ip = client.local_addr()?.ip();
    let udp_socket = UdpSocket::bind((bind_ip, 0))?;
    udp_socket.set_read_timeout(Some(Duration::from_millis(20)))?;
    let udp_addr = udp_socket.local_addr()?;
    request::write_socks5_reply_addr(&mut client, 0x00, udp_addr)?;
    client.set_read_timeout(Some(Duration::from_millis(20)))?;

    let mut client_peer: Option<SocketAddr> = None;
    let mut sessions: std::collections::HashMap<Target, Box<dyn UdpSession>> =
        std::collections::HashMap::new();

    loop {
        if !control_connection_alive(&client)? {
            break;
        }

        let mut did_work = false;
        let mut buf = [0u8; 65535];
        match udp_socket.recv_from(&mut buf) {
            Ok((n, peer)) => {
                client_peer = Some(peer);
                if let Ok((target, payload)) = parse_socks5_udp_datagram(&buf[..n]) {
                    if !sessions.contains_key(&target) {
                        let session = tunnel_client.connect_udp_session(&target)?;
                        sessions.insert(target.clone(), session);
                    }
                    if let Some(session) = sessions.get_mut(&target) {
                        session.send_packet(&payload)?;
                        metrics
                            .bytes_uploaded
                            .fetch_add(payload.len() as u64, Ordering::Relaxed);
                        did_work = true;
                    }
                }
            }
            Err(ref e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(ClientError::Io(e)),
        }

        if let Some(peer) = client_peer {
            for session in sessions.values_mut() {
                while let Some(packet) = session.try_recv_packet()? {
                    let payload = encode_socks5_udp_datagram(&packet.target, &packet.payload)?;
                    udp_socket.send_to(&payload, peer)?;
                    metrics
                        .bytes_downloaded
                        .fetch_add(packet.payload.len() as u64, Ordering::Relaxed);
                    did_work = true;
                }
            }
        }

        if !did_work {
            thread::sleep(Duration::from_millis(10));
        }
    }

    Ok(())
}

fn control_connection_alive(client: &TcpStream) -> io::Result<bool> {
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

fn parse_socks5_udp_datagram(packet: &[u8]) -> io::Result<(Target, Vec<u8>)> {
    if packet.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SOCKS5 UDP packet too short",
        ));
    }
    if packet[0] != 0 || packet[1] != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SOCKS5 UDP reserved bytes must be zero",
        ));
    }
    if packet[2] != 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "SOCKS5 UDP fragmentation is not supported",
        ));
    }

    let (target, header_len) = parse_socks5_target(&packet[3..])?;
    Ok((target, packet[3 + header_len..].to_vec()))
}

fn encode_socks5_udp_datagram(target: &Target, payload: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = vec![0x00, 0x00, 0x00];
    write_socks5_target(&mut out, target)?;
    out.extend_from_slice(payload);
    Ok(out)
}

pub(super) fn parse_socks5_target(data: &[u8]) -> io::Result<(Target, usize)> {
    let atyp = *data
        .first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing SOCKS5 address type"))?;
    match atyp {
        0x01 => {
            if data.len() < 1 + 4 + 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "short SOCKS5 IPv4 address",
                ));
            }
            let host = Ipv4Addr::from([data[1], data[2], data[3], data[4]]).to_string();
            let port = u16::from_be_bytes([data[5], data[6]]);
            Ok((
                Target::new(host, port)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?,
                7,
            ))
        }
        0x03 => {
            let len = *data.get(1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "short SOCKS5 domain length")
            })? as usize;
            if data.len() < 2 + len + 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "short SOCKS5 domain address",
                ));
            }
            let host = String::from_utf8(data[2..2 + len].to_vec())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid domain name"))?;
            let port = u16::from_be_bytes([data[2 + len], data[3 + len]]);
            Ok((
                Target::new(host, port)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?,
                4 + len,
            ))
        }
        0x04 => {
            if data.len() < 1 + 16 + 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "short SOCKS5 IPv6 address",
                ));
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&data[1..17]);
            let host = Ipv6Addr::from(octets).to_string();
            let port = u16::from_be_bytes([data[17], data[18]]);
            Ok((
                Target::new(host, port)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?,
                19,
            ))
        }
        other => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported SOCKS5 address type {other:#04x}"),
        )),
    }
}

fn write_socks5_target(out: &mut Vec<u8>, target: &Target) -> io::Result<()> {
    let host = target.host.trim();
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        out.push(0x01);
        out.extend_from_slice(&ip.octets());
    } else if let Ok(ip) = host.parse::<Ipv6Addr>() {
        out.push(0x04);
        out.extend_from_slice(&ip.octets());
    } else {
        if host.is_empty() || host.len() > u8::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SOCKS5 domain must be 1..255 bytes",
            ));
        }
        out.push(0x03);
        out.push(host.len() as u8);
        out.extend_from_slice(host.as_bytes());
    }
    out.extend_from_slice(&target.port.to_be_bytes());
    Ok(())
}
