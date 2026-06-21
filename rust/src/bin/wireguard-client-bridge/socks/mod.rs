mod protocol;
mod tcp;
mod udp;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tracing::debug;

use crate::wireguard::WireGuardRuntime;

pub(crate) use protocol::TargetAddress;

pub async fn run_socks_server(
    listen: SocketAddr,
    runtime: Arc<WireGuardRuntime>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let listener = TcpListener::bind(listen)
        .await
        .with_context(|| format!("failed to bind SOCKS listener on {listen}"))?;

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    return Ok(());
                }
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted.context("failed to accept SOCKS connection")?;
                let runtime = Arc::clone(&runtime);
                tokio::spawn(async move {
                    if let Err(error) = handle_client(stream, runtime).await {
                        debug!("SOCKS connection {peer} closed: {error:#}");
                    }
                });
            }
        }
    }
}

async fn handle_client(mut stream: TcpStream, runtime: Arc<WireGuardRuntime>) -> Result<()> {
    protocol::perform_handshake(&mut stream).await?;
    let request = protocol::read_request(&mut stream).await?;
    match request {
        protocol::SocksRequest::Connect(target) => {
            tcp::handle_connect(stream, runtime, target).await
        }
        protocol::SocksRequest::UdpAssociate => udp::handle_udp_associate(stream, runtime).await,
    }
}
