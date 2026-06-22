use std::net::SocketAddr;

/// Best-effort lookup of the local process that initiated a TCP connection to
/// our proxy listener. Returns `(pid, comm)` when both can be resolved.
///
/// Implemented on Linux via `/proc/net/tcp{,6}` + `/proc/<pid>/fd/*` + `/proc/<pid>/comm`.
/// On macOS and Windows the lookup is not implemented yet and returns `None`.
pub fn lookup(peer_addr: SocketAddr) -> Option<(u32, String)> {
    backend::lookup(peer_addr)
}

#[cfg(target_os = "linux")]
mod backend {
    use std::fs;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    pub(super) fn lookup(peer_addr: SocketAddr) -> Option<(u32, String)> {
        let inode = match peer_addr {
            SocketAddr::V4(_) => find_inode("/proc/net/tcp", peer_addr)
                .or_else(|| find_inode("/proc/net/tcp6", peer_addr)),
            SocketAddr::V6(_) => find_inode("/proc/net/tcp6", peer_addr)
                .or_else(|| find_inode("/proc/net/tcp", peer_addr)),
        }?;
        let pid = find_pid_with_socket_inode(inode)?;
        let comm = fs::read_to_string(format!("/proc/{pid}/comm"))
            .ok()
            .map(|s| s.trim_end_matches('\n').to_string())?;
        Some((pid, comm))
    }

    fn find_inode(path: &str, peer_addr: SocketAddr) -> Option<u64> {
        let text = fs::read_to_string(path).ok()?;
        for line in text.lines().skip(1) {
            // Columns: sl local_address rem_address st ... inode
            let mut fields = line.split_ascii_whitespace();
            let _sl = fields.next()?;
            let local = fields.next()?;
            let _rem = fields.next()?;
            let _state = fields.next()?;
            // skip tx_queue rx_queue, tr+tm->when, retrnsmt, uid, timeout
            let _ = fields.next()?;
            let _ = fields.next()?;
            let _ = fields.next()?;
            let _ = fields.next()?;
            let _ = fields.next()?;
            let _ = fields.next()?;
            let inode_str = fields.next()?;
            let inode: u64 = inode_str.parse().ok()?;
            if inode == 0 {
                continue;
            }
            let (addr, port) = parse_address(local)?;
            if addr == peer_addr.ip() && port == peer_addr.port() {
                return Some(inode);
            }
        }
        None
    }

    fn parse_address(text: &str) -> Option<(IpAddr, u16)> {
        let (ip_hex, port_hex) = text.split_once(':')?;
        let port = u16::from_str_radix(port_hex, 16).ok()?;
        match ip_hex.len() {
            8 => {
                let raw = u32::from_str_radix(ip_hex, 16).ok()?;
                // /proc/net/tcp stores each 32-bit chunk in host (little-endian on
                // little-endian platforms) order — reverse to get network-order octets.
                let bytes = raw.to_be_bytes();
                let octets = [bytes[3], bytes[2], bytes[1], bytes[0]];
                Some((IpAddr::V4(Ipv4Addr::from(octets)), port))
            }
            32 => {
                let mut octets = [0u8; 16];
                for chunk in 0..4 {
                    let start = chunk * 8;
                    let chunk_hex = ip_hex.get(start..start + 8)?;
                    let raw = u32::from_str_radix(chunk_hex, 16).ok()?;
                    let bytes = raw.to_be_bytes();
                    octets[chunk * 4] = bytes[3];
                    octets[chunk * 4 + 1] = bytes[2];
                    octets[chunk * 4 + 2] = bytes[1];
                    octets[chunk * 4 + 3] = bytes[0];
                }
                Some((IpAddr::V6(Ipv6Addr::from(octets)), port))
            }
            _ => None,
        }
    }

    fn find_pid_with_socket_inode(inode: u64) -> Option<u32> {
        let needle = format!("socket:[{inode}]");
        let dir = fs::read_dir("/proc").ok()?;
        for entry in dir.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let pid: u32 = match name.parse() {
                Ok(pid) => pid,
                Err(_) => continue,
            };
            let fd_dir = match fs::read_dir(format!("/proc/{pid}/fd")) {
                Ok(d) => d,
                Err(_) => continue,
            };
            for fd in fd_dir.flatten() {
                let link = match fs::read_link(fd.path()) {
                    Ok(link) => link,
                    Err(_) => continue,
                };
                if link.to_string_lossy() == needle {
                    return Some(pid);
                }
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::net::Ipv4Addr;

        #[test]
        fn parses_loopback_ipv4_little_endian() {
            let (ip, port) = parse_address("0100007F:1F90").unwrap();
            assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
            assert_eq!(port, 8080);
        }

        #[test]
        fn parses_high_ipv4_with_each_octet() {
            // 10.20.30.40 in little-endian = 281E140A
            let (ip, port) = parse_address("281E140A:0050").unwrap();
            assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(10, 20, 30, 40)));
            assert_eq!(port, 80);
        }

        #[test]
        fn parses_ipv6_loopback() {
            let (ip, port) = parse_address("00000000000000000000000001000000:0050").unwrap();
            assert_eq!(ip, IpAddr::V6(Ipv6Addr::LOCALHOST));
            assert_eq!(port, 80);
        }

        #[test]
        fn rejects_malformed_address() {
            assert!(parse_address("nothex:0050").is_none());
            assert!(parse_address("01:00").is_none());
            assert!(parse_address("0100007F").is_none());
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod backend {
    use std::net::SocketAddr;

    pub(super) fn lookup(_peer_addr: SocketAddr) -> Option<(u32, String)> {
        None
    }
}
