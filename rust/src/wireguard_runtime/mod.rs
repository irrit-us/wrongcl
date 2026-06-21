mod config;
mod device;
mod engine;
mod port_pool;
mod tcp;
mod udp;

use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Context, Result};
use bytes::Bytes;
use smoltcp::iface::{Interface, SocketSet};
use smoltcp::wire::{IpAddress, IpCidr, Ipv4Address, Ipv6Address};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};

use crate::protocol::Target;

pub(crate) use self::config::WireGuardRuntimeConfig;
use self::device::ChannelIpDevice;
use self::engine::WireGuardEngine;
use self::port_pool::{PortPool, PortProtocol, VirtualPort};
use self::tcp::{TcpCommand, TcpInterface};
use self::udp::{UdpCommand, UdpInterface};

#[derive(Clone, Copy, Debug)]
pub struct TargetRoute {
    pub source_ip: IpAddr,
    pub remote_addr: SocketAddr,
}

pub struct WireGuardRuntime {
    config: Arc<WireGuardRuntimeConfig>,
    _runtime: Runtime,
    tcp_commands: mpsc::UnboundedSender<TcpCommand>,
    udp_commands: mpsc::UnboundedSender<UdpCommand>,
    tcp_ports: Arc<PortPool>,
    udp_ports: Arc<PortPool>,
}

pub struct TcpSession {
    writer: TcpSessionWriter,
    inbound_rx: mpsc::UnboundedReceiver<Bytes>,
}

#[derive(Clone)]
pub struct TcpSessionWriter {
    state: Arc<TcpSessionState>,
}

struct TcpSessionState {
    port: Mutex<Option<VirtualPort>>,
    commands: mpsc::UnboundedSender<TcpCommand>,
    pool: Arc<PortPool>,
}

pub struct TcpSessionReader {
    inbound_rx: mpsc::UnboundedReceiver<Bytes>,
}

pub struct UdpSession {
    writer: UdpSessionWriter,
    inbound_rx: mpsc::UnboundedReceiver<Bytes>,
}

#[derive(Clone)]
pub struct UdpSessionWriter {
    state: Arc<UdpSessionState>,
}

struct UdpSessionState {
    port: Mutex<Option<VirtualPort>>,
    commands: mpsc::UnboundedSender<UdpCommand>,
    pool: Arc<PortPool>,
}

impl WireGuardRuntime {
    pub fn start(config: WireGuardRuntimeConfig) -> Result<Self> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("build WireGuard runtime")?;
        let handle = runtime.handle().clone();

        let config = Arc::new(config);
        let tcp_ports = Arc::new(PortPool::new(PortProtocol::Tcp));
        let udp_ports = Arc::new(PortPool::new(PortProtocol::Udp));

        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let (tcp_device, tcp_feed) = ChannelIpDevice::new(config.mtu, outbound_tx.clone());
        let (udp_device, udp_feed) = ChannelIpDevice::new(config.mtu, outbound_tx);

        let engine =
            Arc::new(runtime.block_on(WireGuardEngine::new(config.clone(), tcp_feed, udp_feed))?);
        engine.spawn(&handle, outbound_rx);

        let (tcp_commands, tcp_command_rx) = mpsc::unbounded_channel();
        let (udp_commands, udp_command_rx) = mpsc::unbounded_channel();

        TcpInterface::spawn(
            &handle,
            config.client_addresses.clone(),
            tcp_device,
            tcp_command_rx,
        );
        UdpInterface::spawn(
            &handle,
            config.client_addresses.clone(),
            udp_device,
            udp_command_rx,
        );

        Ok(Self {
            config,
            _runtime: runtime,
            tcp_commands,
            udp_commands,
            tcp_ports,
            udp_ports,
        })
    }

    pub fn resolve_target(&self, target: &Target) -> Result<TargetRoute> {
        if let Ok(ip) = target.host.parse::<IpAddr>() {
            return self.route_for(SocketAddr::new(ip, target.port));
        }

        let addrs = format!("{}:{}", target.host, target.port)
            .to_socket_addrs()
            .with_context(|| format!("failed to resolve {}", target.host))?;

        for addr in addrs {
            if let Ok(route) = self.route_for(addr) {
                return Ok(route);
            }
        }

        bail!("no routable address resolved for {}", target.host);
    }

    pub fn open_tcp(&self, route: TargetRoute) -> Result<TcpSession> {
        let port = self.tcp_ports.acquire()?;
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        self.tcp_commands
            .send(TcpCommand::Open {
                port,
                route,
                inbound: inbound_tx,
            })
            .map_err(|_| anyhow!("TCP interface command channel is closed"))?;

        Ok(TcpSession {
            writer: TcpSessionWriter {
                state: Arc::new(TcpSessionState {
                    port: Mutex::new(Some(port)),
                    commands: self.tcp_commands.clone(),
                    pool: self.tcp_ports.clone(),
                }),
            },
            inbound_rx,
        })
    }

    pub fn open_udp(&self, route: TargetRoute) -> Result<UdpSession> {
        let port = self.udp_ports.acquire()?;
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        self.udp_commands
            .send(UdpCommand::Open {
                port,
                route,
                inbound: inbound_tx,
            })
            .map_err(|_| anyhow!("UDP interface command channel is closed"))?;

        Ok(UdpSession {
            writer: UdpSessionWriter {
                state: Arc::new(UdpSessionState {
                    port: Mutex::new(Some(port)),
                    commands: self.udp_commands.clone(),
                    pool: self.udp_ports.clone(),
                }),
            },
            inbound_rx,
        })
    }

    fn route_for(&self, remote_addr: SocketAddr) -> Result<TargetRoute> {
        let target_ip = remote_addr.ip();
        if !self.config.allows_target(target_ip) {
            bail!("target {target_ip} is outside allowed_ips");
        }

        let source_ip = self
            .config
            .select_source_ip(target_ip)
            .with_context(|| format!("no client address matches target family for {target_ip}"))?;

        Ok(TargetRoute {
            source_ip,
            remote_addr,
        })
    }
}

impl TcpSession {
    pub fn split(self) -> (TcpSessionWriter, TcpSessionReader) {
        (
            self.writer,
            TcpSessionReader {
                inbound_rx: self.inbound_rx,
            },
        )
    }
}

impl TcpSessionWriter {
    pub fn send(&self, data: Bytes) -> Result<()> {
        let Some(port) = self.current_port()? else {
            bail!("TCP session is closed");
        };
        self.state
            .commands
            .send(TcpCommand::Send { port, data })
            .map_err(|_| anyhow!("TCP interface command channel is closed"))
    }

    pub fn shutdown(&self) {
        if let Ok(Some(port)) = self.current_port() {
            let _ = self.state.commands.send(TcpCommand::Close { port });
        }
    }

    fn current_port(&self) -> Result<Option<VirtualPort>> {
        self.state
            .port
            .lock()
            .map(|guard| *guard)
            .map_err(|_| anyhow!("TCP session state lock is poisoned"))
    }
}

impl Drop for TcpSessionState {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.port.lock() {
            if let Some(port) = guard.take() {
                let _ = self.commands.send(TcpCommand::Close { port });
                self.pool.release(port);
            }
        }
    }
}

impl TcpSessionReader {
    pub fn blocking_recv(&mut self) -> Option<Bytes> {
        self.inbound_rx.blocking_recv()
    }
}

impl UdpSession {
    pub fn send(&self, data: Bytes) -> Result<()> {
        self.writer.send(data)
    }

    pub fn try_recv(&mut self) -> Result<Option<Bytes>> {
        match self.inbound_rx.try_recv() {
            Ok(bytes) => Ok(Some(bytes)),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => Ok(None),
        }
    }
}

impl UdpSessionWriter {
    fn send(&self, data: Bytes) -> Result<()> {
        let Some(port) = self.current_port()? else {
            bail!("UDP session is closed");
        };
        self.state
            .commands
            .send(UdpCommand::Send { port, data })
            .map_err(|_| anyhow!("UDP interface command channel is closed"))
    }

    fn current_port(&self) -> Result<Option<VirtualPort>> {
        self.state
            .port
            .lock()
            .map(|guard| *guard)
            .map_err(|_| anyhow!("UDP session state lock is poisoned"))
    }
}

impl Drop for UdpSessionState {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.port.lock() {
            if let Some(port) = guard.take() {
                let _ = self.commands.send(UdpCommand::Close { port });
                self.pool.release(port);
            }
        }
    }
}

pub fn configure_interface(interface: &mut Interface, source_ips: &[IpAddr]) -> Result<()> {
    let mut ip_slots_exhausted = false;
    interface.update_ip_addrs(|ip_addrs| {
        ip_addrs.clear();
        for ip in source_ips {
            let cidr = IpCidr::new(IpAddress::from(*ip), addr_prefix_len(*ip));
            if ip_addrs.push(cidr).is_err() {
                ip_slots_exhausted = true;
                break;
            }
        }
    });
    if ip_slots_exhausted {
        bail!("smoltcp interface ran out of source address slots");
    }

    let mut configured_ipv4 = false;
    let mut configured_ipv6 = false;
    for ip in source_ips {
        match ip {
            IpAddr::V4(addr) if !configured_ipv4 => {
                interface
                    .routes_mut()
                    .add_default_ipv4_route(Ipv4Address::from(addr.octets()))
                    .map_err(|_| anyhow!("smoltcp IPv4 route table is full"))?;
                configured_ipv4 = true;
            }
            IpAddr::V6(addr) if !configured_ipv6 => {
                interface
                    .routes_mut()
                    .add_default_ipv6_route(Ipv6Address::from(addr.octets()))
                    .map_err(|_| anyhow!("smoltcp IPv6 route table is full"))?;
                configured_ipv6 = true;
            }
            _ => {}
        }
    }

    Ok(())
}

fn addr_prefix_len(addr: IpAddr) -> u8 {
    if addr.is_ipv4() {
        32
    } else {
        128
    }
}

fn poll_deadline(iface: &mut Interface, sockets: &SocketSet<'_>) -> Option<Instant> {
    match iface.poll_delay(smoltcp::time::Instant::now(), sockets) {
        Some(smoltcp::time::Duration::ZERO) => None,
        Some(delay) => Some(Instant::now() + Duration::from_millis(delay.total_millis())),
        None => None,
    }
}

fn sleep_until(next_poll: Option<Instant>, has_sessions: bool) -> tokio::time::Sleep {
    match (next_poll, has_sessions) {
        (Some(deadline), _) => tokio::time::sleep_until(deadline),
        (None, true) => tokio::time::sleep(Duration::ZERO),
        (None, false) => tokio::time::sleep(Duration::from_secs(24 * 60 * 60)),
    }
}
