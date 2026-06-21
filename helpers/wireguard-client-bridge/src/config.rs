use std::fs;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::Path;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use ipnet::IpNet;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub listen: SocketAddr,
    pub server_endpoint: SocketAddr,
    pub private_key: [u8; 32],
    pub peer_public_key: [u8; 32],
    pub pre_shared_key: Option<[u8; 32]>,
    pub client_addresses: Vec<IpAddr>,
    pub allowed_ips: Vec<IpNet>,
    pub mtu: usize,
    pub keep_alive: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    listen: String,
    server_endpoint: String,
    private_key: String,
    peer_public_key: String,
    #[serde(default)]
    pre_shared_key: Option<String>,
    client_addresses: Vec<String>,
    allowed_ips: Vec<String>,
    #[serde(default)]
    mtu: Option<usize>,
    #[serde(default)]
    keep_alive: Option<u16>,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        let raw: RawConfig = serde_json::from_slice(&raw)
            .with_context(|| format!("invalid JSON in {}", path.display()))?;
        Self::from_raw(raw)
    }

    fn from_raw(raw: RawConfig) -> Result<Self> {
        if raw.client_addresses.is_empty() {
            bail!("missing client_addresses");
        }
        if raw.allowed_ips.is_empty() {
            bail!("missing allowed_ips");
        }

        let listen = resolve_socket_addr(&raw.listen, "listen")?;
        let server_endpoint = resolve_socket_addr(&raw.server_endpoint, "server_endpoint")?;
        let private_key = decode_key(&raw.private_key, "private_key")?;
        let peer_public_key = decode_key(&raw.peer_public_key, "peer_public_key")?;
        let pre_shared_key = raw
            .pre_shared_key
            .as_deref()
            .map(|value| decode_key(value, "pre_shared_key"))
            .transpose()?;

        let client_addresses = raw
            .client_addresses
            .iter()
            .map(|value| parse_client_address(value))
            .collect::<Result<Vec<_>>>()?;

        let allowed_ips = raw
            .allowed_ips
            .iter()
            .map(|value| {
                value
                    .trim()
                    .parse::<IpNet>()
                    .with_context(|| format!("invalid allowed_ips entry {value:?}"))
            })
            .collect::<Result<Vec<_>>>()?;

        let mtu = raw.mtu.unwrap_or(1400).max(576);
        let keep_alive = Some(raw.keep_alive.unwrap_or(25).max(1));

        Ok(Self {
            listen,
            server_endpoint,
            private_key,
            peer_public_key,
            pre_shared_key,
            client_addresses,
            allowed_ips,
            mtu,
            keep_alive,
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

fn resolve_socket_addr(value: &str, field: &str) -> Result<SocketAddr> {
    let mut addrs = value
        .to_socket_addrs()
        .with_context(|| format!("invalid {field} {value:?}"))?;
    addrs
        .next()
        .with_context(|| format!("unable to resolve {field} {value:?}"))
}

fn decode_key(value: &str, field: &str) -> Result<[u8; 32]> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .with_context(|| format!("decode {field}"))?;
    decoded
        .try_into()
        .map_err(|_: Vec<u8>| anyhow::anyhow!("{field} expected 32 bytes"))
}

fn parse_client_address(value: &str) -> Result<IpAddr> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("empty client address");
    }

    if let Ok(network) = trimmed.parse::<IpNet>() {
        let ip = network.addr();
        let expected_bits = if ip.is_ipv4() { 32 } else { 128 };
        if network.prefix_len() != expected_bits {
            bail!("client address {trimmed:?} must use /32 for IPv4 or /128 for IPv6");
        }
        return Ok(ip);
    }

    trimmed
        .parse::<IpAddr>()
        .with_context(|| format!("invalid client address {trimmed:?}"))
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
