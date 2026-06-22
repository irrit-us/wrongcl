use std::collections::{HashSet, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use ipnet::IpNet;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream as TokioTcpStream, UdpSocket as TokioUdpSocket};
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot, watch, Notify};
use tokio_tun::Tun;
use tracing::{debug, error, warn};
use ts_netstack_smoltcp_core as netcore;
use ts_netstack_smoltcp_core::smoltcp::{
    self as stack_smoltcp,
    phy::{DeviceCapabilities, Medium},
    wire::{
        IpProtocol, Ipv4Packet, Ipv6Packet, TcpPacket as SmolTcpPacket, UdpPacket as SmolUdpPacket,
    },
};
use ts_netstack_smoltcp_core::{HasChannel, Netstack, NetstackControl};
use ts_netstack_smoltcp_socket as netsock;
use ts_netstack_smoltcp_socket::CreateSocket;

use super::TunEnableConfig;
use crate::error::{ClientError, Result};

const IDLE_SLEEP: Duration = Duration::from_millis(100);
const MAX_PACKET: usize = 65_536;

pub(super) struct LinuxTunRuntimeHandle {
    running: Arc<AtomicBool>,
    shutdown: Option<watch::Sender<bool>>,
    thread: Option<JoinHandle<()>>,
}

impl LinuxTunRuntimeHandle {
    pub(super) fn start(config: TunEnableConfig) -> Result<Self> {
        let (startup_tx, startup_rx) = std_mpsc::channel();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = Arc::clone(&running);

        let thread = thread::spawn(move || {
            let startup_tx = Some(startup_tx);
            let result = Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| ClientError::Config(format!("build Linux TUN runtime: {error}")))
                .and_then(|runtime| runtime.block_on(run(config, shutdown_rx, startup_tx)));
            if let Err(error) = result {
                error!("Linux TUN runtime exited: {error}");
            }
            thread_running.store(false, Ordering::SeqCst);
        });

        match startup_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(())) => Ok(Self {
                running,
                shutdown: Some(shutdown_tx),
                thread: Some(thread),
            }),
            Ok(Err(message)) => {
                let _ = shutdown_tx.send(true);
                let _ = thread.join();
                Err(ClientError::Config(message))
            }
            Err(_) => {
                let _ = shutdown_tx.send(true);
                let _ = thread.join();
                Err(ClientError::Config(
                    "timed out waiting for Linux TUN runtime startup".into(),
                ))
            }
        }
    }

    pub(super) fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub(super) fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(true);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Drop for LinuxTunRuntimeHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Clone)]
struct DeviceFeed {
    queue: Arc<Mutex<VecDeque<Bytes>>>,
    notify: Arc<Notify>,
}

struct ChannelIpDevice {
    max_transmission_unit: usize,
    outbound: mpsc::UnboundedSender<Vec<u8>>,
    queue: Arc<Mutex<VecDeque<Bytes>>>,
    notify: Arc<Notify>,
}

impl ChannelIpDevice {
    fn new(
        max_transmission_unit: usize,
        outbound: mpsc::UnboundedSender<Vec<u8>>,
    ) -> (Self, DeviceFeed) {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let notify = Arc::new(Notify::new());
        (
            Self {
                max_transmission_unit,
                outbound,
                queue: Arc::clone(&queue),
                notify: Arc::clone(&notify),
            },
            DeviceFeed { queue, notify },
        )
    }

    fn notifier(&self) -> Arc<Notify> {
        Arc::clone(&self.notify)
    }
}

impl DeviceFeed {
    fn push(&self, packet: Bytes) {
        if let Ok(mut guard) = self.queue.lock() {
            guard.push_back(packet);
            self.notify.notify_one();
        }
    }
}

impl stack_smoltcp::phy::Device for ChannelIpDevice {
    type RxToken<'a>
        = RxToken
    where
        Self: 'a;
    type TxToken<'a>
        = TxToken
    where
        Self: 'a;

    fn receive(
        &mut self,
        _timestamp: stack_smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let packet = self.queue.lock().ok()?.pop_front()?;
        Some((
            RxToken {
                buffer: BytesMut::from(packet.as_ref()),
            },
            TxToken {
                outbound: self.outbound.clone(),
            },
        ))
    }

    fn transmit(&mut self, _timestamp: stack_smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        Some(TxToken {
            outbound: self.outbound.clone(),
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = self.max_transmission_unit;
        caps
    }
}

struct RxToken {
    buffer: BytesMut,
}

impl stack_smoltcp::phy::RxToken for RxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        f(&self.buffer)
    }
}

struct TxToken {
    outbound: mpsc::UnboundedSender<Vec<u8>>,
}

impl stack_smoltcp::phy::TxToken for TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = vec![0u8; len];
        let result = f(&mut buffer);
        if self.outbound.send(buffer).is_err() {
            warn!("failed to enqueue outbound TUN packet");
        }
        result
    }
}

enum EnsureRequest {
    Tcp {
        endpoint: SocketAddr,
        response: oneshot::Sender<std::result::Result<(), String>>,
    },
    Udp {
        endpoint: SocketAddr,
        response: oneshot::Sender<std::result::Result<(), String>>,
    },
}

#[derive(Clone)]
struct RouteSet {
    routes: Arc<Vec<IpNet>>,
}

impl RouteSet {
    fn parse(routes: &[String]) -> Result<Self> {
        let mut parsed = Vec::with_capacity(routes.len());
        for route in routes {
            let net = route.parse::<IpNet>().map_err(|error| {
                ClientError::Config(format!("invalid TUN route {route}: {error}"))
            })?;
            parsed.push(net);
        }
        Ok(Self {
            routes: Arc::new(parsed),
        })
    }

    fn allows(&self, ip: IpAddr) -> bool {
        self.routes.is_empty() || self.routes.iter().any(|route| route.contains(&ip))
    }
}

enum PacketEnsure {
    Tcp(SocketAddr),
    Udp(SocketAddr),
}

fn classify_packet(packet: &[u8], routes: &RouteSet) -> Option<PacketEnsure> {
    let version = packet.first().map(|value| value >> 4)?;
    match version {
        4 => {
            let ip = Ipv4Packet::new_checked(packet).ok()?;
            let destination = IpAddr::V4(ip.dst_addr());
            if !routes.allows(destination) {
                return None;
            }
            match ip.next_header() {
                IpProtocol::Tcp => {
                    let tcp = SmolTcpPacket::new_checked(ip.payload()).ok()?;
                    let endpoint = SocketAddr::new(destination, tcp.dst_port());
                    if tcp.syn() && !tcp.ack() {
                        Some(PacketEnsure::Tcp(endpoint))
                    } else {
                        None
                    }
                }
                IpProtocol::Udp => {
                    let udp = SmolUdpPacket::new_checked(ip.payload()).ok()?;
                    Some(PacketEnsure::Udp(SocketAddr::new(
                        destination,
                        udp.dst_port(),
                    )))
                }
                _ => None,
            }
        }
        6 => {
            let ip = Ipv6Packet::new_checked(packet).ok()?;
            let destination = IpAddr::V6(ip.dst_addr());
            if !routes.allows(destination) {
                return None;
            }
            match ip.next_header() {
                IpProtocol::Tcp => {
                    let tcp = SmolTcpPacket::new_checked(ip.payload()).ok()?;
                    let endpoint = SocketAddr::new(destination, tcp.dst_port());
                    if tcp.syn() && !tcp.ack() {
                        Some(PacketEnsure::Tcp(endpoint))
                    } else {
                        None
                    }
                }
                IpProtocol::Udp => {
                    let udp = SmolUdpPacket::new_checked(ip.payload()).ok()?;
                    Some(PacketEnsure::Udp(SocketAddr::new(
                        destination,
                        udp.dst_port(),
                    )))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

async fn run(
    config: TunEnableConfig,
    shutdown_rx: watch::Receiver<bool>,
    startup_tx: Option<std_mpsc::Sender<std::result::Result<(), String>>>,
) -> Result<()> {
    let route_set = RouteSet::parse(&config.routes)?;
    let stack_ip = parse_stack_ip(&config.address_cidr)?;
    let proxy_addr = SocketAddr::new(
        config
            .proxy_host
            .parse()
            .map_err(|error| ClientError::Config(format!("invalid TUN proxy host: {error}")))?,
        config.proxy_port,
    );

    let tun = Arc::new(
        tokio_tun::Tun::builder()
            .name(&config.interface_name)
            .mtu(config.mtu as i32)
            .build()
            .map_err(|error| {
                ClientError::Config(format!(
                    "open Linux TUN interface {}: {error}",
                    config.interface_name
                ))
            })?
            .into_iter()
            .next()
            .ok_or_else(|| {
                ClientError::Config("tokio-tun did not return a Linux TUN handle".into())
            })?,
    );

    let netstack_config = netcore::Config {
        mtu: config.mtu as usize,
        ..Default::default()
    };
    let mut netstack = Netstack::new(netstack_config, stack_smoltcp::time::Instant::ZERO);
    if !netstack.direct_set_ips([stack_ip]) {
        return Err(ClientError::Config(
            "smoltcp could not store the initial Linux TUN address".into(),
        ));
    }
    let channel = netstack.command_channel();

    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel();
    let (mut device, feed) = ChannelIpDevice::new(config.mtu as usize, outbound_tx);
    let device_notify = device.notifier();

    let (ensure_tx, ensure_rx) = mpsc::unbounded_channel();
    let (driver_error_tx, mut driver_error_rx) = mpsc::unbounded_channel::<String>();

    let driver_shutdown = shutdown_rx.clone();
    let driver = tokio::spawn(async move {
        if let Err(error) =
            run_driver_loop(netstack, &mut device, device_notify, driver_shutdown).await
        {
            let _ = driver_error_tx.send(error.to_string());
        }
    });

    let manager_shutdown = shutdown_rx.clone();
    let manager_channel = channel.clone();
    let mut manager = tokio::spawn(async move {
        run_manager_loop(
            manager_channel,
            stack_ip,
            proxy_addr,
            ensure_rx,
            manager_shutdown,
        )
        .await
    });

    let reader_shutdown = shutdown_rx.clone();
    let reader_tun = Arc::clone(&tun);
    let mut reader = tokio::spawn(async move {
        run_reader_loop(reader_tun, route_set, ensure_tx, feed, reader_shutdown).await
    });

    let writer_shutdown = shutdown_rx.clone();
    let writer_tun = Arc::clone(&tun);
    let mut writer = tokio::spawn(async move {
        run_writer_loop(writer_tun, &mut outbound_rx, writer_shutdown).await
    });

    if let Some(startup_tx) = startup_tx {
        let _ = startup_tx.send(Ok(()));
    }

    let result = tokio::select! {
        changed = wait_for_shutdown(shutdown_rx.clone()) => {
            let _ = changed;
            Ok(())
        }
        Some(error) = driver_error_rx.recv() => Err(ClientError::Config(error)),
        result = &mut manager => join_task("Linux TUN manager", result),
        result = &mut reader => join_task("Linux TUN reader", result),
        result = &mut writer => join_task("Linux TUN writer", result),
    };

    driver.abort();
    manager.abort();
    reader.abort();
    writer.abort();
    let _ = driver.await;

    result
}

async fn run_driver_loop(
    mut netstack: Netstack,
    device: &mut ChannelIpDevice,
    notify: Arc<Notify>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let started_at = std::time::Instant::now();
    loop {
        if *shutdown.borrow() {
            return Ok(());
        }

        netstack.process_cmds();
        let now =
            stack_smoltcp::time::Instant::from_micros(started_at.elapsed().as_micros() as i64);
        let _ = netstack.poll_device_io(now, device);
        let delay = netstack.poll_delay(now).unwrap_or(IDLE_SLEEP);

        tokio::select! {
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    return Ok(());
                }
            }
            _ = notify.notified() => {}
            maybe_cmd = netstack.wait_for_cmd() => {
                if let Some(command) = maybe_cmd {
                    netstack.process_one_cmd(command);
                } else {
                    return Ok(());
                }
            }
            _ = tokio::time::sleep(delay.min(IDLE_SLEEP)) => {}
        }
    }
}

async fn run_manager_loop(
    channel: netcore::Channel,
    stack_ip: IpAddr,
    proxy_addr: SocketAddr,
    mut requests: mpsc::UnboundedReceiver<EnsureRequest>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let mut configured_ips = vec![stack_ip];
    let mut known_ips = HashSet::from([stack_ip]);
    let mut tcp_endpoints = HashSet::new();
    let mut udp_endpoints = HashSet::new();

    loop {
        if *shutdown.borrow() {
            return Ok(());
        }

        tokio::select! {
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    return Ok(());
                }
            }
            maybe_request = requests.recv() => {
                let Some(request) = maybe_request else {
                    return Ok(());
                };
                match request {
                    EnsureRequest::Tcp { endpoint, response } => {
                        let result = ensure_ip(&channel, &mut configured_ips, &mut known_ips, endpoint.ip())
                            .await
                            .and_then(|_| ensure_tcp_listener(&channel, &mut tcp_endpoints, endpoint, proxy_addr, shutdown.clone()));
                        let _ = response.send(result.map_err(|error| error.to_string()));
                    }
                    EnsureRequest::Udp { endpoint, response } => {
                        let result = ensure_ip(&channel, &mut configured_ips, &mut known_ips, endpoint.ip())
                            .await
                            .and_then(|_| ensure_udp_socket(&channel, &mut udp_endpoints, endpoint, proxy_addr, shutdown.clone()));
                        let _ = response.send(result.map_err(|error| error.to_string()));
                    }
                }
            }
        }
    }
}

async fn run_reader_loop(
    tun: Arc<Tun>,
    routes: RouteSet,
    ensure_tx: mpsc::UnboundedSender<EnsureRequest>,
    feed: DeviceFeed,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let mut buffer = vec![0u8; MAX_PACKET];
    let mut tcp_endpoints = HashSet::new();
    let mut udp_endpoints = HashSet::new();

    loop {
        if *shutdown.borrow() {
            return Ok(());
        }

        let read = tokio::select! {
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    return Ok(());
                }
                continue;
            }
            result = tun.recv(&mut buffer) => result,
        }?;

        let packet = Bytes::copy_from_slice(&buffer[..read]);
        match classify_packet(&packet, &routes) {
            Some(PacketEnsure::Tcp(endpoint)) => {
                if tcp_endpoints.insert(endpoint) {
                    wait_for_endpoint(&ensure_tx, PacketEnsure::Tcp(endpoint)).await?;
                }
                feed.push(packet);
            }
            Some(PacketEnsure::Udp(endpoint)) => {
                if udp_endpoints.insert(endpoint) {
                    wait_for_endpoint(&ensure_tx, PacketEnsure::Udp(endpoint)).await?;
                }
                feed.push(packet);
            }
            None => feed.push(packet),
        }
    }
}

async fn run_writer_loop(
    tun: Arc<Tun>,
    outbound_rx: &mut mpsc::UnboundedReceiver<Vec<u8>>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    loop {
        if *shutdown.borrow() {
            return Ok(());
        }

        let packet = tokio::select! {
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    return Ok(());
                }
                continue;
            }
            packet = outbound_rx.recv() => packet,
        };

        let Some(packet) = packet else {
            return Ok(());
        };
        tun.send_all(&packet).await.map_err(|error| {
            ClientError::Io(std::io::Error::new(
                error.kind(),
                format!("write Linux TUN packet: {error}"),
            ))
        })?;
    }
}

async fn wait_for_endpoint(
    ensure_tx: &mpsc::UnboundedSender<EnsureRequest>,
    ensure: PacketEnsure,
) -> Result<()> {
    let (response_tx, response_rx) = oneshot::channel();
    let request = match ensure {
        PacketEnsure::Tcp(endpoint) => EnsureRequest::Tcp {
            endpoint,
            response: response_tx,
        },
        PacketEnsure::Udp(endpoint) => EnsureRequest::Udp {
            endpoint,
            response: response_tx,
        },
    };
    ensure_tx
        .send(request)
        .map_err(|_| ClientError::Config("Linux TUN manager channel is closed".into()))?;
    match response_rx.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(message)) => Err(ClientError::Config(message)),
        Err(_) => Err(ClientError::Config(
            "Linux TUN manager dropped the endpoint registration response".into(),
        )),
    }
}

async fn ensure_ip(
    channel: &netcore::Channel,
    configured_ips: &mut Vec<IpAddr>,
    known_ips: &mut HashSet<IpAddr>,
    ip: IpAddr,
) -> Result<()> {
    if !known_ips.insert(ip) {
        return Ok(());
    }
    configured_ips.push(ip);
    channel
        .set_ips(configured_ips.iter().copied())
        .await
        .map_err(|error| ClientError::Config(format!("update Linux TUN netstack IPs: {error}")))
}

fn ensure_tcp_listener(
    channel: &netcore::Channel,
    endpoints: &mut HashSet<SocketAddr>,
    endpoint: SocketAddr,
    proxy_addr: SocketAddr,
    shutdown: watch::Receiver<bool>,
) -> Result<()> {
    if !endpoints.insert(endpoint) {
        return Ok(());
    }

    let channel = channel.clone();
    tokio::spawn(async move {
        match channel.tcp_listen(endpoint).await {
            Ok(listener) => run_tcp_listener(listener, proxy_addr, shutdown).await,
            Err(error) => warn!("failed to create Linux TUN TCP listener for {endpoint}: {error}"),
        }
    });

    Ok(())
}

fn ensure_udp_socket(
    channel: &netcore::Channel,
    endpoints: &mut HashSet<SocketAddr>,
    endpoint: SocketAddr,
    proxy_addr: SocketAddr,
    shutdown: watch::Receiver<bool>,
) -> Result<()> {
    if !endpoints.insert(endpoint) {
        return Ok(());
    }

    let channel = channel.clone();
    tokio::spawn(async move {
        match channel.udp_bind(endpoint).await {
            Ok(socket) => run_udp_bridge(socket, endpoint, proxy_addr, shutdown).await,
            Err(error) => warn!("failed to create Linux TUN UDP socket for {endpoint}: {error}"),
        }
    });

    Ok(())
}

async fn run_tcp_listener(
    listener: netsock::TcpListener,
    proxy_addr: SocketAddr,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        if *shutdown.borrow() {
            return;
        }
        let accepted = tokio::select! {
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    return;
                }
                continue;
            }
            accepted = listener.accept() => accepted,
        };

        match accepted {
            Ok(stream) => {
                let target = listener.local_addr();
                let child_shutdown = shutdown.clone();
                tokio::spawn(async move {
                    if let Err(error) =
                        run_tcp_bridge(stream, target, proxy_addr, child_shutdown).await
                    {
                        debug!("Linux TUN TCP bridge for {target} ended: {error}");
                    }
                });
            }
            Err(error) => {
                warn!(
                    "Linux TUN TCP listener on {} stopped accepting: {error}",
                    listener.local_addr()
                );
                return;
            }
        }
    }
}

async fn run_tcp_bridge(
    inbound: netsock::TcpStream,
    target: SocketAddr,
    proxy_addr: SocketAddr,
    mut shutdown: watch::Receiver<bool>,
) -> std::io::Result<()> {
    let mut outbound = TokioTcpStream::connect(proxy_addr).await?;
    socks5_connect(&mut outbound, target).await?;
    let inbound = Arc::new(inbound);
    let (mut reader, mut writer) = outbound.into_split();

    let mut upstream_shutdown = shutdown.clone();
    let inbound_reader = Arc::clone(&inbound);
    let upstream = async move {
        let mut buffer = vec![0u8; 16 * 1024];
        loop {
            let read = tokio::select! {
                result = shutdown.changed() => {
                    if result.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                    continue;
                }
                read = inbound_reader.recv(&mut buffer) => read.map_err(netstack_io)?,
            };
            if read == 0 {
                break;
            }
            writer.write_all(&buffer[..read]).await?;
        }
        let _ = writer.shutdown().await;
        Ok::<(), std::io::Error>(())
    };

    let inbound_writer = Arc::clone(&inbound);
    let downstream = async move {
        let mut buffer = vec![0u8; 16 * 1024];
        loop {
            let read = tokio::select! {
                result = upstream_shutdown.changed() => {
                    if result.is_err() || *upstream_shutdown.borrow() {
                        return Ok(());
                    }
                    continue;
                }
                read = reader.read(&mut buffer) => read,
            }?;
            if read == 0 {
                break;
            }
            send_all_netstack(&inbound_writer, &buffer[..read]).await?;
        }
        Ok::<(), std::io::Error>(())
    };

    let _ = tokio::join!(upstream, downstream);
    Ok(())
}

async fn run_udp_bridge(
    socket: netsock::UdpSocket,
    target: SocketAddr,
    proxy_addr: SocketAddr,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut control = match TokioTcpStream::connect(proxy_addr).await {
        Ok(stream) => stream,
        Err(error) => {
            warn!("Linux TUN UDP bridge could not connect to SOCKS proxy {proxy_addr}: {error}");
            return;
        }
    };
    let relay = match socks5_udp_associate(&mut control).await {
        Ok(relay) => relay,
        Err(error) => {
            warn!("Linux TUN UDP bridge could not open SOCKS UDP associate: {error}");
            return;
        }
    };

    let bind_addr = match relay {
        SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    };
    let relay_socket = match TokioUdpSocket::bind(bind_addr).await {
        Ok(socket) => Arc::new(socket),
        Err(error) => {
            warn!("Linux TUN UDP bridge could not bind relay socket: {error}");
            return;
        }
    };
    if let Err(error) = relay_socket.connect(relay).await {
        warn!("Linux TUN UDP bridge could not connect relay socket to {relay}: {error}");
        return;
    }

    let last_client = Arc::new(tokio::sync::Mutex::new(None::<SocketAddr>));
    let socket = Arc::new(socket);

    let mut relay_shutdown = shutdown.clone();
    let uplink_socket = Arc::clone(&socket);
    let uplink_relay = Arc::clone(&relay_socket);
    let uplink_client = Arc::clone(&last_client);
    let uplink = async move {
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let (source, read) = tokio::select! {
                result = shutdown.changed() => {
                    if result.is_err() || *shutdown.borrow() {
                        return Ok::<(), std::io::Error>(());
                    }
                    continue;
                }
                recv = uplink_socket.recv_from(&mut buffer) => recv.map_err(netstack_io)?,
            };
            {
                let mut guard = uplink_client.lock().await;
                *guard = Some(source);
            }
            let packet = encode_socks5_udp_packet(target, &buffer[..read]);
            uplink_relay.send(&packet).await?;
        }
    };

    let downlink_socket = Arc::clone(&socket);
    let downlink_relay = Arc::clone(&relay_socket);
    let downlink_client = Arc::clone(&last_client);
    let downlink = async move {
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let read = tokio::select! {
                result = relay_shutdown.changed() => {
                    if result.is_err() || *relay_shutdown.borrow() {
                        return Ok::<(), std::io::Error>(());
                    }
                    continue;
                }
                recv = downlink_relay.recv(&mut buffer) => recv,
            }?;
            let Ok((packet_target, payload)) = parse_socks5_udp_packet(&buffer[..read]) else {
                continue;
            };
            if packet_target != target {
                continue;
            }
            let client = {
                let guard = downlink_client.lock().await;
                *guard
            };
            let Some(client) = client else {
                continue;
            };
            downlink_socket
                .send_to(client, &payload)
                .await
                .map_err(netstack_io)?;
        }
    };

    let _control = control;
    let _ = tokio::join!(uplink, downlink);
}

async fn send_all_netstack(stream: &netsock::TcpStream, buffer: &[u8]) -> std::io::Result<()> {
    let mut written = 0;
    while written < buffer.len() {
        let sent = stream.send(&buffer[written..]).await.map_err(netstack_io)?;
        if sent == 0 {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        written += sent;
    }
    Ok(())
}

fn join_task<T>(
    label: &str,
    result: std::result::Result<Result<T>, tokio::task::JoinError>,
) -> Result<()> {
    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(error) => Err(ClientError::Config(format!(
            "{label} task panicked: {error}"
        ))),
    }
}

fn parse_stack_ip(address_cidr: &str) -> Result<IpAddr> {
    let (ip, _) = address_cidr
        .split_once('/')
        .ok_or_else(|| ClientError::Config("invalid TUN address_cidr".into()))?;
    ip.parse()
        .map_err(|error| ClientError::Config(format!("invalid TUN address_cidr IP: {error}")))
}

async fn wait_for_shutdown(
    mut shutdown: watch::Receiver<bool>,
) -> std::result::Result<(), watch::error::RecvError> {
    while !*shutdown.borrow() {
        shutdown.changed().await?;
    }
    Ok(())
}

fn netstack_io(error: netcore::Error) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

async fn socks5_connect(stream: &mut TokioTcpStream, target: SocketAddr) -> std::io::Result<()> {
    socks5_negotiate(stream).await?;
    let mut request = vec![0x05, 0x01, 0x00];
    write_socks5_addr(&mut request, target);
    stream.write_all(&request).await?;

    let mut reply = [0u8; 4];
    stream.read_exact(&mut reply).await?;
    if reply[0] != 0x05 {
        return Err(std::io::Error::other(
            "SOCKS5 CONNECT reply used a bad version",
        ));
    }
    if reply[1] != 0x00 {
        return Err(std::io::Error::other(format!(
            "SOCKS5 CONNECT failed with reply {:#04x}",
            reply[1]
        )));
    }
    let _ = read_socks5_bound_addr(stream, reply[3]).await?;
    Ok(())
}

async fn socks5_udp_associate(stream: &mut TokioTcpStream) -> std::io::Result<SocketAddr> {
    socks5_negotiate(stream).await?;
    let request = [0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    stream.write_all(&request).await?;

    let mut reply = [0u8; 4];
    stream.read_exact(&mut reply).await?;
    if reply[0] != 0x05 {
        return Err(std::io::Error::other(
            "SOCKS5 UDP ASSOCIATE reply used a bad version",
        ));
    }
    if reply[1] != 0x00 {
        return Err(std::io::Error::other(format!(
            "SOCKS5 UDP ASSOCIATE failed with reply {:#04x}",
            reply[1]
        )));
    }
    let relay = read_socks5_bound_addr(stream, reply[3]).await?;
    Ok(match relay {
        SocketAddr::V4(addr) if addr.ip().is_unspecified() => {
            SocketAddr::new(stream.peer_addr()?.ip(), addr.port())
        }
        SocketAddr::V6(addr) if addr.ip().is_unspecified() => {
            SocketAddr::new(stream.peer_addr()?.ip(), addr.port())
        }
        other => other,
    })
}

async fn socks5_negotiate(stream: &mut TokioTcpStream) -> std::io::Result<()> {
    stream.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut method = [0u8; 2];
    stream.read_exact(&mut method).await?;
    if method != [0x05, 0x00] {
        return Err(std::io::Error::other(
            "SOCKS5 proxy rejected no-auth negotiation",
        ));
    }
    Ok(())
}

fn write_socks5_addr(out: &mut Vec<u8>, target: SocketAddr) {
    match target {
        SocketAddr::V4(addr) => {
            out.push(0x01);
            out.extend_from_slice(&addr.ip().octets());
            out.extend_from_slice(&addr.port().to_be_bytes());
        }
        SocketAddr::V6(addr) => {
            out.push(0x04);
            out.extend_from_slice(&addr.ip().octets());
            out.extend_from_slice(&addr.port().to_be_bytes());
        }
    }
}

async fn read_socks5_bound_addr(
    stream: &mut TokioTcpStream,
    atyp: u8,
) -> std::io::Result<SocketAddr> {
    match atyp {
        0x01 => {
            let mut payload = [0u8; 6];
            stream.read_exact(&mut payload).await?;
            Ok(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(
                    payload[0], payload[1], payload[2], payload[3],
                )),
                u16::from_be_bytes([payload[4], payload[5]]),
            ))
        }
        0x04 => {
            let mut payload = [0u8; 18];
            stream.read_exact(&mut payload).await?;
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&payload[..16]);
            Ok(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::from(octets)),
                u16::from_be_bytes([payload[16], payload[17]]),
            ))
        }
        0x03 => {
            let mut length = [0u8; 1];
            stream.read_exact(&mut length).await?;
            let mut domain = vec![0u8; length[0] as usize + 2];
            stream.read_exact(&mut domain).await?;
            Err(std::io::Error::other(
                "SOCKS5 proxy returned a domain relay address, which Linux TUN does not support",
            ))
        }
        other => Err(std::io::Error::other(format!(
            "SOCKS5 reply used unsupported address type {other:#04x}"
        ))),
    }
}

fn encode_socks5_udp_packet(target: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let mut packet = vec![0x00, 0x00, 0x00];
    write_socks5_addr(&mut packet, target);
    packet.extend_from_slice(payload);
    packet
}

fn parse_socks5_udp_packet(packet: &[u8]) -> std::io::Result<(SocketAddr, Vec<u8>)> {
    if packet.len() < 4 {
        return Err(std::io::Error::other("SOCKS5 UDP packet is too short"));
    }
    if packet[0] != 0 || packet[1] != 0 || packet[2] != 0 {
        return Err(std::io::Error::other(
            "SOCKS5 UDP reserved/fragment bytes are unsupported",
        ));
    }
    match packet[3] {
        0x01 => {
            if packet.len() < 10 {
                return Err(std::io::Error::other("SOCKS5 UDP IPv4 packet is too short"));
            }
            let target = SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(packet[4], packet[5], packet[6], packet[7])),
                u16::from_be_bytes([packet[8], packet[9]]),
            );
            Ok((target, packet[10..].to_vec()))
        }
        0x04 => {
            if packet.len() < 22 {
                return Err(std::io::Error::other("SOCKS5 UDP IPv6 packet is too short"));
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&packet[4..20]);
            let target = SocketAddr::new(
                IpAddr::V6(Ipv6Addr::from(octets)),
                u16::from_be_bytes([packet[20], packet[21]]),
            );
            Ok((target, packet[22..].to_vec()))
        }
        other => Err(std::io::Error::other(format!(
            "SOCKS5 UDP address type {other:#04x} is unsupported"
        ))),
    }
}
