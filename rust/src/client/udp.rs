use super::*;

pub(super) struct StreamUdpSession {
    writer: Box<dyn TunnelWriter>,
    responses: Receiver<std::result::Result<UdpPacket, ClientError>>,
}

impl StreamUdpSession {
    fn new(stream: Box<dyn Tunnel>, target: Target) -> Result<Self> {
        let (reader, writer) = stream.split_box()?;
        let (tx, rx) = mpsc::channel();
        let target_for_thread = target.clone();
        thread::spawn(move || {
            let mut reader = LengthPacketReader::new(reader);
            loop {
                match reader.read_packet() {
                    Ok(packet) => {
                        if tx
                            .send(Ok(UdpPacket {
                                target: target_for_thread.clone(),
                                payload: packet.to_vec(),
                            }))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(PacketReadError::Io(ref e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                        break;
                    }
                    Err(PacketReadError::Io(e)) => {
                        let _ = tx.send(Err(ClientError::Io(e)));
                        break;
                    }
                    Err(PacketReadError::TooLarge(len)) => {
                        let _ = tx.send(Err(ClientError::Config(format!(
                            "UDP packet too large: {len} bytes"
                        ))));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            writer,
            responses: rx,
        })
    }
}

impl Drop for StreamUdpSession {
    fn drop(&mut self) {
        let _ = self.writer.shutdown_write();
    }
}

impl UdpSession for StreamUdpSession {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        LengthPacketWriter::new(self.writer.as_mut()).write_packet(payload)?;
        self.writer.flush()?;
        Ok(())
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        match self.responses.try_recv() {
            Ok(result) => result.map(Some),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => Ok(None),
        }
    }
}

struct TrojanUdpSession {
    target: Target,
    writer: Box<dyn TunnelWriter>,
    responses: Receiver<std::result::Result<UdpPacket, ClientError>>,
}

impl TrojanUdpSession {
    fn new(stream: Box<dyn Tunnel>, target: Target) -> Result<Self> {
        let (mut reader, writer) = stream.split_box()?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = Vec::new();
            let mut tmp = [0u8; 8192];
            loop {
                match reader.read(&mut tmp) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&tmp[..n]);
                        loop {
                            match trojan::parse_udp_packet(&buf) {
                                Ok(Some((target, payload, consumed))) => {
                                    buf.drain(..consumed);
                                    if tx.send(Ok(UdpPacket { target, payload })).is_err() {
                                        return;
                                    }
                                }
                                Ok(None) => break,
                                Err(err) => {
                                    let _ = tx.send(Err(err));
                                    return;
                                }
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(e) => {
                        let _ = tx.send(Err(ClientError::Io(e)));
                        return;
                    }
                }
            }
        });
        Ok(Self {
            target,
            writer,
            responses: rx,
        })
    }
}

impl Drop for TrojanUdpSession {
    fn drop(&mut self) {
        let _ = self.writer.shutdown_write();
    }
}

impl UdpSession for TrojanUdpSession {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        let packet = trojan::encode_udp_packet(&self.target, payload)?;
        self.writer.write_all(&packet)?;
        self.writer.flush()?;
        Ok(())
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        match self.responses.try_recv() {
            Ok(result) => result.map(Some),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => Ok(None),
        }
    }
}

struct ShadowsocksUdpSession {
    target: Target,
    config: wrongsv_shadowsocks::ServerConfig,
    socket: UdpSocket,
    responses: Receiver<std::result::Result<UdpPacket, ClientError>>,
    client_session_id: [u8; 8],
    next_packet_id: u64,
}

impl ShadowsocksUdpSession {
    fn new(
        config: wrongsv_shadowsocks::ServerConfig,
        server_addr: std::net::SocketAddr,
        target: Target,
    ) -> Result<Self> {
        let bind_addr = match server_addr {
            std::net::SocketAddr::V4(_) => "0.0.0.0:0",
            std::net::SocketAddr::V6(_) => "[::]:0",
        };
        let socket = UdpSocket::bind(bind_addr)?;
        socket.connect(server_addr)?;
        let read_socket = socket.try_clone()?;
        read_socket.set_read_timeout(Some(Duration::from_millis(200)))?;
        let config_for_thread = config.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0u8; 65535];
            loop {
                match read_socket.recv(&mut buf) {
                    Ok(0) => continue,
                    Ok(n) => {
                        let packet = &buf[..n];
                        let parsed = if config_for_thread.method.is_aead_2022() {
                            wrongsv_shadowsocks::decrypt_aead_2022_udp_response(
                                packet,
                                &config_for_thread,
                            )
                            .map(|response| UdpPacket {
                                target: Target::new(response.address.to_string(), response.port.0)
                                    .expect("valid target"),
                                payload: response.payload,
                            })
                            .map_err(|e| {
                                ClientError::Config(format!("Shadowsocks UDP response: {e}"))
                            })
                        } else {
                            let plaintext =
                                wrongsv_shadowsocks::decrypt_udp_packet(packet, &config_for_thread)
                                    .map_err(|e| {
                                        ClientError::Config(format!(
                                            "Shadowsocks UDP response: {e}"
                                        ))
                                    });
                            plaintext.and_then(|plain| {
                                let (address, port, consumed) =
                                    wrongsv_shadowsocks::parse_request_header(&plain).map_err(
                                        |e| {
                                            ClientError::Config(format!(
                                                "Shadowsocks UDP header: {e}"
                                            ))
                                        },
                                    )?;
                                Ok(UdpPacket {
                                    target: Target::new(address.to_string(), port.0)?,
                                    payload: plain[consumed..].to_vec(),
                                })
                            })
                        };
                        if tx.send(parsed).is_err() {
                            break;
                        }
                    }
                    Err(ref e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(ClientError::Io(e)));
                        break;
                    }
                }
            }
        });

        let mut client_session_id = [0u8; 8];
        rand::rngs::OsRng.fill_bytes(&mut client_session_id);

        Ok(Self {
            target,
            config,
            socket,
            responses: rx,
            client_session_id,
            next_packet_id: 0,
        })
    }
}

impl UdpSession for ShadowsocksUdpSession {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        let address = Address::parse(&self.target.host);
        let port = Port(self.target.port);
        let packet = if self.config.method.is_aead_2022() {
            let packet = wrongsv_shadowsocks::encrypt_aead_2022_udp_request(
                &self.config,
                self.client_session_id,
                self.next_packet_id,
                &address,
                port,
                payload,
            )
            .map_err(|e| ClientError::Config(format!("Shadowsocks UDP request: {e}")))?;
            self.next_packet_id = self.next_packet_id.wrapping_add(1);
            packet
        } else {
            let mut plaintext = Vec::with_capacity(payload.len() + self.target.host.len() + 32);
            wrongsv_shadowsocks::write_request_header(&mut plaintext, &address, port);
            plaintext.extend_from_slice(payload);
            wrongsv_shadowsocks::encrypt_udp_packet(&plaintext, &self.config)
                .map_err(|e| ClientError::Config(format!("Shadowsocks UDP request: {e}")))?
        };
        self.socket.send(&packet)?;
        Ok(())
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        match self.responses.try_recv() {
            Ok(result) => result.map(Some),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => Ok(None),
        }
    }
}

struct RemoteSocks5UdpSession {
    target: Target,
    socket: UdpSocket,
    responses: Receiver<std::result::Result<UdpPacket, ClientError>>,
    _control: TcpStream,
}

impl RemoteSocks5UdpSession {
    fn new(control: TcpStream, relay: Target, target: Target) -> Result<Self> {
        let relay_addr = format!("{}:{}", relay.host, relay.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| ClientError::Config("failed to resolve SOCKS5 UDP relay".into()))?;
        let bind_addr = match relay_addr {
            std::net::SocketAddr::V4(_) => "0.0.0.0:0",
            std::net::SocketAddr::V6(_) => "[::]:0",
        };
        let socket = UdpSocket::bind(bind_addr)?;
        socket.connect(relay_addr)?;
        let read_socket = socket.try_clone()?;
        read_socket.set_read_timeout(Some(Duration::from_millis(200)))?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0u8; 65535];
            loop {
                match read_socket.recv(&mut buf) {
                    Ok(0) => continue,
                    Ok(n) => {
                        let parsed = super::remote::parse_socks5_udp_packet(&buf[..n]);
                        if tx.send(parsed).is_err() {
                            break;
                        }
                    }
                    Err(ref e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(ClientError::Io(e)));
                        break;
                    }
                }
            }
        });

        Ok(Self {
            target,
            socket,
            responses: rx,
            _control: control,
        })
    }
}

impl UdpSession for RemoteSocks5UdpSession {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()> {
        let packet = super::remote::encode_socks5_udp_packet(&self.target, payload)?;
        self.socket.send(&packet)?;
        Ok(())
    }

    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>> {
        match self.responses.try_recv() {
            Ok(result) => result.map(Some),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => Ok(None),
        }
    }
}

pub(super) fn open_stream_udp_session(
    stream: Box<dyn Tunnel>,
    target: Target,
) -> Result<Box<dyn UdpSession>> {
    Ok(Box::new(StreamUdpSession::new(stream, target)?))
}

pub(super) fn open_trojan_udp_session(
    stream: Box<dyn Tunnel>,
    target: Target,
) -> Result<Box<dyn UdpSession>> {
    Ok(Box::new(TrojanUdpSession::new(stream, target)?))
}

pub(super) fn open_shadowsocks_udp_session(
    config: wrongsv_shadowsocks::ServerConfig,
    server_addr: std::net::SocketAddr,
    target: Target,
) -> Result<Box<dyn UdpSession>> {
    Ok(Box::new(ShadowsocksUdpSession::new(
        config,
        server_addr,
        target,
    )?))
}

pub(super) fn open_remote_socks5_udp_session(
    control: TcpStream,
    relay: Target,
    target: Target,
) -> Result<Box<dyn UdpSession>> {
    Ok(Box::new(RemoteSocks5UdpSession::new(
        control, relay, target,
    )?))
}
