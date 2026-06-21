use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::Notify;

use super::protocol::{self, TargetAddress};
use crate::wireguard::{UdpSessionReader, UdpSessionWriter, WireGuardRuntime};

struct SessionEntry {
    writer: UdpSessionWriter,
    _task: tokio::task::JoinHandle<()>,
}

pub async fn handle_udp_associate(
    mut stream: TcpStream,
    runtime: Arc<WireGuardRuntime>,
) -> Result<()> {
    let bind_ip = relay_bind_ip(stream.local_addr()?.ip());
    let relay = Arc::new(UdpSocket::bind(SocketAddr::new(bind_ip, 0)).await?);
    let relay_addr = relay.local_addr()?;

    protocol::write_udp_associate_success(&mut stream, relay_addr).await?;

    let closed = Arc::new(Notify::new());
    let mut control_reader = stream;
    let closed_task = Arc::clone(&closed);
    tokio::spawn(async move {
        let mut buf = [0u8; 1];
        loop {
            match control_reader.read(&mut buf).await {
                Ok(0) | Err(_) => {
                    closed_task.notify_waiters();
                    break;
                }
                Ok(_) => {}
            }
        }
    });

    let mut sessions: HashMap<String, SessionEntry> = HashMap::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        tokio::select! {
            _ = closed.notified() => return Ok(()),
            received = relay.recv_from(&mut buffer) => {
                let (size, peer) = received.context("failed to receive UDP datagram from local client")?;
                let (target, payload) = match protocol::parse_udp_packet(&buffer[..size]) {
                    Ok(packet) => packet,
                    Err(_) => continue,
                };

                let key = target.key();
                if !sessions.contains_key(&key) {
                    let route = match runtime.resolve_target(&target).await {
                        Ok(route) => route,
                        Err(_) => continue,
                    };
                    let session = match runtime.open_udp(route) {
                        Ok(session) => session,
                        Err(_) => continue,
                    };
                    let (writer, reader) = session.split();
                    let relay_socket = Arc::clone(&relay);
                    let response_target = target.clone();
                    let task = tokio::spawn(async move {
                        let _ = forward_udp_responses(relay_socket, peer, response_target, reader).await;
                    });
                    sessions.insert(key.clone(), SessionEntry { writer, _task: task });
                }

                if let Some(entry) = sessions.get(&key) {
                    let _ = entry.writer.send(payload);
                }
            }
        }
    }
}

async fn forward_udp_responses(
    relay: Arc<UdpSocket>,
    client_peer: SocketAddr,
    target: TargetAddress,
    mut reader: UdpSessionReader,
) -> Result<()> {
    while let Some(payload) = reader.recv().await {
        let packet = protocol::encode_udp_packet(&target, &payload)?;
        relay.send_to(&packet, client_peer).await?;
    }
    Ok(())
}

fn relay_bind_ip(ip: IpAddr) -> IpAddr {
    if ip.is_unspecified() {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
        ip
    }
}
