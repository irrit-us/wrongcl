use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use boringtun::noise::errors::WireGuardError;
use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use bytes::Bytes;
use smoltcp::wire::{IpProtocol, IpVersion, Ipv4Packet, Ipv6Packet};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, trace, warn};

use crate::config::AppConfig;

use super::device::DeviceFeed;
use super::port_pool::PortProtocol;

const MAX_PACKET: usize = 65_536;

pub struct WireGuardEngine {
    local_addrs: HashSet<IpAddr>,
    peer: Mutex<Tunn>,
    udp: UdpSocket,
    endpoint: std::net::SocketAddr,
    tcp_feed: DeviceFeed,
    udp_feed: DeviceFeed,
}

impl WireGuardEngine {
    pub fn new(config: Arc<AppConfig>, tcp_feed: DeviceFeed, udp_feed: DeviceFeed) -> Result<Self> {
        let peer = Tunn::new(
            StaticSecret::from(config.private_key),
            PublicKey::from(config.peer_public_key),
            config.pre_shared_key,
            config.keep_alive,
            0,
            None,
        );

        let bind_addr = if config.server_endpoint.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };

        let udp = std::net::UdpSocket::bind(bind_addr)?;
        udp.set_nonblocking(true)?;
        let udp = UdpSocket::from_std(udp)?;

        Ok(Self {
            local_addrs: config.client_addresses.iter().copied().collect(),
            peer: Mutex::new(peer),
            udp,
            endpoint: config.server_endpoint,
            tcp_feed,
            udp_feed,
        })
    }

    pub fn spawn(self: Arc<Self>, outbound_rx: mpsc::UnboundedReceiver<Vec<u8>>) {
        let produce = Arc::clone(&self);
        tokio::spawn(async move { produce.run_producer(outbound_rx).await });

        let routine = Arc::clone(&self);
        tokio::spawn(async move { routine.run_routine().await });

        let consume = Arc::clone(&self);
        tokio::spawn(async move { consume.run_consumer().await });
    }

    async fn run_producer(self: Arc<Self>, mut outbound_rx: mpsc::UnboundedReceiver<Vec<u8>>) {
        while let Some(packet) = outbound_rx.recv().await {
            if let Err(error) = self.send_ip_packet(&packet).await {
                error!("failed to send IP packet through WireGuard: {error:#}");
            }
        }
    }

    async fn run_routine(self: Arc<Self>) {
        loop {
            let mut send_buf = [0u8; MAX_PACKET];
            let result = { self.peer.lock().await.update_timers(&mut send_buf) };
            self.handle_routine_result(result).await;
        }
    }

    async fn run_consumer(self: Arc<Self>) {
        loop {
            let mut recv_buf = [0u8; MAX_PACKET];
            let mut send_buf = [0u8; MAX_PACKET];

            let size = match self.udp.recv(&mut recv_buf).await {
                Ok(size) => size,
                Err(error) => {
                    error!("failed to read from WireGuard UDP socket: {error:#}");
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    continue;
                }
            };

            let packet = &recv_buf[..size];
            let result = {
                let mut peer = self.peer.lock().await;
                peer.decapsulate(None, packet, &mut send_buf)
            };

            match result {
                TunnResult::WriteToNetwork(outbound) => {
                    if let Err(error) = self.udp.send_to(outbound, self.endpoint).await {
                        error!("failed to send decapsulation follow-up packet: {error:#}");
                        continue;
                    }

                    let mut peer = self.peer.lock().await;
                    loop {
                        let mut inner_buf = [0u8; MAX_PACKET];
                        match peer.decapsulate(None, &[], &mut inner_buf) {
                            TunnResult::WriteToNetwork(outbound) => {
                                if let Err(error) = self.udp.send_to(outbound, self.endpoint).await
                                {
                                    error!(
                                        "failed to flush queued decapsulation packet: {error:#}"
                                    );
                                    break;
                                }
                            }
                            _ => break,
                        }
                    }
                }
                TunnResult::WriteToTunnelV4(inner, _) | TunnResult::WriteToTunnelV6(inner, _) => {
                    trace!("received decapsulated IP packet ({} bytes)", inner.len());
                    if let Some(protocol) = self.route_protocol(inner) {
                        let bytes = Bytes::copy_from_slice(inner);
                        match protocol {
                            PortProtocol::Tcp => self.tcp_feed.push(bytes),
                            PortProtocol::Udp => self.udp_feed.push(bytes),
                        }
                    }
                }
                TunnResult::Done => {}
                TunnResult::Err(error) => {
                    debug!("ignoring WireGuard decapsulation error: {error:?}");
                }
            }
        }
    }

    async fn send_ip_packet(&self, packet: &[u8]) -> Result<()> {
        let mut send_buf = [0u8; MAX_PACKET];
        let result = {
            let mut peer = self.peer.lock().await;
            peer.encapsulate(packet, &mut send_buf)
        };

        match result {
            TunnResult::WriteToNetwork(outbound) => {
                self.udp.send_to(outbound, self.endpoint).await?;
            }
            TunnResult::Done => {}
            TunnResult::Err(error) => {
                debug!("ignoring WireGuard encapsulation error: {error:?}");
            }
            other => {
                warn!("unexpected WireGuard encapsulation state: {other:?}");
            }
        }

        Ok(())
    }

    async fn handle_routine_result<'a>(&self, result: TunnResult<'a>) {
        match result {
            TunnResult::WriteToNetwork(packet) => {
                if let Err(error) = self.udp.send_to(packet, self.endpoint).await {
                    error!("failed to send WireGuard routine packet: {error:#}");
                }
            }
            TunnResult::Err(WireGuardError::ConnectionExpired) => {
                warn!("WireGuard handshake expired; re-initiating");
                let mut buffer = [0u8; MAX_PACKET];
                let result = self
                    .peer
                    .lock()
                    .await
                    .format_handshake_initiation(&mut buffer, false);
                Box::pin(self.handle_routine_result(result)).await;
            }
            TunnResult::Done => {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            TunnResult::Err(error) => {
                debug!("ignoring WireGuard timer error: {error:?}");
            }
            other => {
                warn!("unexpected WireGuard timer state: {other:?}");
            }
        }
    }

    fn route_protocol(&self, packet: &[u8]) -> Option<PortProtocol> {
        match IpVersion::of_packet(packet) {
            Ok(IpVersion::Ipv4) => {
                let packet = Ipv4Packet::new_checked(packet).ok()?;
                if !self.local_addrs.contains(&IpAddr::V4(packet.dst_addr())) {
                    return None;
                }
                match packet.next_header() {
                    IpProtocol::Tcp => Some(PortProtocol::Tcp),
                    IpProtocol::Udp => Some(PortProtocol::Udp),
                    _ => None,
                }
            }
            Ok(IpVersion::Ipv6) => {
                let packet = Ipv6Packet::new_checked(packet).ok()?;
                if !self.local_addrs.contains(&IpAddr::V6(packet.dst_addr())) {
                    return None;
                }
                match packet.next_header() {
                    IpProtocol::Tcp => Some(PortProtocol::Tcp),
                    IpProtocol::Udp => Some(PortProtocol::Udp),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
