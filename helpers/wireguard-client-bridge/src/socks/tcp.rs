use std::sync::Arc;

use anyhow::{Context, Result};
use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::protocol::{self, TargetAddress};
use crate::wireguard::WireGuardRuntime;

pub async fn handle_connect(
    mut stream: TcpStream,
    runtime: Arc<WireGuardRuntime>,
    target: TargetAddress,
) -> Result<()> {
    let route = match runtime.resolve_target(&target).await {
        Ok(route) => route,
        Err(error) => {
            protocol::write_failure(&mut stream, 0x04).await?;
            return Err(error);
        }
    };

    let mut session = match runtime.open_tcp(route) {
        Ok(session) => session,
        Err(error) => {
            protocol::write_failure(&mut stream, 0x05).await?;
            return Err(error);
        }
    };

    protocol::write_connect_success(&mut stream).await?;

    let mut buffer = [0u8; 64 * 1024];
    let mut local_closed = false;

    loop {
        if local_closed {
            match session.recv().await {
                Some(bytes) => {
                    stream
                        .write_all(&bytes)
                        .await
                        .context("failed to write TCP payload to local client")?;
                }
                None => return Ok(()),
            }
            continue;
        }

        tokio::select! {
            received = session.recv() => {
                match received {
                    Some(bytes) => {
                        stream
                            .write_all(&bytes)
                            .await
                            .context("failed to write TCP payload to local client")?;
                    }
                    None => return Ok(()),
                }
            }
            read = stream.read(&mut buffer) => {
                match read.context("failed to read TCP payload from local client")? {
                    0 => {
                        local_closed = true;
                        session.shutdown();
                    }
                    size => session
                        .send(Bytes::copy_from_slice(&buffer[..size]))
                        .context("failed to forward TCP payload into WireGuard session")?,
                }
            }
        }
    }
}
