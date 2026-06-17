use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::validate_port;
use crate::error::{ClientError, Result};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Target {
    pub host: String,
    pub port: u16,
}

impl Target {
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self> {
        let target = Self {
            host: host.into(),
            port,
        };
        target.validate()?;
        Ok(target)
    }

    pub fn validate(&self) -> Result<()> {
        if self.host.trim().is_empty() {
            return Err(ClientError::Config("target host is required".into()));
        }
        validate_port(self.port, "target port")?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VlessAddress {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    Domain(String),
}

impl VlessAddress {
    pub fn parse(host: &str) -> Result<Self> {
        let host = host.trim();
        if host.is_empty() {
            return Err(ClientError::Config("host is required".into()));
        }

        let bracketless = host
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
            .unwrap_or(host);

        if let Ok(ip) = bracketless.parse::<Ipv4Addr>() {
            return Ok(Self::Ipv4(ip));
        }

        if let Ok(ip) = bracketless.parse::<Ipv6Addr>() {
            return Ok(Self::Ipv6(ip));
        }

        if host.len() > u8::MAX as usize {
            return Err(ClientError::Config("domain must be 1..255 bytes".into()));
        }

        Ok(Self::Domain(host.to_string()))
    }

    pub fn write_to(&self, mut writer: impl Write) -> Result<()> {
        match self {
            VlessAddress::Ipv4(ip) => {
                writer.write_all(&[0x01])?;
                writer.write_all(&ip.octets())?;
            }
            VlessAddress::Domain(domain) => {
                writer.write_all(&[0x02, domain.len() as u8])?;
                writer.write_all(domain.as_bytes())?;
            }
            VlessAddress::Ipv6(ip) => {
                writer.write_all(&[0x03])?;
                writer.write_all(&ip.octets())?;
            }
        }
        Ok(())
    }
}

pub fn encode_raw_vless_header(uuid: &str, target: &Target, flow: &str) -> Result<Vec<u8>> {
    encode_vless_header(uuid, target, flow, 0x01)
}

pub fn encode_udp_vless_header(uuid: &str, target: &Target, flow: &str) -> Result<Vec<u8>> {
    encode_vless_header(uuid, target, flow, 0x02)
}

fn encode_vless_header(uuid: &str, target: &Target, flow: &str, command: u8) -> Result<Vec<u8>> {
    target.validate()?;
    let parsed_uuid = Uuid::parse_str(uuid.trim())
        .map_err(|e| ClientError::Config(format!("invalid UUID '{uuid}': {e}")))?;
    let address = VlessAddress::parse(&target.host)?;

    let mut header = Vec::with_capacity(64 + target.host.len() + flow.len());
    header.push(0x00);
    header.extend_from_slice(parsed_uuid.as_bytes());

    let flow = flow.trim();
    if flow.is_empty() {
        header.push(0x00);
    } else {
        if flow.len() >= 128 {
            return Err(ClientError::Config(format!(
                "VLESS flow string too long ({} bytes, max 127)",
                flow.len()
            )));
        }
        let addons_len = 2 + flow.len();
        header.push(addons_len as u8);
        header.push(0x0a);
        header.push(flow.len() as u8);
        header.extend_from_slice(flow.as_bytes());
    }

    header.push(command);
    header.extend_from_slice(&target.port.to_be_bytes());
    address.write_to(&mut header)?;
    Ok(header)
}

pub fn read_raw_vless_response(mut reader: impl Read) -> Result<()> {
    let mut response = [0u8; 2];
    reader.read_exact(&mut response)?;
    if response[0] != 0x00 {
        return Err(ClientError::Config(format!(
            "invalid VLESS response version: {}",
            response[0]
        )));
    }
    if response[1] > 0 {
        let mut addons = vec![0u8; response[1] as usize];
        reader.read_exact(&mut addons)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_UUID: &str = "12345678-1234-1234-1234-123456789abc";

    #[test]
    fn encodes_domain_header() {
        let header =
            encode_raw_vless_header(TEST_UUID, &Target::new("example.com", 443).unwrap(), "")
                .unwrap();

        assert_eq!(header[0], 0x00);
        assert_eq!(
            &header[1..17],
            Uuid::parse_str(TEST_UUID).unwrap().as_bytes()
        );
        assert_eq!(header[17], 0x00);
        assert_eq!(header[18], 0x01);
        assert_eq!(&header[19..21], &443u16.to_be_bytes());
        assert_eq!(header[21], 0x02);
        assert_eq!(header[22], "example.com".len() as u8);
        assert_eq!(&header[23..], b"example.com");
    }

    #[test]
    fn encodes_ipv4_header() {
        let header =
            encode_raw_vless_header(TEST_UUID, &Target::new("127.0.0.1", 80).unwrap(), "").unwrap();

        assert_eq!(header[21], 0x01);
        assert_eq!(&header[22..26], &[127, 0, 0, 1]);
    }

    #[test]
    fn rejects_invalid_uuid() {
        let err = encode_raw_vless_header("not-a-uuid", &Target::new("127.0.0.1", 80).unwrap(), "")
            .unwrap_err();

        assert!(matches!(err, ClientError::Config(_)));
    }

    #[test]
    fn encodes_vision_flow_addons() {
        let header = encode_raw_vless_header(
            TEST_UUID,
            &Target::new("example.com", 443).unwrap(),
            "xtls-rprx-vision",
        )
        .unwrap();

        assert_eq!(header[0], 0x00);
        assert_eq!(header[17], 18);
        assert_eq!(header[18], 0x0a);
        assert_eq!(header[19], 16);
        assert_eq!(&header[20..36], b"xtls-rprx-vision");
        assert_eq!(header[36], 0x01);
        assert_eq!(&header[37..39], &443u16.to_be_bytes());
        assert_eq!(header[39], 0x02);
        assert_eq!(header[40], "example.com".len() as u8);
        assert_eq!(&header[41..], b"example.com");
    }
}
