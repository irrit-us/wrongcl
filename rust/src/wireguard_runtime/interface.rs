use std::net::IpAddr;

use anyhow::{anyhow, bail, Result};
use smoltcp::iface::{Interface, SocketSet};
use smoltcp::wire::{IpAddress, IpCidr, Ipv4Address, Ipv6Address};
use tokio::time::{Duration, Instant};

pub fn configure_interface(interface: &mut Interface, source_ips: &[IpAddr]) -> Result<()> {
    let mut ip_slots_exhausted = false;
    interface.update_ip_addrs(|ip_addrs| {
        ip_addrs.clear();
        for ip in source_ips {
            let cidr = IpCidr::new(IpAddress::from(*ip), addr_prefix_len(*ip));
            if ip_addrs.push(cidr).is_err() {
                ip_slots_exhausted = true;
                break;
            }
        }
    });
    if ip_slots_exhausted {
        bail!("smoltcp interface ran out of source address slots");
    }

    let mut configured_ipv4 = false;
    let mut configured_ipv6 = false;
    for ip in source_ips {
        match ip {
            IpAddr::V4(addr) if !configured_ipv4 => {
                interface
                    .routes_mut()
                    .add_default_ipv4_route(Ipv4Address::from(addr.octets()))
                    .map_err(|_| anyhow!("smoltcp IPv4 route table is full"))?;
                configured_ipv4 = true;
            }
            IpAddr::V6(addr) if !configured_ipv6 => {
                interface
                    .routes_mut()
                    .add_default_ipv6_route(Ipv6Address::from(addr.octets()))
                    .map_err(|_| anyhow!("smoltcp IPv6 route table is full"))?;
                configured_ipv6 = true;
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn poll_deadline(iface: &mut Interface, sockets: &SocketSet<'_>) -> Option<Instant> {
    match iface.poll_delay(smoltcp::time::Instant::now(), sockets) {
        Some(smoltcp::time::Duration::ZERO) => None,
        Some(delay) => Some(Instant::now() + Duration::from_millis(delay.total_millis())),
        None => None,
    }
}

pub fn sleep_until(next_poll: Option<Instant>, has_sessions: bool) -> tokio::time::Sleep {
    match (next_poll, has_sessions) {
        (Some(deadline), _) => tokio::time::sleep_until(deadline),
        (None, true) => tokio::time::sleep(Duration::ZERO),
        (None, false) => tokio::time::sleep(Duration::from_secs(24 * 60 * 60)),
    }
}

fn addr_prefix_len(addr: IpAddr) -> u8 {
    if addr.is_ipv4() {
        32
    } else {
        128
    }
}
