use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TargetAddress {
    pub host: String,
    pub port: u16,
}

impl TargetAddress {
    pub fn key(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub enum SocksRequest {
    Connect(TargetAddress),
    UdpAssociate,
}

pub async fn perform_handshake<S>(stream: &mut S) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut version = [0u8; 2];
    stream.read_exact(&mut version).await?;
    if version[0] != 0x05 {
        bail!("unsupported SOCKS version {}", version[0]);
    }

    let mut methods = vec![0u8; usize::from(version[1])];
    stream.read_exact(&mut methods).await?;
    stream.write_all(&[0x05, 0x00]).await?;
    stream.flush().await?;
    Ok(())
}

pub async fn read_request<S>(stream: &mut S) -> Result<SocksRequest>
where
    S: AsyncRead + Unpin,
{
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != 0x05 {
        bail!("invalid request version {}", header[0]);
    }

    let target = read_target(stream, header[3]).await?;
    match header[1] {
        0x01 => Ok(SocksRequest::Connect(target)),
        0x03 => {
            let _ = target;
            Ok(SocksRequest::UdpAssociate)
        }
        command => bail!("unsupported SOCKS command {command}"),
    }
}

pub async fn write_connect_success<S>(stream: &mut S) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;
    stream.flush().await?;
    Ok(())
}

pub async fn write_udp_associate_success<S>(stream: &mut S, bind_addr: SocketAddr) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    let mut reply = vec![0x05, 0x00, 0x00];
    append_target_addr(
        &mut reply,
        &TargetAddress {
            host: bind_addr.ip().to_string(),
            port: bind_addr.port(),
        },
    )?;
    stream.write_all(&reply).await?;
    stream.flush().await?;
    Ok(())
}

pub async fn write_failure<S>(stream: &mut S, code: u8) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    stream
        .write_all(&[0x05, code, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;
    stream.flush().await?;
    Ok(())
}

pub fn parse_udp_packet(packet: &[u8]) -> Result<(TargetAddress, Bytes)> {
    if packet.len() < 4 {
        bail!("short SOCKS UDP packet");
    }
    if packet[0] != 0 || packet[1] != 0 {
        bail!("invalid SOCKS UDP reserved bytes");
    }
    if packet[2] != 0 {
        bail!("fragmented SOCKS UDP packets are unsupported");
    }

    let (target, consumed) = parse_target_from_slice(&packet[3..])?;
    Ok((target, Bytes::copy_from_slice(&packet[3 + consumed..])))
}

pub fn encode_udp_packet(target: &TargetAddress, payload: &[u8]) -> Result<Vec<u8>> {
    let mut packet = vec![0x00, 0x00, 0x00];
    append_target_addr(&mut packet, target)?;
    packet.extend_from_slice(payload);
    Ok(packet)
}

fn append_target_addr(buffer: &mut Vec<u8>, target: &TargetAddress) -> Result<()> {
    if let Ok(ip) = target.host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) => {
                buffer.push(0x01);
                buffer.extend_from_slice(&v4.octets());
            }
            IpAddr::V6(v6) => {
                buffer.push(0x04);
                buffer.extend_from_slice(&v6.octets());
            }
        }
    } else {
        if target.host.is_empty() || target.host.len() > 255 {
            bail!("invalid target host");
        }
        buffer.push(0x03);
        buffer.push(u8::try_from(target.host.len()).unwrap_or(0));
        buffer.extend_from_slice(target.host.as_bytes());
    }

    buffer.extend_from_slice(&target.port.to_be_bytes());
    Ok(())
}

async fn read_target<S>(stream: &mut S, atyp: u8) -> Result<TargetAddress>
where
    S: AsyncRead + Unpin,
{
    match atyp {
        0x01 => {
            let mut buf = [0u8; 6];
            stream.read_exact(&mut buf).await?;
            Ok(TargetAddress {
                host: IpAddr::V4(Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3])).to_string(),
                port: u16::from_be_bytes([buf[4], buf[5]]),
            })
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut buf = vec![0u8; usize::from(len[0]) + 2];
            stream.read_exact(&mut buf).await?;
            Ok(TargetAddress {
                host: String::from_utf8(buf[..usize::from(len[0])].to_vec())
                    .context("invalid domain target")?,
                port: u16::from_be_bytes([buf[usize::from(len[0])], buf[usize::from(len[0]) + 1]]),
            })
        }
        0x04 => {
            let mut buf = [0u8; 18];
            stream.read_exact(&mut buf).await?;
            Ok(TargetAddress {
                host: IpAddr::from(<[u8; 16]>::try_from(&buf[..16]).unwrap()).to_string(),
                port: u16::from_be_bytes([buf[16], buf[17]]),
            })
        }
        other => bail!("unsupported address type {other:#x}"),
    }
}

fn parse_target_from_slice(packet: &[u8]) -> Result<(TargetAddress, usize)> {
    if packet.is_empty() {
        bail!("missing UDP address type");
    }

    match packet[0] {
        0x01 => {
            if packet.len() < 7 {
                bail!("short UDP IPv4 target");
            }
            Ok((
                TargetAddress {
                    host: IpAddr::V4(Ipv4Addr::new(packet[1], packet[2], packet[3], packet[4]))
                        .to_string(),
                    port: u16::from_be_bytes([packet[5], packet[6]]),
                },
                7,
            ))
        }
        0x03 => {
            if packet.len() < 2 {
                bail!("short UDP domain target");
            }
            let name_len = usize::from(packet[1]);
            if packet.len() < 4 + name_len {
                bail!("short UDP domain target");
            }
            Ok((
                TargetAddress {
                    host: String::from_utf8(packet[2..2 + name_len].to_vec())
                        .context("invalid UDP domain target")?,
                    port: u16::from_be_bytes([packet[2 + name_len], packet[3 + name_len]]),
                },
                4 + name_len,
            ))
        }
        0x04 => {
            if packet.len() < 19 {
                bail!("short UDP IPv6 target");
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&packet[1..17]);
            Ok((
                TargetAddress {
                    host: IpAddr::from(octets).to_string(),
                    port: u16::from_be_bytes([packet[17], packet[18]]),
                },
                19,
            ))
        }
        other => bail!("unsupported UDP address type {other:#x}"),
    }
}
