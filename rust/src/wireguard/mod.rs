mod bridge;
mod cache;

use crate::endpoint::WireGuardOptions;
use crate::error::{ClientError, Result};
use crate::protocol::Target;

pub fn connect_wireguard(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
    target: &Target,
) -> Result<Box<dyn crate::client::Tunnel>> {
    let runtime = cache::acquire_runtime(server_host, server_port, opts)?;
    let route = runtime.resolve_target(target).map_err(map_runtime_error)?;
    let session = runtime.open_tcp(route).map_err(map_runtime_error)?;
    bridge::runtime_tunnel(session, runtime)
}

pub fn connect_wireguard_udp(
    server_host: &str,
    server_port: u16,
    opts: &WireGuardOptions,
    target: &Target,
) -> Result<Box<dyn crate::client::UdpSession>> {
    let runtime = cache::acquire_runtime(server_host, server_port, opts)?;
    let route = runtime.resolve_target(target).map_err(map_runtime_error)?;
    let session = runtime.open_udp(route).map_err(map_runtime_error)?;
    Ok(bridge::runtime_udp_session(
        session,
        runtime,
        target.clone(),
    ))
}

fn map_runtime_error(error: anyhow::Error) -> ClientError {
    if let Some(io_error) = error.downcast_ref::<std::io::Error>() {
        return ClientError::Io(std::io::Error::new(io_error.kind(), io_error.to_string()));
    }
    ClientError::Config(error.to_string())
}
