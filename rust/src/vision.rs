use std::io::{self, Read, Write};
use std::time::Duration;

use uuid::Uuid;
use wrongsv_vless::vision::{
    CMD_PADDING_CONTINUE, CMD_PADDING_DIRECT, CMD_PADDING_END, TrafficState, is_complete_record,
    xtls_filter_tls, xtls_padding, xtls_unpadding,
};

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::error::{ClientError, Result};

const TLS_APP_DATA_START: [u8; 3] = [0x17, 0x03, 0x03];
const DEFAULT_TESTSEED: [u32; 4] = [900, 500, 900, 256];

pub fn wrap(inner: Box<dyn Tunnel>, uuid: &str) -> Result<Box<dyn Tunnel>> {
    let parsed = Uuid::parse_str(uuid.trim())
        .map_err(|e| ClientError::Config(format!("invalid VLESS UUID for Vision: {e}")))?;
    let bytes = *parsed.as_bytes();
    Ok(Box::new(VisionTunnel::new(inner, bytes)))
}

struct VisionTunnel {
    inner: Box<dyn Tunnel>,
    read_state: TrafficState,
    write_state: TrafficState,
    write_uuid: Option<[u8; 16]>,
    read_direct: bool,
    write_direct: bool,
    raw_buf: Vec<u8>,
    leftover: Vec<u8>,
}

impl VisionTunnel {
    fn new(inner: Box<dyn Tunnel>, uuid_bytes: [u8; 16]) -> Self {
        Self {
            inner,
            read_state: TrafficState::new(&uuid_bytes),
            write_state: TrafficState::new(&uuid_bytes),
            write_uuid: Some(uuid_bytes),
            read_direct: false,
            write_direct: false,
            raw_buf: vec![0u8; 32768],
            leftover: Vec::new(),
        }
    }
}

struct VisionReadHalf {
    inner: Box<dyn TunnelReader>,
    read_state: TrafficState,
    read_direct: bool,
    raw_buf: Vec<u8>,
    leftover: Vec<u8>,
}

struct VisionWriteHalf {
    inner: Box<dyn TunnelWriter>,
    write_state: TrafficState,
    write_uuid: Option<[u8; 16]>,
    write_direct: bool,
}

impl Read for VisionTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_vision(
            self.inner.as_mut(),
            &mut self.read_state,
            &mut self.read_direct,
            &mut self.raw_buf,
            &mut self.leftover,
            buf,
        )
    }
}

impl Read for VisionReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_vision(
            self.inner.as_mut(),
            &mut self.read_state,
            &mut self.read_direct,
            &mut self.raw_buf,
            &mut self.leftover,
            buf,
        )
    }
}

impl Write for VisionTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_vision(
            self.inner.as_mut(),
            &mut self.write_state,
            &mut self.write_uuid,
            &mut self.write_direct,
            buf,
        )
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Write for VisionWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_vision(
            self.inner.as_mut(),
            &mut self.write_state,
            &mut self.write_uuid,
            &mut self.write_direct,
            buf,
        )
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl TunnelWriter for VisionWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        self.inner.shutdown_write()
    }
}

impl Tunnel for VisionTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Vision tunnel cannot be cloned (TrafficState is single-owner)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let VisionTunnel {
            inner,
            read_state,
            write_state,
            write_uuid,
            read_direct,
            write_direct,
            raw_buf,
            leftover,
        } = *self;
        let (inner_reader, inner_writer) = inner.split_box()?;
        Ok((
            Box::new(VisionReadHalf {
                inner: inner_reader,
                read_state,
                read_direct,
                raw_buf,
                leftover,
            }),
            Box::new(VisionWriteHalf {
                inner: inner_writer,
                write_state,
                write_uuid,
                write_direct,
            }),
        ))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        self.inner.shutdown_write()
    }

    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        self.inner.set_socket_timeouts(read, write)
    }
}

fn read_vision(
    inner: &mut dyn Read,
    read_state: &mut TrafficState,
    read_direct: &mut bool,
    raw_buf: &mut Vec<u8>,
    leftover: &mut Vec<u8>,
    buf: &mut [u8],
) -> io::Result<usize> {
    if buf.is_empty() {
        return Ok(0);
    }

    if !leftover.is_empty() {
        let n = leftover.len().min(buf.len());
        buf[..n].copy_from_slice(&leftover[..n]);
        leftover.drain(..n);
        return Ok(n);
    }

    if raw_buf.len() < buf.len() {
        raw_buf.resize(buf.len(), 0);
    }

    loop {
        let n = inner.read(&mut raw_buf[..buf.len()])?;
        if n == 0 {
            return Ok(0);
        }

        if *read_direct {
            let copy_len = n.min(buf.len());
            buf[..copy_len].copy_from_slice(&raw_buf[..copy_len]);
            return Ok(copy_len);
        }

        let within =
            read_state.outbound.within_padding_buffers || read_state.number_of_packet_to_filter > 0;
        if !within {
            let copy_len = n.min(buf.len());
            buf[..copy_len].copy_from_slice(&raw_buf[..copy_len]);
            return Ok(copy_len);
        }

        let unpadded = xtls_unpadding(&raw_buf[..n], read_state, false);

        {
            let dir = &mut read_state.outbound;
            if dir.remaining_content > 0 || dir.remaining_padding > 0 || dir.current_command == 0 {
                dir.within_padding_buffers = true;
            } else if dir.current_command == 1 {
                dir.within_padding_buffers = false;
            } else if dir.current_command == 2 {
                dir.within_padding_buffers = false;
                dir.direct_copy = true;
                *read_direct = true;
            }
        }

        if read_state.number_of_packet_to_filter > 0 {
            xtls_filter_tls(&unpadded, read_state);
        }

        if unpadded.is_empty() {
            continue;
        }

        let copy_len = unpadded.len().min(buf.len());
        buf[..copy_len].copy_from_slice(&unpadded[..copy_len]);
        if copy_len < unpadded.len() {
            leftover.extend_from_slice(&unpadded[copy_len..]);
        }
        return Ok(copy_len);
    }
}

fn write_vision(
    inner: &mut dyn Write,
    write_state: &mut TrafficState,
    write_uuid: &mut Option<[u8; 16]>,
    write_direct: &mut bool,
    buf: &[u8],
) -> io::Result<usize> {
    if *write_direct {
        return inner.write(buf);
    }
    if buf.is_empty() {
        return Ok(0);
    }

    if write_state.number_of_packet_to_filter > 0 {
        xtls_filter_tls(buf, write_state);
    }

    let is_padding = write_state.outbound.is_padding;
    if !is_padding {
        inner.write_all(buf)?;
        return Ok(buf.len());
    }

    let is_complete = is_complete_record(buf);
    let long_padding = write_state.is_tls;

    if write_state.is_tls && buf.len() >= 6 && buf[..3] == TLS_APP_DATA_START && is_complete {
        let command = if write_state.enable_xtls {
            CMD_PADDING_DIRECT
        } else {
            CMD_PADDING_END
        };
        if write_state.enable_xtls {
            write_state.outbound.direct_copy = true;
            *write_direct = true;
        }
        let frame = xtls_padding(buf, command, write_uuid, false, &DEFAULT_TESTSEED);
        write_state.outbound.is_padding = false;
        inner.write_all(&frame)?;
    } else {
        let frame = xtls_padding(
            buf,
            CMD_PADDING_CONTINUE,
            write_uuid,
            long_padding,
            &DEFAULT_TESTSEED,
        );
        inner.write_all(&frame)?;
    }

    Ok(buf.len())
}
