use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const TUIC_VERSION: u8 = 0x05;
const TUIC_CMD_CONNECT: u8 = 0x01;
const TUIC_CMD_PACKET: u8 = 0x02;
const TUIC_ADDR_NONE: u8 = 0xff;
const TUIC_MAX_DATAGRAM_PAYLOAD: usize = 1200;

fn encode_tuic_connect(address: &str) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(address.len() + 24);
    out.push(TUIC_VERSION);
    out.push(TUIC_CMD_CONNECT);
    encode_tuic_address(address, &mut out)?;
    Ok(out)
}

fn encode_tuic_address(address: &str, out: &mut Vec<u8>) -> std::io::Result<()> {
    let (host, port) = split_tuic_host_port(address)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    if let Ok(ip) = host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) => {
                out.push(0x01);
                out.extend_from_slice(&v4.octets());
                out.extend_from_slice(&port.to_be_bytes());
            }
            IpAddr::V6(v6) => {
                out.push(0x02);
                out.extend_from_slice(&v6.octets());
                out.extend_from_slice(&port.to_be_bytes());
            }
        }
    } else {
        out.push(0x00);
        out.push(host.len() as u8);
        out.extend_from_slice(host.as_bytes());
        out.extend_from_slice(&port.to_be_bytes());
    }
    Ok(())
}

fn split_tuic_host_port(s: &str) -> std::result::Result<(&str, u16), &'static str> {
    if let Some(rest) = s.strip_prefix('[') {
        let end = rest.find("]:").ok_or("invalid IPv6 host:port")?;
        let host = &rest[..end];
        let port = rest[end + 2..].parse::<u16>().map_err(|_| "invalid port")?;
        return Ok((host, port));
    }
    let (host, port) = s.rsplit_once(':').ok_or("missing port")?;
    Ok((host, port.parse::<u16>().map_err(|_| "invalid port")?))
}

pub(super) fn fragment_tuic_payload(
    assoc_id: u16,
    address: &str,
    payload: &[u8],
    packet_id: u16,
) -> std::result::Result<Vec<Vec<u8>>, String> {
    if payload.len() <= TUIC_MAX_DATAGRAM_PAYLOAD {
        return Ok(vec![encode_tuic_packet(
            assoc_id, packet_id, 0, 1, address, payload,
        )?]);
    }
    let mut fragments = Vec::new();
    let chunk_count = payload
        .len()
        .div_ceil(TUIC_MAX_DATAGRAM_PAYLOAD)
        .min(u8::MAX as usize);
    let total = chunk_count as u8;
    for (idx, chunk) in payload.chunks(TUIC_MAX_DATAGRAM_PAYLOAD).enumerate() {
        let addr = if idx == 0 { address } else { "" };
        fragments.push(encode_tuic_packet(
            assoc_id, packet_id, idx as u8, total, addr, chunk,
        )?);
    }
    Ok(fragments)
}

fn encode_tuic_packet(
    assoc_id: u16,
    packet_id: u16,
    fragment_index: u8,
    fragment_total: u8,
    address: &str,
    payload: &[u8],
) -> std::result::Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(16 + address.len() + payload.len());
    out.push(TUIC_VERSION);
    out.push(TUIC_CMD_PACKET);
    out.extend_from_slice(&assoc_id.to_be_bytes());
    out.extend_from_slice(&packet_id.to_be_bytes());
    out.push(fragment_total);
    out.push(fragment_index);
    out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    if fragment_index == 0 {
        encode_tuic_address(address, &mut out).map_err(|err| err.to_string())?;
    } else {
        out.push(TUIC_ADDR_NONE);
    }
    out.extend_from_slice(payload);
    Ok(out)
}

pub(super) fn parse_tuic_datagram_command(
    packet: &[u8],
) -> std::result::Result<TuicCommand, String> {
    let (cmd, consumed) =
        try_parse_tuic_command(packet)?.ok_or("TUIC datagram too short to contain a command")?;
    if consumed != packet.len() {
        return Err(format!(
            "TUIC datagram contains {} trailing bytes",
            packet.len() - consumed
        ));
    }
    Ok(cmd)
}

fn try_parse_tuic_command(buf: &[u8]) -> std::result::Result<Option<(TuicCommand, usize)>, String> {
    let mut pos = 0usize;
    let Some(version) = buf.get(pos).copied() else {
        return Ok(None);
    };
    pos += 1;
    if version != TUIC_VERSION {
        return Err(format!("unexpected TUIC version: {version:#x}"));
    }
    let Some(cmd) = buf.get(pos).copied() else {
        return Ok(None);
    };
    pos += 1;
    match cmd {
        TUIC_CMD_PACKET => {
            if buf.len() < pos + 8 {
                return Ok(None);
            }
            let assoc_id = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
            pos += 2;
            let packet_id = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
            pos += 2;
            let frag_total = buf[pos];
            pos += 1;
            let fragment_index = buf[pos];
            pos += 1;
            let size = u16::from_be_bytes([buf[pos], buf[pos + 1]]) as usize;
            pos += 2;
            let address = parse_tuic_address(buf, &mut pos)?;
            if buf.len() < pos + size {
                return Ok(None);
            }
            let payload = buf[pos..pos + size].to_vec();
            pos += size;
            Ok(Some((
                TuicCommand::Packet(TuicPacket {
                    assoc_id,
                    packet_id,
                    frag_total,
                    fragment_index,
                    address,
                    payload,
                }),
                pos,
            )))
        }
        _ => Ok(None),
    }
}

pub(super) enum TuicCommand {
    Packet(TuicPacket),
}

pub(super) struct TuicPacket {
    pub(super) assoc_id: u16,
    pub(super) packet_id: u16,
    pub(super) frag_total: u8,
    pub(super) fragment_index: u8,
    pub(super) address: Option<String>,
    pub(super) payload: Vec<u8>,
}

pub(super) struct TuicPacketAssembly {
    fragments: Vec<Option<Vec<u8>>>,
    address: Option<String>,
}

impl TuicPacketAssembly {
    pub(super) fn new(fragment_total: u8) -> Self {
        Self {
            fragments: vec![None; fragment_total as usize],
            address: None,
        }
    }

    pub(super) fn insert(
        &mut self,
        fragment_index: u8,
        address: Option<String>,
        payload: Vec<u8>,
    ) -> std::result::Result<(), String> {
        let idx = fragment_index as usize;
        if idx >= self.fragments.len() {
            return Err("invalid TUIC fragment index".into());
        }
        if self.address.is_none() && address.is_some() {
            self.address = address;
        }
        self.fragments[idx] = Some(payload);
        Ok(())
    }

    pub(super) fn is_complete(&self) -> bool {
        self.fragments.iter().all(Option::is_some)
    }

    pub(super) fn take_payload(
        &mut self,
    ) -> std::result::Result<(Option<String>, Vec<u8>), String> {
        let mut out = Vec::new();
        for fragment in self.fragments.iter_mut() {
            out.extend_from_slice(fragment.take().as_deref().ok_or("missing TUIC fragment")?);
        }
        Ok((self.address.take(), out))
    }
}

fn parse_tuic_address(buf: &[u8], pos: &mut usize) -> std::result::Result<Option<String>, String> {
    let Some(addr_type) = buf.get(*pos).copied() else {
        return Ok(None);
    };
    *pos += 1;
    match addr_type {
        TUIC_ADDR_NONE => Ok(None),
        0x00 => {
            let Some(len) = buf.get(*pos).copied() else {
                return Ok(None);
            };
            *pos += 1;
            let len = len as usize;
            if buf.len() < *pos + len + 2 {
                return Ok(None);
            }
            let host = std::str::from_utf8(&buf[*pos..*pos + len])
                .map_err(|err| err.to_string())?
                .to_string();
            *pos += len;
            let port = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
            *pos += 2;
            Ok(Some(format!("{host}:{port}")))
        }
        0x01 => {
            if buf.len() < *pos + 4 + 2 {
                return Ok(None);
            }
            let raw = [buf[*pos], buf[*pos + 1], buf[*pos + 2], buf[*pos + 3]];
            *pos += 4;
            let port = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
            *pos += 2;
            Ok(Some(format!("{}:{port}", Ipv4Addr::from(raw))))
        }
        0x02 => {
            if buf.len() < *pos + 16 + 2 {
                return Ok(None);
            }
            let mut raw = [0u8; 16];
            raw.copy_from_slice(&buf[*pos..*pos + 16]);
            *pos += 16;
            let port = u16::from_be_bytes([buf[*pos], buf[*pos + 1]]);
            *pos += 2;
            Ok(Some(format!("[{}]:{port}", Ipv6Addr::from(raw))))
        }
        other => Err(format!("unexpected TUIC address type: {other:#x}")),
    }
}

pub(super) fn target_authority(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub(super) fn encode_connect_request(target: &str) -> std::io::Result<Vec<u8>> {
    encode_tuic_connect(target)
}
