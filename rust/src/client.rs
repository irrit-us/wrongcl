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
    Endpoint, HuOptions, Hysteria2Options, MixedOptions, OuterSecurity, ProxyProtocol,
    ShadowsocksOptions, Transport, TrojanOptions, VlessOptions, WsOptions,
};
use crate::error::{ClientError, Result};
use crate::gdocsviewer;
use crate::hysteria2;
use crate::kcp;
use crate::meek;
use crate::naive;
use crate::protocol::{
    Target, encode_raw_vless_header, encode_udp_vless_header, read_raw_vless_response,
};
use crate::quic;
use crate::reality;
use crate::shadowsocks as ss;
use crate::shadowtls;
use crate::tls;
use crate::trojan;
use crate::tuic;
use crate::vision;
use crate::webtransport;
use crate::wireguard;

mod protocols;
mod remote;
mod udp;
mod websocket;

pub(crate) use self::remote::connect_tcp;
#[cfg(test)]
use self::remote::{encode_socks5_udp_packet, parse_socks5_udp_packet, read_http_headers};
#[cfg(test)]
use self::websocket::{OpCode, read_ws_frame, write_ws_frame};

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
            ProxyProtocol::Naive(opts) => self.connect_naive(target, &opts),
            ProxyProtocol::Hysteria2(opts) => self.connect_hysteria2(target, &opts),
            ProxyProtocol::Tuic(opts) => self.connect_tuic(target, &opts),
            ProxyProtocol::Trojan(opts) => self.connect_trojan(target, &opts),
            ProxyProtocol::Mixed(opts) => self.connect_mixed(target, &opts),
            ProxyProtocol::Shadowsocks(opts) => self.connect_shadowsocks(target, &opts),
            ProxyProtocol::Wireguard(opts) => self.connect_wireguard(target, &opts),
        }
    }

    pub fn supports_udp(&self) -> bool {
        match &self.server.endpoint.proxy {
            ProxyProtocol::Vless(opts) => {
                if !opts.flow.trim().is_empty() {
                    return false;
                }
                match &self.server.endpoint.transport {
                    Transport::Kcp(_) => true,
                    Transport::Quic(opts) => opts.udp_enabled,
                    Transport::Webtransport(opts) => opts.udp_enabled,
                    _ => true,
                }
            }
            ProxyProtocol::Hysteria2(opts) => opts.udp_enabled,
            ProxyProtocol::Naive(_) => false,
            ProxyProtocol::Tuic(_)
            | ProxyProtocol::Trojan(_)
            | ProxyProtocol::Shadowsocks(_)
            | ProxyProtocol::Wireguard(_) => true,
            ProxyProtocol::Mixed(_) => true,
        }
    }

    pub fn connect_udp_session(&self, target: &Target) -> Result<Box<dyn UdpSession>> {
        match self.server.endpoint.proxy.clone() {
            ProxyProtocol::Vless(opts) => self.connect_vless_udp(target, &opts),
            ProxyProtocol::Naive(opts) => self.connect_naive_udp(target, &opts),
            ProxyProtocol::Hysteria2(opts) => self.connect_hysteria2_udp(target, &opts),
            ProxyProtocol::Tuic(opts) => self.connect_tuic_udp(target, &opts),
            ProxyProtocol::Trojan(opts) => self.connect_trojan_udp(target, &opts),
            ProxyProtocol::Shadowsocks(opts) => self.connect_shadowsocks_udp(target, &opts),
            ProxyProtocol::Mixed(opts) => self.connect_mixed_udp(target, &opts),
            ProxyProtocol::Wireguard(opts) => self.connect_wireguard_udp(target, &opts),
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProbeResult {
    pub bytes_read: usize,
    pub preview: String,
}

#[cfg(test)]
mod tests;
