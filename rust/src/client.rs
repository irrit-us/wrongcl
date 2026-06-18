use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, Shutdown, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use base64::Engine as _;
use rand::RngCore;
use serde::Serialize;
use wrongsv_net_types::{Address, Port};
use wrongsv_vless_encoding::{LengthPacketReader, LengthPacketWriter, PacketReadError};

use crate::anytls;
use crate::config::ServerConfig;
use crate::endpoint::{
    Endpoint, HuOptions, MixedOptions, OuterSecurity, ProxyProtocol, ShadowsocksOptions, Transport,
    TrojanOptions, VlessOptions, WsOptions,
};
use crate::error::{ClientError, Result};
use crate::protocol::{
    encode_raw_vless_header, encode_udp_vless_header, read_raw_vless_response, Target,
};
use crate::reality;
use crate::shadowsocks as ss;
use crate::shadowtls;
use crate::tls;
use crate::trojan;
use crate::vision;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const MIXED_DETECT_TIMEOUT: Duration = Duration::from_millis(500);
const VISION_FLOW: &str = "xtls-rprx-vision";

pub trait TunnelReader: Read + Send {}

impl<T: Read + Send + ?Sized> TunnelReader for T {}

pub trait TunnelWriter: Write + Send {
    fn shutdown_write(&mut self) -> io::Result<()>;
}

pub trait Tunnel: Read + Write + Send {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>>;
    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)>;
    fn shutdown_write(&mut self) -> io::Result<()>;
    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UdpPacket {
    pub target: Target,
    pub payload: Vec<u8>,
}

pub trait UdpSession: Send {
    fn send_packet(&mut self, payload: &[u8]) -> Result<()>;
    fn try_recv_packet(&mut self) -> Result<Option<UdpPacket>>;
}

impl Tunnel for TcpStream {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(self.try_clone()?))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        split_cloneable_tunnel(self)
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        self.shutdown(Shutdown::Write)
    }

    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        TcpStream::set_read_timeout(self, read)?;
        TcpStream::set_write_timeout(self, write)?;
        Ok(())
    }
}

struct DefaultTunnelReader {
    inner: Box<dyn Tunnel>,
}

impl Read for DefaultTunnelReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

struct DefaultTunnelWriter {
    inner: Box<dyn Tunnel>,
}

impl Write for DefaultTunnelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl TunnelWriter for DefaultTunnelWriter {
    fn shutdown_write(&mut self) -> io::Result<()> {
        self.inner.shutdown_write()
    }
}

pub(crate) fn split_cloneable_tunnel<T: Tunnel + 'static>(
    inner: Box<T>,
) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
    let writer = inner.try_clone_box()?;
    let reader: Box<dyn Tunnel> = inner;
    Ok((
        Box::new(DefaultTunnelReader { inner: reader }),
        Box::new(DefaultTunnelWriter { inner: writer }),
    ))
}

#[derive(Clone, Debug)]
pub struct WrongsvClient {
    server: ServerConfig,
}

impl WrongsvClient {
    pub fn new(server: ServerConfig) -> Result<Self> {
        server.endpoint.validate()?;
        Ok(Self { server })
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.server.endpoint
    }

    pub fn stack_summary(&self) -> String {
        self.server.endpoint.stack_summary()
    }

    pub fn connect(&self, target: &Target) -> Result<Box<dyn Tunnel>> {
        match self.server.endpoint.proxy.clone() {
            ProxyProtocol::Vless(opts) => self.connect_vless(target, &opts),
            ProxyProtocol::Trojan(opts) => self.connect_trojan(target, &opts),
            ProxyProtocol::Mixed(opts) => self.connect_mixed(target, &opts),
            ProxyProtocol::Shadowsocks(opts) => self.connect_shadowsocks(target, &opts),
        }
    }

    pub fn supports_udp(&self) -> bool {
        match &self.server.endpoint.proxy {
            ProxyProtocol::Vless(opts) => opts.flow.trim().is_empty(),
            ProxyProtocol::Trojan(_) | ProxyProtocol::Shadowsocks(_) => true,
            ProxyProtocol::Mixed(_) => false,
        }
    }

    pub fn connect_udp_session(&self, target: &Target) -> Result<Box<dyn UdpSession>> {
        match self.server.endpoint.proxy.clone() {
            ProxyProtocol::Vless(opts) => self.connect_vless_udp(target, &opts),
            ProxyProtocol::Trojan(opts) => self.connect_trojan_udp(target, &opts),
            ProxyProtocol::Shadowsocks(opts) => self.connect_shadowsocks_udp(target, &opts),
            ProxyProtocol::Mixed(_) => Err(ClientError::UnsupportedProtocol(
                "remote mixed proxy does not support UDP ASSOCIATE in wrongcl yet".into(),
            )),
        }
    }

    pub fn probe(&self, target: &Target, payload: &str) -> Result<ProbeResult> {
        let payload = if payload.is_empty() {
            format!(
                "HEAD / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                target.host
            )
        } else {
            payload.to_string()
        };

        let mut tunnel = self.connect(target)?;
        tunnel.write_all(payload.as_bytes())?;
        tunnel.flush().ok();

        let mut buf = [0u8; 2048];
        let n = tunnel.read(&mut buf)?;
        if n == 0 {
            return Err(ClientError::Config(
                "probe target closed without response".into(),
            ));
        }

        Ok(ProbeResult {
            bytes_read: n,
            preview: String::from_utf8_lossy(&buf[..n]).to_string(),
        })
    }

    fn connect_vless(&self, target: &Target, opts: &VlessOptions) -> Result<Box<dyn Tunnel>> {
        let (mut stream, timeout_handle) = self.open_proxy_stack()?;
        let header = encode_raw_vless_header(&opts.uuid, target, &opts.flow)?;
        stream.write_all(&header)?;
        read_raw_vless_response(&mut stream)?;
        if let Some(handle) = timeout_handle {
            clear_timeouts(&handle)?;
        }
        let flow = opts.flow.trim();
        if flow == VISION_FLOW {
            stream = vision::wrap(stream, &opts.uuid)?;
        }
        Ok(stream)
    }

    fn connect_vless_udp(
        &self,
        target: &Target,
        opts: &VlessOptions,
    ) -> Result<Box<dyn UdpSession>> {
        if opts.flow.trim() == VISION_FLOW {
            return Err(ClientError::UnsupportedProtocol(
                "XTLS Vision does not support UDP".into(),
            ));
        }
        let (mut stream, timeout_handle) = self.open_proxy_stack()?;
        let header = encode_udp_vless_header(&opts.uuid, target, &opts.flow)?;
        stream.write_all(&header)?;
        read_raw_vless_response(&mut stream)?;
        if let Some(handle) = timeout_handle {
            clear_timeouts(&handle)?;
        }
        open_stream_udp_session(stream, target.clone())
    }

    fn connect_trojan(&self, target: &Target, opts: &TrojanOptions) -> Result<Box<dyn Tunnel>> {
        let (mut stream, timeout_handle) = self.open_proxy_stack()?;
        let header = trojan::encode_handshake(&opts.password, target)?;
        stream.write_all(&header)?;
        stream.flush().ok();
        if let Some(handle) = timeout_handle {
            clear_timeouts(&handle)?;
        }
        Ok(stream)
    }

    fn connect_trojan_udp(
        &self,
        target: &Target,
        opts: &TrojanOptions,
    ) -> Result<Box<dyn UdpSession>> {
        let (mut stream, timeout_handle) = self.open_proxy_stack()?;
        let header = trojan::encode_udp_associate_handshake(&opts.password)?;
        stream.write_all(&header)?;
        stream.flush().ok();
        if let Some(handle) = timeout_handle {
            clear_timeouts(&handle)?;
        }
        open_trojan_udp_session(stream, target.clone())
    }

    fn connect_mixed(&self, target: &Target, opts: &MixedOptions) -> Result<Box<dyn Tunnel>> {
        let mut tcp = self.connect_tcp_with_timeouts()?;
        tcp.set_read_timeout(Some(MIXED_DETECT_TIMEOUT))?;
        tcp.set_write_timeout(Some(MIXED_DETECT_TIMEOUT))?;
        match remote_socks5_connect(&mut tcp, opts, target) {
            Ok(()) => {
                clear_timeouts(&tcp)?;
                Ok(Box::new(tcp))
            }
            Err(socks_err) => {
                let mut http = self.connect_tcp_with_timeouts()?;
                match remote_http_connect(&mut http, opts, target) {
                    Ok(()) => {
                        clear_timeouts(&http)?;
                        Ok(Box::new(http))
                    }
                    Err(http_err) => Err(ClientError::Config(format!(
                        "remote mixed proxy connect failed: SOCKS5 path: {socks_err}; HTTP CONNECT path: {http_err}"
                    ))),
                }
            }
        }
    }

    fn connect_shadowsocks(
        &self,
        target: &Target,
        opts: &ShadowsocksOptions,
    ) -> Result<Box<dyn Tunnel>> {
        let tcp = self.connect_tcp_with_timeouts()?;
        let timeout_handle = tcp.try_clone()?;
        let inner: Box<dyn Tunnel> = Box::new(tcp);
        let tunnel = ss::open_tunnel(inner, opts, target)?;
        clear_timeouts(&timeout_handle)?;
        Ok(tunnel)
    }

    fn connect_shadowsocks_udp(
        &self,
        target: &Target,
        opts: &ShadowsocksOptions,
    ) -> Result<Box<dyn UdpSession>> {
        let config = wrongsv_shadowsocks::ServerConfig::new(&opts.method, opts.password.clone())
            .map_err(|e| ClientError::Config(format!("Shadowsocks: {e}")))?;
        let server_addr = format!("{}:{}", self.server.host, self.server.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| {
                ClientError::Config("failed to resolve Shadowsocks server address".into())
            })?;
        open_shadowsocks_udp_session(config, server_addr, target.clone())
    }

    fn open_proxy_stack(&self) -> Result<(Box<dyn Tunnel>, Option<TcpStream>)> {
        if let Transport::Xhttp(xopts) = &self.server.endpoint.transport {
            let stream = crate::xhttp::connect_xhttp(
                &self.server.host,
                self.server.port,
                xopts,
                &self.server.endpoint.outer_security,
            )?;
            return Ok((stream, None));
        }
        if let Transport::Grpc(gopts) = &self.server.endpoint.transport {
            let stream = crate::grpc::connect_grpc(
                &self.server.host,
                self.server.port,
                gopts,
                &self.server.endpoint.outer_security,
            )?;
            return Ok((stream, None));
        }
        let tcp = self.connect_tcp_with_timeouts()?;
        let timeout_handle = tcp.try_clone()?;
        let stream = self.wrap_outer_then_transport(tcp)?;
        Ok((stream, Some(timeout_handle)))
    }

    fn wrap_outer_then_transport(&self, tcp: TcpStream) -> Result<Box<dyn Tunnel>> {
        let outer = wrap_outer_security(tcp, &self.server.endpoint.outer_security)?;
        wrap_transport(
            outer,
            &self.server.endpoint.transport,
            &self.server.host,
            self.server.port,
        )
    }

    fn connect_tcp_with_timeouts(&self) -> Result<TcpStream> {
        let stream = connect_tcp(&self.server.host, self.server.port)?;
        stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
        stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;
        Ok(stream)
    }
}

fn wrap_outer_security(tcp: TcpStream, outer: &OuterSecurity) -> Result<Box<dyn Tunnel>> {
    match outer {
        OuterSecurity::None => Ok(Box::new(tcp)),
        OuterSecurity::Tls(opts) => tls::wrap(tcp, opts),
        OuterSecurity::Reality(opts) => reality::wrap(tcp, opts),
        OuterSecurity::AnyTls(opts) => anytls::wrap(tcp, opts),
        OuterSecurity::ShadowTls(opts) => shadowtls::wrap(tcp, opts),
    }
}

fn wrap_transport(
    inner: Box<dyn Tunnel>,
    transport: &Transport,
    server_host: &str,
    server_port: u16,
) -> Result<Box<dyn Tunnel>> {
    match transport {
        Transport::Raw => Ok(inner),
        Transport::Httpupgrade(opts) => connect_httpupgrade(inner, opts, server_host, server_port),
        Transport::Websocket(opts) => connect_websocket(inner, opts, server_host, server_port),
        Transport::Xhttp(_) => Err(ClientError::Config(
            "XHTTP transport must be opened via open_proxy_stack, not wrap_transport".into(),
        )),
        Transport::Grpc(_) => Err(ClientError::Config(
            "gRPC transport must be opened via open_proxy_stack, not wrap_transport".into(),
        )),
    }
}

fn connect_httpupgrade(
    mut inner: Box<dyn Tunnel>,
    opts: &HuOptions,
    server_host: &str,
    server_port: u16,
) -> Result<Box<dyn Tunnel>> {
    let path = normalized_path(&opts.path, "/");
    let host = host_header(opts.host.as_deref(), server_host, server_port);
    http_upgrade_handshake(inner.as_mut(), &path, host)?;
    Ok(inner)
}

fn connect_websocket(
    mut inner: Box<dyn Tunnel>,
    opts: &WsOptions,
    server_host: &str,
    server_port: u16,
) -> Result<Box<dyn Tunnel>> {
    let path = normalized_path(&opts.path, "/");
    let host = host_header(opts.host.as_deref(), server_host, server_port);
    websocket_handshake(inner.as_mut(), &path, host)?;
    Ok(Box::new(WebSocketTunnel::new(inner)))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProbeResult {
    pub bytes_read: usize,
    pub preview: String,
}

struct StreamUdpSession {
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

fn open_stream_udp_session(stream: Box<dyn Tunnel>, target: Target) -> Result<Box<dyn UdpSession>> {
    Ok(Box::new(StreamUdpSession::new(stream, target)?))
}

fn open_trojan_udp_session(stream: Box<dyn Tunnel>, target: Target) -> Result<Box<dyn UdpSession>> {
    Ok(Box::new(TrojanUdpSession::new(stream, target)?))
}

fn open_shadowsocks_udp_session(
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

fn clear_timeouts<T: Tunnel + ?Sized>(stream: &T) -> io::Result<()> {
    stream.set_socket_timeouts(None, None)
}

fn normalized_path(value: &str, default: &str) -> String {
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

fn host_header(explicit: Option<&str>, server_host: &str, server_port: u16) -> String {
    explicit
        .map(|value| value.to_string())
        .unwrap_or_else(|| format!("{server_host}:{server_port}"))
}

fn read_http_headers(stream: &mut dyn Read, context: &str) -> io::Result<String> {
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

fn http_upgrade_handshake(stream: &mut dyn Tunnel, path: &str, host: String) -> Result<()> {
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

fn websocket_handshake(stream: &mut dyn Tunnel, path: &str, host: String) -> Result<()> {
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

fn remote_socks5_connect(
    stream: &mut TcpStream,
    opts: &MixedOptions,
    target: &Target,
) -> Result<()> {
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
    read_socks_bound_address(stream, reply[3])?;
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

fn read_socks_bound_address(stream: &mut TcpStream, atyp: u8) -> Result<()> {
    match atyp {
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
        other => {
            return Err(ClientError::Config(format!(
                "remote SOCKS5 reply used unsupported address type {other:#04x}"
            )));
        }
    }
    Ok(())
}

fn remote_http_connect(stream: &mut TcpStream, opts: &MixedOptions, target: &Target) -> Result<()> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpCode {
    Close = 0x08,
    Ping = 0x09,
    Pong = 0x0a,
    Binary = 0x02,
}

struct WebSocketTunnel {
    inner: Box<dyn Tunnel>,
    read_buf: Vec<u8>,
}

impl WebSocketTunnel {
    fn new(inner: Box<dyn Tunnel>) -> Self {
        Self {
            inner,
            read_buf: Vec::new(),
        }
    }
}

impl Tunnel for WebSocketTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(Self {
            inner: self.inner.try_clone_box()?,
            read_buf: Vec::new(),
        }))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        split_cloneable_tunnel(self)
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        let _ = write_ws_frame(self.inner.as_mut(), &[], OpCode::Close, true);
        self.inner.shutdown_write()
    }

    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        self.inner.set_socket_timeouts(read, write)
    }
}

impl Read for WebSocketTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.read_buf.is_empty() {
            let n = self.read_buf.len().min(buf.len());
            buf[..n].copy_from_slice(&self.read_buf[..n]);
            self.read_buf.drain(..n);
            return Ok(n);
        }

        loop {
            let (opcode, payload) = read_ws_frame(self.inner.as_mut())?;
            match opcode {
                OpCode::Binary => {
                    let n = payload.len().min(buf.len());
                    buf[..n].copy_from_slice(&payload[..n]);
                    if n < payload.len() {
                        self.read_buf.extend_from_slice(&payload[n..]);
                    }
                    return Ok(n);
                }
                OpCode::Close => return Ok(0),
                OpCode::Ping => {
                    write_ws_frame(self.inner.as_mut(), &payload, OpCode::Pong, true)?;
                }
                OpCode::Pong => {}
            }
        }
    }
}

impl Write for WebSocketTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_ws_frame(self.inner.as_mut(), buf, OpCode::Binary, true)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn write_ws_frame(
    stream: &mut dyn Write,
    payload: &[u8],
    opcode: OpCode,
    masked: bool,
) -> io::Result<()> {
    let mut header = Vec::with_capacity(14);
    header.push(0x80 | opcode as u8);
    let mask_bit = if masked { 0x80 } else { 0x00 };
    match payload.len() {
        len if len < 126 => header.push(mask_bit | len as u8),
        len if len <= u16::MAX as usize => {
            header.push(mask_bit | 126);
            header.extend_from_slice(&(len as u16).to_be_bytes());
        }
        len => {
            header.push(mask_bit | 127);
            header.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }

    if masked {
        let mut mask = [0u8; 4];
        rand::thread_rng().fill_bytes(&mut mask);
        header.extend_from_slice(&mask);
        let mut masked_payload = Vec::with_capacity(payload.len());
        for (idx, byte) in payload.iter().enumerate() {
            masked_payload.push(byte ^ mask[idx % 4]);
        }
        stream.write_all(&header)?;
        stream.write_all(&masked_payload)?;
    } else {
        stream.write_all(&header)?;
        stream.write_all(payload)?;
    }
    stream.flush()
}

fn read_ws_frame(stream: &mut dyn Read) -> io::Result<(OpCode, Vec<u8>)> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;
    let opcode = match header[0] & 0x0f {
        0x02 => OpCode::Binary,
        0x08 => OpCode::Close,
        0x09 => OpCode::Ping,
        0x0a => OpCode::Pong,
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported WebSocket opcode {other:#x}"),
            ));
        }
    };
    let masked = header[1] & 0x80 != 0;
    let mut len = (header[1] & 0x7f) as u64;
    if len == 126 {
        let mut extended = [0u8; 2];
        stream.read_exact(&mut extended)?;
        len = u16::from_be_bytes(extended) as u64;
    } else if len == 127 {
        let mut extended = [0u8; 8];
        stream.read_exact(&mut extended)?;
        len = u64::from_be_bytes(extended);
    }

    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }

    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload)?;
    if masked {
        for (idx, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[idx % 4];
        }
    }
    Ok((opcode, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ClientConfig, LocalProxyConfig};
    use crate::endpoint::{
        Endpoint, HuOptions, MixedOptions, ProxyProtocol, ShadowsocksOptions, Transport,
        VlessOptions, WsOptions,
    };
    use crate::proxy::{ProxyHandle, ProxySnapshot};
    use std::net::{SocketAddr, TcpListener};
    use std::sync::mpsc;
    use std::thread;

    const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

    fn vless_server(host: &str, port: u16, uuid: &str, transport: Transport) -> ServerConfig {
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

    fn mixed_server(host: &str, port: u16, opts: MixedOptions) -> ServerConfig {
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

    fn shadowsocks_server(host: &str, port: u16, opts: ShadowsocksOptions) -> ServerConfig {
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

    #[test]
    fn probe_works_against_fake_raw_vless_server() {
        let server = spawn_fake_server(FakeCarrier::Raw);
        let client = WrongsvClient::new(vless_server(
            "127.0.0.1",
            server.port,
            TEST_UUID,
            Transport::Raw,
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();
        assert_eq!(result.bytes_read, 4);
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn probe_works_against_fake_httpupgrade_server() {
        let server = spawn_fake_server(FakeCarrier::HttpUpgrade);
        let client = WrongsvClient::new(vless_server(
            "127.0.0.1",
            server.port,
            TEST_UUID,
            Transport::Httpupgrade(HuOptions {
                path: "/up".into(),
                host: None,
            }),
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn socks_proxy_udp_works_against_fake_httpupgrade_server() {
        let server = spawn_fake_server(FakeCarrier::HttpUpgrade);
        let client = WrongsvClient::new(vless_server(
            "127.0.0.1",
            server.port,
            TEST_UUID,
            Transport::Httpupgrade(HuOptions {
                path: "/up".into(),
                host: None,
            }),
        ))
        .unwrap();

        let mut session = client
            .connect_udp_session(&Target::new("example.com", 53).unwrap())
            .unwrap();
        session.send_packet(b"ping-udp").unwrap();
        for _ in 0..20 {
            if let Some(packet) = session.try_recv_packet().unwrap() {
                assert_eq!(packet.payload, b"ping-udp");
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("no UDP response from HTTPUpgrade session");
    }

    #[test]
    fn socks_proxy_works_against_fake_httpupgrade_server() {
        let server = spawn_fake_server(FakeCarrier::HttpUpgrade);
        let mut proxy = ProxyHandle::start(ClientConfig {
            server: vless_server(
                "127.0.0.1",
                server.port,
                TEST_UUID,
                Transport::Httpupgrade(HuOptions {
                    path: "/up".into(),
                    host: None,
                }),
            ),
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
        })
        .unwrap();

        let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-httpup").unwrap();
        proxy.stop().unwrap();

        assert_eq!(response, b"hello-httpup".to_vec());
    }

    #[test]
    fn probe_works_against_fake_websocket_server() {
        let server = spawn_fake_server(FakeCarrier::WebSocket);
        let client = WrongsvClient::new(vless_server(
            "127.0.0.1",
            server.port,
            TEST_UUID,
            Transport::Websocket(WsOptions {
                path: "/ws".into(),
                host: None,
            }),
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn socks_proxy_udp_works_against_fake_websocket_server() {
        let server = spawn_fake_server(FakeCarrier::WebSocket);
        let client = WrongsvClient::new(vless_server(
            "127.0.0.1",
            server.port,
            TEST_UUID,
            Transport::Websocket(WsOptions {
                path: "/ws".into(),
                host: None,
            }),
        ))
        .unwrap();

        let mut session = client
            .connect_udp_session(&Target::new("example.com", 53).unwrap())
            .unwrap();
        session.send_packet(b"ping-udp").unwrap();
        for _ in 0..20 {
            if let Some(packet) = session.try_recv_packet().unwrap() {
                assert_eq!(packet.payload, b"ping-udp");
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("no UDP response from WebSocket session");
    }

    #[test]
    fn socks_proxy_works_against_fake_websocket_server() {
        let server = spawn_fake_server(FakeCarrier::WebSocket);
        let mut proxy = ProxyHandle::start(ClientConfig {
            server: vless_server(
                "127.0.0.1",
                server.port,
                TEST_UUID,
                Transport::Websocket(WsOptions {
                    path: "/ws".into(),
                    host: None,
                }),
            ),
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
        })
        .unwrap();

        let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-ws").unwrap();
        proxy.stop().unwrap();

        assert_eq!(response, b"hello-ws".to_vec());
    }

    #[test]
    fn probe_works_against_fake_remote_socks5_server() {
        let server = spawn_fake_socks5_server(None, None);
        let client = WrongsvClient::new(mixed_server(
            "127.0.0.1",
            server.port,
            MixedOptions::default(),
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn probe_works_against_fake_authenticated_remote_socks5_server() {
        let server = spawn_fake_socks5_server(Some("user"), Some("pass"));
        let client = WrongsvClient::new(mixed_server(
            "127.0.0.1",
            server.port,
            MixedOptions {
                username: Some("user".into()),
                password: Some("pass".into()),
            },
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn probe_works_against_fake_remote_http_connect_server() {
        let server = spawn_fake_http_connect_server(None, None);
        let client = WrongsvClient::new(mixed_server(
            "127.0.0.1",
            server.port,
            MixedOptions::default(),
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn probe_works_against_fake_authenticated_remote_http_connect_server() {
        let server = spawn_fake_http_connect_server(Some("user"), Some("pass"));
        let client = WrongsvClient::new(mixed_server(
            "127.0.0.1",
            server.port,
            MixedOptions {
                username: Some("user".into()),
                password: Some("pass".into()),
            },
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn probe_works_against_fake_shadowsocks_server() {
        let server =
            spawn_fake_shadowsocks_server("chacha20-ietf-poly1305".into(), "hunter2".into());
        let client = WrongsvClient::new(shadowsocks_server(
            "127.0.0.1",
            server.port,
            ShadowsocksOptions {
                method: "chacha20-ietf-poly1305".into(),
                password: "hunter2".into(),
            },
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping")
            .unwrap();
        assert_eq!(result.bytes_read, 4);
        assert_eq!(result.preview, "ping");
    }

    #[test]
    fn probe_works_against_fake_shadowsocks_aead_2022_server() {
        let psk_b64 = "AAAAAAAAAAAAAAAAAAAAAA==";
        let server =
            spawn_fake_shadowsocks_server("2022-blake3-aes-128-gcm".into(), psk_b64.into());
        let client = WrongsvClient::new(shadowsocks_server(
            "127.0.0.1",
            server.port,
            ShadowsocksOptions {
                method: "2022-blake3-aes-128-gcm".into(),
                password: psk_b64.into(),
            },
        ))
        .unwrap();

        let result = client
            .probe(&Target::new("example.com", 80).unwrap(), "ping-2022")
            .unwrap();
        assert_eq!(result.preview, "ping-2022");
    }

    #[test]
    fn socks_proxy_works_against_fake_shadowsocks_server() {
        let server =
            spawn_fake_shadowsocks_server("chacha20-ietf-poly1305".into(), "hunter2".into());
        let mut proxy = ProxyHandle::start(ClientConfig {
            server: shadowsocks_server(
                "127.0.0.1",
                server.port,
                ShadowsocksOptions {
                    method: "chacha20-ietf-poly1305".into(),
                    password: "hunter2".into(),
                },
            ),
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
        })
        .unwrap();

        let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-ss").unwrap();
        proxy.stop().unwrap();

        assert_eq!(response, b"hello-ss".to_vec());
    }

    #[test]
    fn socks_proxy_works_against_fake_shadowsocks_aead_2022_server() {
        let psk_b64 = "AAAAAAAAAAAAAAAAAAAAAA==";
        let server =
            spawn_fake_shadowsocks_server("2022-blake3-aes-128-gcm".into(), psk_b64.into());
        let mut proxy = ProxyHandle::start(ClientConfig {
            server: shadowsocks_server(
                "127.0.0.1",
                server.port,
                ShadowsocksOptions {
                    method: "2022-blake3-aes-128-gcm".into(),
                    password: psk_b64.into(),
                },
            ),
            local: LocalProxyConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
        })
        .unwrap();

        let response = run_socks_echo(proxy.snapshot().socket_addr(), b"hello-2022").unwrap();
        proxy.stop().unwrap();

        assert_eq!(response, b"hello-2022".to_vec());
    }

    fn run_socks_echo(local_addr: SocketAddr, payload: &[u8]) -> io::Result<Vec<u8>> {
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

    fn spawn_fake_shadowsocks_server(method: String, password: String) -> FakeServer {
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
            parse_request_header, ServerConfig as SsServerConfig, ShadowsocksReader,
            ShadowsocksWriter,
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

    enum FakeCarrier {
        Raw,
        HttpUpgrade,
        WebSocket,
    }

    struct FakeServer {
        port: u16,
    }

    trait SnapshotAddr {
        fn socket_addr(&self) -> SocketAddr;
    }

    impl SnapshotAddr for ProxySnapshot {
        fn socket_addr(&self) -> SocketAddr {
            format!("{}:{}", self.local_host, self.local_port)
                .parse()
                .unwrap()
        }
    }

    fn spawn_fake_server(carrier: FakeCarrier) -> FakeServer {
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

    fn spawn_fake_socks5_server(username: Option<&str>, password: Option<&str>) -> FakeServer {
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

    fn spawn_fake_http_connect_server(
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
        assert_eq!(request[1], 0x01);
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
}
