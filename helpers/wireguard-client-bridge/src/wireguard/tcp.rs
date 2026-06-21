use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;

use anyhow::{Context, Result};
use bytes::Bytes;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp;
use smoltcp::wire::{HardwareAddress, IpAddress};
use tokio::sync::mpsc;
use tracing::{debug, error};

use super::device::ChannelIpDevice;
use super::port_pool::VirtualPort;
use super::{configure_interface, TargetRoute};

const MAX_PACKET: usize = 65_536;

pub enum TcpCommand {
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

pub struct TcpInterface {
    source_ips: Vec<IpAddr>,
    device: ChannelIpDevice,
    commands: mpsc::UnboundedReceiver<TcpCommand>,
    sockets: SocketSet<'static>,
}

impl TcpInterface {
    pub fn spawn(
        source_ips: Vec<IpAddr>,
        device: ChannelIpDevice,
        commands: mpsc::UnboundedReceiver<TcpCommand>,
    ) {
        tokio::spawn(async move {
            let runtime = Self {
                source_ips,
                device,
                commands,
                sockets: SocketSet::new([]),
            };
            if let Err(error) = runtime.run().await {
                error!("TCP WireGuard interface failed: {error:#}");
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
        let mut handles: HashMap<VirtualPort, SocketHandle> = HashMap::new();
        let mut inbound: HashMap<VirtualPort, mpsc::UnboundedSender<Bytes>> = HashMap::new();
        let mut send_queues: HashMap<VirtualPort, VecDeque<Bytes>> = HashMap::new();
        let mut close_requested: HashSet<VirtualPort> = HashSet::new();

        loop {
            let sleep = super::sleep_until(next_poll, !handles.is_empty());
            tokio::pin!(sleep);

            tokio::select! {
                _ = &mut sleep => {
                    let poll_start = smoltcp::time::Instant::now();
                    let _ = iface.poll(poll_start, &mut device, &mut self.sockets);

                    let mut closed_ports = Vec::new();
                    for (port, handle) in &handles {
                        let socket = self.sockets.get_mut::<tcp::Socket>(*handle);

                        if socket.state() == tcp::State::Closed {
                            closed_ports.push(*port);
                            continue;
                        }

                        if socket.can_send() {
                            if let Some(queue) = send_queues.get_mut(port) {
                                if let Some(chunk) = queue.pop_front() {
                                    match socket.send_slice(&chunk) {
                                        Ok(sent) if sent < chunk.len() => {
                                            queue.push_front(Bytes::copy_from_slice(&chunk[sent..]));
                                        }
                                        Ok(_) => {}
                                        Err(error) => {
                                            debug!("{} failed to send TCP chunk into smoltcp socket: {error:?}", port);
                                        }
                                    }
                                } else if close_requested.contains(port) {
                                    socket.close();
                                }
                            }
                        }

                        if socket.can_recv() {
                            match socket.recv(|buffer| (buffer.len(), Bytes::copy_from_slice(buffer))) {
                                Ok(data) if !data.is_empty() => {
                                    let dropped = inbound
                                        .get(port)
                                        .map(|tx| tx.send(data).is_err())
                                        .unwrap_or(true);
                                    if dropped {
                                        closed_ports.push(*port);
                                    }
                                }
                                Ok(_) => {}
                                Err(error) => {
                                    debug!("{} failed to receive TCP chunk from smoltcp socket: {error:?}", port);
                                }
                            }
                        }
                    }

                    for port in closed_ports {
                        close_requested.remove(&port);
                        send_queues.remove(&port);
                        inbound.remove(&port);
                        if let Some(handle) = handles.remove(&port) {
                            let _ = self.sockets.remove(handle);
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
                        TcpCommand::Open { port, route, inbound: tx } => {
                            let handle = self.sockets.add(new_client_socket());
                            let context = iface.context();
                            let socket = self.sockets.get_mut::<tcp::Socket>(handle);
                            socket
                                .connect(
                                    context,
                                    (IpAddress::from(route.remote_addr.ip()), route.remote_addr.port()),
                                    (IpAddress::from(route.source_ip), port.number()),
                                )
                                .with_context(|| format!("failed to open TCP session {port}"))?;

                            handles.insert(port, handle);
                            inbound.insert(port, tx);
                            send_queues.insert(port, VecDeque::new());
                            close_requested.remove(&port);
                            next_poll = None;
                        }
                        TcpCommand::Send { port, data } => {
                            if let Some(queue) = send_queues.get_mut(&port) {
                                queue.push_back(data);
                                next_poll = None;
                            }
                        }
                        TcpCommand::Close { port } => {
                            close_requested.insert(port);
                            next_poll = None;
                        }
                    }
                }
            }
        }
    }
}

fn new_client_socket() -> tcp::Socket<'static> {
    let rx_data = vec![0u8; MAX_PACKET];
    let tx_data = vec![0u8; MAX_PACKET];
    tcp::Socket::new(
        tcp::SocketBuffer::new(rx_data),
        tcp::SocketBuffer::new(tx_data),
    )
}
