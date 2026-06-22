use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore};
use serde::{Deserialize, Serialize};

use crate::error::{ClientError, Result};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum DnsBackend {
    #[default]
    System,
    Udp {
        server: SocketAddr,
    },
    Doh {
        url: String,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnsSettings {
    #[serde(default)]
    pub backend: DnsBackend,
}

impl DnsSettings {
    pub fn validate(&self) -> Result<()> {
        if let DnsBackend::Doh { url } = &self.backend {
            let parsed = parse_doh_url(url)?;
            if parsed.host.is_empty() {
                return Err(ClientError::Config(format!(
                    "DoH url '{url}' is missing a hostname"
                )));
            }
        }
        Ok(())
    }
}

pub fn resolve(host: &str, settings: &DnsSettings) -> Result<Vec<IpAddr>> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![ip]);
    }
    match &settings.backend {
        DnsBackend::System => resolve_system(host),
        DnsBackend::Udp { server } => resolve_udp(host, *server),
        DnsBackend::Doh { url } => resolve_doh(host, url),
    }
}

fn resolve_system(host: &str) -> Result<Vec<IpAddr>> {
    let mut ips = Vec::new();
    for addr in (host, 0u16).to_socket_addrs()? {
        let ip = addr.ip();
        if !ips.contains(&ip) {
            ips.push(ip);
        }
    }
    if ips.is_empty() {
        return Err(ClientError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("system resolver returned no records for '{host}'"),
        )));
    }
    Ok(ips)
}

fn resolve_udp(host: &str, server: SocketAddr) -> Result<Vec<IpAddr>> {
    let mut ips = Vec::new();
    for qtype in [QTYPE_A, QTYPE_AAAA] {
        match udp_query(host, server, qtype) {
            Ok(answers) => ips.extend(answers),
            Err(e) if qtype == QTYPE_AAAA && !ips.is_empty() => {
                tracing::debug!(?e, "DNS AAAA query failed but A succeeded; continuing");
            }
            Err(e) => {
                if qtype == QTYPE_AAAA {
                    return Err(e);
                }
            }
        }
    }
    if ips.is_empty() {
        return Err(ClientError::Config(format!(
            "DNS resolver returned no records for '{host}'"
        )));
    }
    Ok(ips)
}

fn udp_query(host: &str, server: SocketAddr, qtype: u16) -> Result<Vec<IpAddr>> {
    let bind = match server {
        SocketAddr::V4(_) => "0.0.0.0:0",
        SocketAddr::V6(_) => "[::]:0",
    };
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    let query = build_query(host, qtype, 0xabcd)?;
    socket.send_to(&query, server)?;
    let mut buf = [0u8; 1500];
    let n = socket.recv(&mut buf)?;
    parse_answers(&buf[..n], qtype)
}

fn resolve_doh(host: &str, url: &str) -> Result<Vec<IpAddr>> {
    let mut ips = Vec::new();
    for qtype in [QTYPE_A, QTYPE_AAAA] {
        match doh_query(host, url, qtype) {
            Ok(answers) => ips.extend(answers),
            Err(e) => {
                if qtype == QTYPE_AAAA && !ips.is_empty() {
                    tracing::debug!(?e, "DoH AAAA query failed but A succeeded; continuing");
                } else if qtype == QTYPE_AAAA {
                    return Err(e);
                }
            }
        }
    }
    if ips.is_empty() {
        return Err(ClientError::Config(format!(
            "DoH resolver returned no records for '{host}'"
        )));
    }
    Ok(ips)
}

fn doh_query(host: &str, url: &str, qtype: u16) -> Result<Vec<IpAddr>> {
    let parsed = parse_doh_url(url)?;
    let query = build_query(host, qtype, 0)?;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&query);
    let path = if parsed.path.contains('?') {
        format!("{}&dns={}", parsed.path, b64)
    } else {
        format!("{}?dns={}", parsed.path, b64)
    };
    let addr = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| {
            ClientError::Config(format!("DoH host '{}' did not resolve", parsed.host))
        })?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let server_name = ServerName::try_from(parsed.host.clone())
        .map_err(|_| ClientError::Config(format!("invalid DoH hostname: {}", parsed.host)))?;
    let config = doh_tls_config();
    let mut conn = ClientConnection::new(config, server_name)
        .map_err(|e| ClientError::Config(format!("rustls: {e}")))?;
    let mut tls = rustls::Stream::new(&mut conn, &mut stream);
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: wrongcl\r\nAccept: application/dns-message\r\nConnection: close\r\n\r\n",
        path, parsed.host
    );
    tls.write_all(request.as_bytes())?;
    tls.flush().ok();
    let mut response = Vec::new();
    match tls.read_to_end(&mut response) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
        Err(e) => return Err(ClientError::Io(e)),
    }
    let body = parse_http_response(&response)?;
    parse_answers(body, qtype)
}

fn doh_tls_config() -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

struct DohUrl {
    host: String,
    port: u16,
    path: String,
}

fn parse_doh_url(url: &str) -> Result<DohUrl> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| ClientError::Config(format!("DoH url must start with https://: {url}")))?;
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/dns-query"),
    };
    let (host, port) = if let Some(idx) = authority.rfind(':') {
        let port_str = &authority[idx + 1..];
        if let Ok(port) = port_str.parse::<u16>() {
            (&authority[..idx], port)
        } else {
            (authority, 443)
        }
    } else {
        (authority, 443)
    };
    if host.is_empty() {
        return Err(ClientError::Config(format!("DoH url missing host: {url}")));
    }
    Ok(DohUrl {
        host: host.to_string(),
        port,
        path: path.to_string(),
    })
}

fn parse_http_response(bytes: &[u8]) -> Result<&[u8]> {
    let sep = b"\r\n\r\n";
    let header_end = bytes
        .windows(sep.len())
        .position(|w| w == sep)
        .ok_or_else(|| ClientError::Config("DoH response missing header terminator".into()))?;
    let header = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| ClientError::Config("DoH response header is not UTF-8".into()))?;
    let mut lines = header.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| ClientError::Config("DoH response missing status line".into()))?;
    let mut parts = status_line.splitn(3, ' ');
    let _version = parts.next();
    let code = parts
        .next()
        .ok_or_else(|| ClientError::Config(format!("DoH status malformed: {status_line}")))?;
    if code != "200" {
        return Err(ClientError::Config(format!(
            "DoH server returned HTTP {code}"
        )));
    }
    Ok(&bytes[header_end + sep.len()..])
}

const QTYPE_A: u16 = 1;
const QTYPE_AAAA: u16 = 28;
const QCLASS_IN: u16 = 1;

fn build_query(host: &str, qtype: u16, id: u16) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&id.to_be_bytes());
    buf.extend_from_slice(&0x0100u16.to_be_bytes()); // RD=1
    buf.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    buf.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT
    buf.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    buf.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT
    encode_name(&mut buf, host)?;
    buf.extend_from_slice(&qtype.to_be_bytes());
    buf.extend_from_slice(&QCLASS_IN.to_be_bytes());
    Ok(buf)
}

fn encode_name(buf: &mut Vec<u8>, host: &str) -> Result<()> {
    let trimmed = host.trim_end_matches('.');
    for label in trimmed.split('.') {
        if label.is_empty() {
            return Err(ClientError::Config(format!(
                "DNS name '{host}' has an empty label"
            )));
        }
        if label.len() > 63 {
            return Err(ClientError::Config(format!(
                "DNS label '{label}' exceeds 63 octets"
            )));
        }
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0);
    Ok(())
}

fn parse_answers(bytes: &[u8], wanted_qtype: u16) -> Result<Vec<IpAddr>> {
    if bytes.len() < 12 {
        return Err(ClientError::Config(
            "DNS response truncated below header".into(),
        ));
    }
    let flags = u16::from_be_bytes([bytes[2], bytes[3]]);
    let rcode = flags & 0x000f;
    if rcode != 0 {
        return Err(ClientError::Config(format!(
            "DNS server returned rcode {rcode}"
        )));
    }
    let qdcount = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    let ancount = u16::from_be_bytes([bytes[6], bytes[7]]) as usize;
    let mut cursor = 12usize;
    for _ in 0..qdcount {
        cursor = skip_name(bytes, cursor)?;
        if cursor + 4 > bytes.len() {
            return Err(ClientError::Config(
                "DNS response truncated in question section".into(),
            ));
        }
        cursor += 4; // qtype + qclass
    }
    let mut ips = Vec::new();
    for _ in 0..ancount {
        cursor = skip_name(bytes, cursor)?;
        if cursor + 10 > bytes.len() {
            return Err(ClientError::Config(
                "DNS response truncated in answer header".into(),
            ));
        }
        let rtype = u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]);
        let rclass = u16::from_be_bytes([bytes[cursor + 2], bytes[cursor + 3]]);
        let rdlength = u16::from_be_bytes([bytes[cursor + 8], bytes[cursor + 9]]) as usize;
        cursor += 10;
        if cursor + rdlength > bytes.len() {
            return Err(ClientError::Config(
                "DNS response truncated in rdata".into(),
            ));
        }
        if rclass == QCLASS_IN && rtype == wanted_qtype {
            match wanted_qtype {
                QTYPE_A if rdlength == 4 => {
                    let octets = [
                        bytes[cursor],
                        bytes[cursor + 1],
                        bytes[cursor + 2],
                        bytes[cursor + 3],
                    ];
                    ips.push(IpAddr::V4(Ipv4Addr::from(octets)));
                }
                QTYPE_AAAA if rdlength == 16 => {
                    let mut octets = [0u8; 16];
                    octets.copy_from_slice(&bytes[cursor..cursor + 16]);
                    ips.push(IpAddr::V6(Ipv6Addr::from(octets)));
                }
                _ => {}
            }
        }
        cursor += rdlength;
    }
    Ok(ips)
}

fn skip_name(bytes: &[u8], mut cursor: usize) -> Result<usize> {
    loop {
        if cursor >= bytes.len() {
            return Err(ClientError::Config(
                "DNS name extends past response end".into(),
            ));
        }
        let len = bytes[cursor];
        if len == 0 {
            return Ok(cursor + 1);
        }
        if len & 0xc0 == 0xc0 {
            if cursor + 2 > bytes.len() {
                return Err(ClientError::Config(
                    "DNS name pointer extends past response end".into(),
                ));
            }
            return Ok(cursor + 2);
        }
        cursor += 1 + len as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_is_system() {
        let s = DnsSettings::default();
        assert_eq!(s.backend, DnsBackend::System);
    }

    #[test]
    fn validate_rejects_non_https_doh() {
        let s = DnsSettings {
            backend: DnsBackend::Doh {
                url: "http://example.com/dns-query".into(),
            },
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn parses_doh_url_default_port_and_path() {
        let p = parse_doh_url("https://1.1.1.1/dns-query").unwrap();
        assert_eq!(p.host, "1.1.1.1");
        assert_eq!(p.port, 443);
        assert_eq!(p.path, "/dns-query");
    }

    #[test]
    fn parses_doh_url_with_port() {
        let p = parse_doh_url("https://dns.example:8443/q").unwrap();
        assert_eq!(p.host, "dns.example");
        assert_eq!(p.port, 8443);
        assert_eq!(p.path, "/q");
    }

    #[test]
    fn parses_doh_url_without_path_defaults() {
        let p = parse_doh_url("https://dns.example").unwrap();
        assert_eq!(p.path, "/dns-query");
    }

    #[test]
    fn build_query_round_trips_through_parser() {
        let q = build_query("example.com", QTYPE_A, 0x1234).unwrap();
        assert_eq!(&q[0..2], &[0x12, 0x34]);
        assert_eq!(&q[2..4], &[0x01, 0x00]);
        assert!(q.ends_with(&[0, 1, 0, 1])); // qtype A, class IN
    }

    #[test]
    fn parse_answers_extracts_ipv4() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u16.to_be_bytes()); // id
        bytes.extend_from_slice(&0x8180u16.to_be_bytes()); // standard response NOERROR
        bytes.extend_from_slice(&1u16.to_be_bytes()); // qd
        bytes.extend_from_slice(&1u16.to_be_bytes()); // an
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        encode_name(&mut bytes, "example.com").unwrap();
        bytes.extend_from_slice(&QTYPE_A.to_be_bytes());
        bytes.extend_from_slice(&QCLASS_IN.to_be_bytes());
        // Answer: pointer to question name (0xc00c), type A, class IN, ttl=0, rdlength=4, 1.2.3.4
        bytes.extend_from_slice(&[0xc0, 0x0c]);
        bytes.extend_from_slice(&QTYPE_A.to_be_bytes());
        bytes.extend_from_slice(&QCLASS_IN.to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&4u16.to_be_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4]);
        let ips = parse_answers(&bytes, QTYPE_A).unwrap();
        assert_eq!(ips, vec![IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))]);
    }

    #[test]
    fn parse_answers_skips_unrelated_record_types() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0x8180u16.to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        encode_name(&mut bytes, "example.com").unwrap();
        bytes.extend_from_slice(&QTYPE_A.to_be_bytes());
        bytes.extend_from_slice(&QCLASS_IN.to_be_bytes());
        // Wrong qtype: CNAME (5)
        bytes.extend_from_slice(&[0xc0, 0x0c]);
        bytes.extend_from_slice(&5u16.to_be_bytes());
        bytes.extend_from_slice(&QCLASS_IN.to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&[0xc0, 0x0c]);
        let ips = parse_answers(&bytes, QTYPE_A).unwrap();
        assert!(ips.is_empty());
    }

    #[test]
    fn parse_answers_rejects_nonzero_rcode() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0x8183u16.to_be_bytes()); // NXDOMAIN (rcode=3)
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        let err = parse_answers(&bytes, QTYPE_A).unwrap_err();
        assert!(err.to_string().contains("rcode 3"));
    }

    #[test]
    fn resolve_returns_literal_ip_unchanged() {
        let s = DnsSettings::default();
        let ips = resolve("203.0.113.7", &s).unwrap();
        assert_eq!(ips, vec![IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))]);
    }

    #[test]
    fn resolve_udp_against_in_process_server() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = socket.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; 1500];
            let (n, peer) = socket.recv_from(&mut buf).unwrap();
            let id = [buf[0], buf[1]];
            let mut response = Vec::new();
            response.extend_from_slice(&id);
            response.extend_from_slice(&0x8180u16.to_be_bytes());
            response.extend_from_slice(&1u16.to_be_bytes());
            response.extend_from_slice(&1u16.to_be_bytes());
            response.extend_from_slice(&0u16.to_be_bytes());
            response.extend_from_slice(&0u16.to_be_bytes());
            // Echo the question section
            response.extend_from_slice(&buf[12..n]);
            // Answer: pointer to question name, A 9.9.9.9
            response.extend_from_slice(&[0xc0, 0x0c]);
            response.extend_from_slice(&QTYPE_A.to_be_bytes());
            response.extend_from_slice(&QCLASS_IN.to_be_bytes());
            response.extend_from_slice(&60u32.to_be_bytes());
            response.extend_from_slice(&4u16.to_be_bytes());
            response.extend_from_slice(&[9, 9, 9, 9]);
            socket.send_to(&response, peer).unwrap();
        });

        let settings = DnsSettings {
            backend: DnsBackend::Udp {
                server: server_addr,
            },
        };
        let ips = resolve("example.com", &settings).unwrap();
        handle.join().unwrap();
        assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9))));
    }
}
