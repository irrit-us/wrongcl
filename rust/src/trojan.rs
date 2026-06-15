use std::net::{Ipv4Addr, Ipv6Addr};

use sha2::{Digest, Sha224};

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
}
