use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

use base64::Engine as _;
use ipnet::IpNet;

use crate::endpoint::WireGuardOptions;
use crate::error::{ClientError, Result};

#[derive(Clone, Debug)]
pub struct WireGuardRuntimeConfig {
    pub server_endpoint: SocketAddr,
    pub private_key: [u8; 32],
    pub peer_public_key: [u8; 32],
    pub pre_shared_key: Option<[u8; 32]>,
    pub client_addresses: Vec<IpAddr>,
    pub allowed_ips: Vec<IpNet>,
    pub mtu: usize,
    pub keep_alive: Option<u16>,
}

impl WireGuardRuntimeConfig {
    pub fn from_options(
        server_host: &str,
        server_port: u16,
        opts: &WireGuardOptions,
    ) -> Result<Self> {
        let server_endpoint = resolve_socket_addr(server_host, server_port)?;
        let private_key = decode_key(&opts.private_key, "wireguard.private-key")?;
        let peer_public_key = decode_key(&opts.peer_public_key, "wireguard.peer-public-key")?;
        let pre_shared_key = opts
            .pre_shared_key
            .as_deref()
            .map(|value| decode_key(value, "wireguard.pre-shared-key"))
            .transpose()?;
        let client_ip = parse_client_address(&opts.client_ip)?;
        let allowed_ips = opts
            .allowed_ips
            .iter()
            .map(|value| {
                value.trim().parse::<IpNet>().map_err(|error| {
                    ClientError::Config(format!(
                        "invalid wireguard.allowed-ips entry '{}': {error}",
                        value
                    ))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            server_endpoint,
            private_key,
            peer_public_key,
            pre_shared_key,
            client_addresses: vec![client_ip],
            allowed_ips,
            mtu: usize::try_from(opts.mtu.max(576)).unwrap_or(1400),
            keep_alive: Some(25),
        })
    }

    pub fn select_source_ip(&self, target_ip: IpAddr) -> Option<IpAddr> {
        self.client_addresses
            .iter()
            .copied()
            .find(|candidate| candidate.is_ipv4() == target_ip.is_ipv4())
    }

    pub fn allows_target(&self, target_ip: IpAddr) -> bool {
        self.allowed_ips
            .iter()
            .any(|network| network.contains(&target_ip))
    }
}

fn resolve_socket_addr(host: &str, port: u16) -> Result<SocketAddr> {
    let mut addrs = format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|error| ClientError::Config(format!("resolve wireguard endpoint: {error}")))?;
    addrs.next().ok_or_else(|| {
        ClientError::Config(format!(
            "unable to resolve wireguard endpoint {host}:{port}"
        ))
    })
}

fn decode_key(value: &str, field: &str) -> Result<[u8; 32]> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .map_err(|error| ClientError::Config(format!("decode {field}: {error}")))?;
    decoded
        .try_into()
        .map_err(|_: Vec<u8>| ClientError::Config(format!("{field} expected 32 bytes")))
}

fn parse_client_address(value: &str) -> Result<IpAddr> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ClientError::Config(
            "wireguard client address is empty".into(),
        ));
    }

    if let Ok(network) = trimmed.parse::<IpNet>() {
        let ip = network.addr();
        let expected_bits = if ip.is_ipv4() { 32 } else { 128 };
        if network.prefix_len() != expected_bits {
            return Err(ClientError::Config(format!(
                "wireguard client address '{}' must use /32 for IPv4 or /128 for IPv6",
                trimmed
            )));
        }
        return Ok(ip);
    }

    trimmed.parse::<IpAddr>().map_err(|error| {
        ClientError::Config(format!(
            "invalid wireguard client address '{}': {error}",
            trimmed
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_client_address_accepts_host_ip() {
        let ip = parse_client_address("10.0.0.2").unwrap();
        assert_eq!(ip.to_string(), "10.0.0.2");
    }

    #[test]
    fn parse_client_address_rejects_non_host_prefix() {
        let error = parse_client_address("10.0.0.2/24").unwrap_err();
        assert!(error.to_string().contains("/32"));
    }
}
