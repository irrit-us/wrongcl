use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;

use anyhow::{Context, Result};
use bytes::Bytes;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::udp::{self, UdpMetadata};
use smoltcp::wire::{HardwareAddress, IpAddress};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tracing::{debug, error};

use super::device::ChannelIpDevice;
use super::port_pool::VirtualPort;
use super::{TargetRoute, configure_interface};

const MAX_PACKET: usize = 65_536;
const UDP_META_CAPACITY: usize = 16;

pub enum UdpCommand {
    Open {
        port: VirtualPort,
        route: TargetRoute,
        inbound: mpsc::UnboundedSender<Bytes>,
    },
    Send {
        port: VirtualPort,
        data: Bytes,
    },
    Close {
        port: VirtualPort,
    },
}

struct UdpHandle {
    socket: SocketHandle,
    target: std::net::SocketAddr,
    inbound: mpsc::UnboundedSender<Bytes>,
}

pub struct UdpInterface {
    source_ips: Vec<IpAddr>,
    device: ChannelIpDevice,
    commands: mpsc::UnboundedReceiver<UdpCommand>,
    sockets: SocketSet<'static>,
}

impl UdpInterface {
    pub fn spawn(
        handle: &Handle,
        source_ips: Vec<IpAddr>,
        device: ChannelIpDevice,
        commands: mpsc::UnboundedReceiver<UdpCommand>,
    ) {
        handle.spawn(async move {
            let runtime = Self {
                source_ips,
                device,
                commands,
                sockets: SocketSet::new([]),
            };
            if let Err(error) = runtime.run().await {
                error!("UDP WireGuard interface failed: {error:#}");
            }
        });
    }

    async fn run(mut self) -> Result<()> {
        let notify = self.device.notify();
        let mut device = self.device;
        let mut iface = Interface::new(
            Config::new(HardwareAddress::Ip),
            &mut device,
            smoltcp::time::Instant::now(),
        );
        configure_interface(&mut iface, &self.source_ips)?;

        let mut next_poll = None;
        let mut handles: HashMap<VirtualPort, UdpHandle> = HashMap::new();
        let mut send_queues: HashMap<VirtualPort, VecDeque<Bytes>> = HashMap::new();

        loop {
            let sleep = super::sleep_until(next_poll, !handles.is_empty());
            tokio::pin!(sleep);

            tokio::select! {
                _ = &mut sleep => {
                    let poll_start = smoltcp::time::Instant::now();
                    let _ = iface.poll(poll_start, &mut device, &mut self.sockets);

                    let mut closed_ports = Vec::new();
                    for (port, handle) in &handles {
                        let socket = self.sockets.get_mut::<udp::Socket>(handle.socket);

                        if socket.can_send() {
                            if let Some(queue) = send_queues.get_mut(port) {
                                while let Some(chunk) = queue.pop_front() {
                                    let target = UdpMetadata::from(handle.target);
                                    if let Err(error) = socket.send_slice(&chunk, target) {
                                        debug!("{} failed to send UDP payload into smoltcp socket: {error:?}", port);
                                        break;
                                    }
                                }
                            }
                        }

                        if socket.can_recv() {
                            match socket.recv() {
                                Ok((payload, _peer)) if !payload.is_empty() => {
                                    if handle.inbound.send(Bytes::copy_from_slice(payload)).is_err() {
                                        closed_ports.push(*port);
                                    }
                                }
                                Ok(_) => {}
                                Err(error) => {
                                    debug!("{} failed to receive UDP payload from smoltcp socket: {error:?}", port);
                                }
                            }
                        }
                    }

                    for port in closed_ports {
                        send_queues.remove(&port);
                        if let Some(handle) = handles.remove(&port) {
                            let _ = self.sockets.remove(handle.socket);
                        }
                    }

                    next_poll = super::poll_deadline(&mut iface, &self.sockets);
                }
                _ = notify.notified() => {
                    next_poll = None;
                }
                command = self.commands.recv() => {
                    let Some(command) = command else {
                        return Ok(());
                    };

                    match command {
                        UdpCommand::Open { port, route, inbound } => {
                            let socket = new_client_socket(route.source_ip, port)?;
                            let handle = self.sockets.add(socket);
                            handles.insert(port, UdpHandle {
                                socket: handle,
                                target: route.remote_addr,
                                inbound,
                            });
                            send_queues.insert(port, VecDeque::new());
                            next_poll = None;
                        }
                        UdpCommand::Send { port, data } => {
                            if let Some(queue) = send_queues.get_mut(&port) {
                                queue.push_back(data);
                                next_poll = None;
                            }
                        }
                        UdpCommand::Close { port } => {
                            send_queues.remove(&port);
                            if let Some(handle) = handles.remove(&port) {
                                let _ = self.sockets.remove(handle.socket);
                            }
                            next_poll = None;
                        }
                    }
                }
            }
        }
    }
}

fn new_client_socket(source_ip: IpAddr, port: VirtualPort) -> Result<udp::Socket<'static>> {
    let rx_meta = vec![udp::PacketMetadata::EMPTY; UDP_META_CAPACITY];
    let tx_meta = vec![udp::PacketMetadata::EMPTY; UDP_META_CAPACITY];
    let rx_data = vec![0u8; MAX_PACKET];
    let tx_data = vec![0u8; MAX_PACKET];
    let mut socket = udp::Socket::new(
        udp::PacketBuffer::new(rx_meta, rx_data),
        udp::PacketBuffer::new(tx_meta, tx_data),
    );
    socket
        .bind((IpAddress::from(source_ip), port.number()))
        .with_context(|| format!("failed to bind UDP session {}", port))?;
    Ok(socket)
}
