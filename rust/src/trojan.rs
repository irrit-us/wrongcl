use std::net::{Ipv4Addr, Ipv6Addr};

use sha2::{Digest, Sha224};
use wrongsv_net_types::{Address, Port};

use crate::error::{ClientError, Result};
use crate::protocol::Target;

pub fn encode_handshake(password: &str, target: &Target) -> Result<Vec<u8>> {
    target.validate()?;
    if password.is_empty() {
        return Err(ClientError::Config(
            "Trojan password must not be empty".into(),
        ));
    }

    let mut hash_hex = String::with_capacity(56);
    for byte in Sha224::digest(password.as_bytes()) {
        hash_hex.push_str(&format!("{byte:02x}"));
    }

    let mut buf = Vec::with_capacity(64 + target.host.len());
    buf.extend_from_slice(hash_hex.as_bytes());
    buf.extend_from_slice(b"\r\n");
    buf.push(0x01);
    write_trojan_address(&mut buf, &target.host, target.port)?;
    buf.extend_from_slice(b"\r\n");
    Ok(buf)
}

pub fn encode_udp_associate_handshake(password: &str) -> Result<Vec<u8>> {
    if password.is_empty() {
        return Err(ClientError::Config(
            "Trojan password must not be empty".into(),
        ));
    }

    let mut hash_hex = String::with_capacity(56);
    for byte in Sha224::digest(password.as_bytes()) {
        hash_hex.push_str(&format!("{byte:02x}"));
    }

    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(hash_hex.as_bytes());
    buf.extend_from_slice(b"\r\n");
    buf.push(0x03);
    buf.push(0x01);
    buf.extend_from_slice(&[0, 0, 0, 0]);
    buf.extend_from_slice(&0u16.to_be_bytes());
    buf.extend_from_slice(b"\r\n");
    Ok(buf)
}

pub fn encode_udp_packet(target: &Target, payload: &[u8]) -> Result<Vec<u8>> {
    target.validate()?;
    if payload.len() > u16::MAX as usize {
        return Err(ClientError::Config(
            "Trojan UDP payload must be at most 65535 bytes".into(),
        ));
    }
    let address = Address::parse(&target.host);
    let port = Port(target.port);
    let mut buf = Vec::with_capacity(payload.len() + target.host.len() + 32);
    write_udp_packet(&mut buf, &address, port, payload)?;
    Ok(buf)
}

pub fn parse_udp_packet(data: &[u8]) -> Result<Option<(Target, Vec<u8>, usize)>> {
    let mut pos = 0;
    if data.is_empty() {
        return Ok(None);
    }
    let address_type = data[pos];
    pos += 1;
    let Some(address) = parse_trojan_address(data, &mut pos, address_type)? else {
        return Ok(None);
    };
    if data.len() < pos + 6 {
        return Ok(None);
    }
    let port = u16::from_be_bytes([data[pos], data[pos + 1]]);
    if port == 0 {
        return Err(ClientError::Config(
            "Trojan UDP packet used port zero".into(),
        ));
    }
    pos += 2;
    let payload_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    if &data[pos..pos + 2] != b"\r\n" {
        return Err(ClientError::Config("Trojan UDP packet missing CRLF".into()));
    }
    pos += 2;
    if data.len() < pos + payload_len {
        return Ok(None);
    }
    Ok(Some((
        Target::new(address.to_string(), port)?,
        data[pos..pos + payload_len].to_vec(),
        pos + payload_len,
    )))
}

fn write_trojan_address(buf: &mut Vec<u8>, host: &str, port: u16) -> Result<()> {
    let bracketless = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);

    if let Ok(ip) = bracketless.parse::<Ipv4Addr>() {
        buf.push(0x01);
        buf.extend_from_slice(&ip.octets());
    } else if let Ok(ip) = bracketless.parse::<Ipv6Addr>() {
        buf.push(0x04);
        buf.extend_from_slice(&ip.octets());
    } else {
        let domain = host.as_bytes();
        if domain.is_empty() || domain.len() > u8::MAX as usize {
            return Err(ClientError::Config(
                "Trojan domain must be 1..255 bytes".into(),
            ));
        }
        buf.push(0x03);
        buf.push(domain.len() as u8);
        buf.extend_from_slice(domain);
    }
    buf.extend_from_slice(&port.to_be_bytes());
    Ok(())
}

fn parse_trojan_address(data: &[u8], pos: &mut usize, address_type: u8) -> Result<Option<Address>> {
    match address_type {
        0x01 => {
            if data.len() < *pos + 4 {
                return Ok(None);
            }
            let octets: [u8; 4] = data[*pos..*pos + 4]
                .try_into()
                .map_err(|_| ClientError::Config("invalid Trojan IPv4 address".into()))?;
            *pos += 4;
            Ok(Some(Address::IPv4(octets)))
        }
        0x03 => {
            if data.len() < *pos + 1 {
                return Ok(None);
            }
            let len = data[*pos] as usize;
            *pos += 1;
            if data.len() < *pos + len {
                return Ok(None);
            }
            let domain = String::from_utf8(data[*pos..*pos + len].to_vec())
                .map_err(|_| ClientError::Config("invalid Trojan domain".into()))?;
            *pos += len;
            Ok(Some(Address::Domain(domain)))
        }
        0x04 => {
            if data.len() < *pos + 16 {
                return Ok(None);
            }
            let octets: [u8; 16] = data[*pos..*pos + 16]
                .try_into()
                .map_err(|_| ClientError::Config("invalid Trojan IPv6 address".into()))?;
            *pos += 16;
            Ok(Some(Address::IPv6(octets)))
        }
        other => Err(ClientError::Config(format!(
            "unsupported Trojan UDP address type {other:#04x}"
        ))),
    }
}

fn write_udp_packet(
    out: &mut Vec<u8>,
    address: &Address,
    port: Port,
    payload: &[u8],
) -> Result<()> {
    if port.0 == 0 || payload.len() > u16::MAX as usize {
        return Err(ClientError::Config("invalid Trojan UDP packet".into()));
    }
    match address {
        Address::IPv4(octets) => {
            out.push(0x01);
            out.extend_from_slice(octets);
        }
        Address::Domain(domain) => {
            if domain.is_empty() || domain.len() > u8::MAX as usize {
                return Err(ClientError::Config(
                    "Trojan domain must be 1..255 bytes".into(),
                ));
            }
            out.push(0x03);
            out.push(domain.len() as u8);
            out.extend_from_slice(domain.as_bytes());
        }
        Address::IPv6(octets) => {
            out.push(0x04);
            out.extend_from_slice(octets);
        }
    }
    out.extend_from_slice(&port.0.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(payload);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_domain_handshake() {
        let header =
            encode_handshake("hunter2", &Target::new("example.com", 443).unwrap()).unwrap();
        let expected_hash: String = Sha224::digest(b"hunter2")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(&header[0..56], expected_hash.as_bytes());
        assert_eq!(&header[56..58], b"\r\n");
        assert_eq!(header[58], 0x01);
        assert_eq!(header[59], 0x03);
        assert_eq!(header[60], 11);
        assert_eq!(&header[61..72], b"example.com");
        assert_eq!(&header[72..74], &443u16.to_be_bytes());
        assert_eq!(&header[74..76], b"\r\n");
    }

    #[test]
    fn encodes_ipv4_handshake() {
        let header = encode_handshake("password", &Target::new("127.0.0.1", 80).unwrap()).unwrap();
        assert_eq!(header[58], 0x01);
        assert_eq!(header[59], 0x01);
        assert_eq!(&header[60..64], &[127, 0, 0, 1]);
        assert_eq!(&header[64..66], &80u16.to_be_bytes());
    }

    #[test]
    fn rejects_empty_password() {
        let err = encode_handshake("", &Target::new("127.0.0.1", 80).unwrap()).unwrap_err();
        assert!(matches!(err, ClientError::Config(_)));
    }

    #[test]
    fn udp_packet_roundtrip() {
        let target = Target::new("example.com", 53).unwrap();
        let packet = encode_udp_packet(&target, b"dns").unwrap();
        let (decoded, payload, consumed) = parse_udp_packet(&packet).unwrap().unwrap();
        assert_eq!(decoded, target);
        assert_eq!(payload, b"dns");
        assert_eq!(consumed, packet.len());
    }
}
