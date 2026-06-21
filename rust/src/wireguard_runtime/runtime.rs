use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use bytes::Bytes;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;

use crate::protocol::Target;

use super::config::WireGuardRuntimeConfig;
use super::device::ChannelIpDevice;
use super::engine::WireGuardEngine;
use super::port_pool::{PortPool, PortProtocol};
use super::session::{
    TcpSession, TcpSessionState, TcpSessionWriter, UdpSession, UdpSessionState, UdpSessionWriter,
};
use super::tcp::{TcpCommand, TcpInterface};
use super::udp::{UdpCommand, UdpInterface};

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
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel::<Bytes>();
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
                    port: std::sync::Mutex::new(Some(port)),
                    commands: self.tcp_commands.clone(),
                    pool: self.tcp_ports.clone(),
                }),
            },
            inbound_rx,
        })
    }

    pub fn open_udp(&self, route: TargetRoute) -> Result<UdpSession> {
        let port = self.udp_ports.acquire()?;
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel::<Bytes>();
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
                    port: std::sync::Mutex::new(Some(port)),
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
