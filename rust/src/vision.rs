use std::io::{self, Read, Write};
use std::time::Duration;

use uuid::Uuid;
use wrongsv_vless::vision::{
    is_complete_record, xtls_filter_tls, xtls_padding, xtls_unpadding, TrafficState,
    CMD_PADDING_CONTINUE, CMD_PADDING_DIRECT, CMD_PADDING_END,
};

use crate::client::Tunnel;
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

impl Read for VisionTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if !self.leftover.is_empty() {
            let n = self.leftover.len().min(buf.len());
            buf[..n].copy_from_slice(&self.leftover[..n]);
            self.leftover.drain(..n);
            return Ok(n);
        }

        if self.raw_buf.len() < buf.len() {
            self.raw_buf.resize(buf.len(), 0);
        }

        loop {
            let n = self.inner.read(&mut self.raw_buf[..buf.len()])?;
            if n == 0 {
                return Ok(0);
            }

            if self.read_direct {
                let copy_len = n.min(buf.len());
                buf[..copy_len].copy_from_slice(&self.raw_buf[..copy_len]);
                return Ok(copy_len);
            }

            let within = self.read_state.outbound.within_padding_buffers
                || self.read_state.number_of_packet_to_filter > 0;
            if !within {
                let copy_len = n.min(buf.len());
                buf[..copy_len].copy_from_slice(&self.raw_buf[..copy_len]);
                return Ok(copy_len);
            }

            let unpadded = xtls_unpadding(&self.raw_buf[..n], &mut self.read_state, false);

            {
                let dir = &mut self.read_state.outbound;
                if dir.remaining_content > 0
                    || dir.remaining_padding > 0
                    || dir.current_command == 0
                {
                    dir.within_padding_buffers = true;
                } else if dir.current_command == 1 {
                    dir.within_padding_buffers = false;
                } else if dir.current_command == 2 {
                    dir.within_padding_buffers = false;
                    dir.direct_copy = true;
                    self.read_direct = true;
                }
            }

            if self.read_state.number_of_packet_to_filter > 0 {
                xtls_filter_tls(&unpadded, &mut self.read_state);
            }

            if unpadded.is_empty() {
                continue;
            }

            let copy_len = unpadded.len().min(buf.len());
            buf[..copy_len].copy_from_slice(&unpadded[..copy_len]);
            if copy_len < unpadded.len() {
                self.leftover.extend_from_slice(&unpadded[copy_len..]);
            }
            return Ok(copy_len);
        }
    }
}

impl Write for VisionTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.write_direct {
            return self.inner.write(buf);
        }
        if buf.is_empty() {
            return Ok(0);
        }

        if self.write_state.number_of_packet_to_filter > 0 {
            xtls_filter_tls(buf, &mut self.write_state);
        }

        let is_padding = self.write_state.outbound.is_padding;
        if !is_padding {
            self.inner.write_all(buf)?;
            return Ok(buf.len());
        }

        let is_complete = is_complete_record(buf);
        let long_padding = self.write_state.is_tls;

        if self.write_state.is_tls
            && buf.len() >= 6
            && buf[..3] == TLS_APP_DATA_START
            && is_complete
        {
            let command = if self.write_state.enable_xtls {
                CMD_PADDING_DIRECT
            } else {
                CMD_PADDING_END
            };
            if self.write_state.enable_xtls {
                self.write_state.outbound.direct_copy = true;
                self.write_direct = true;
            }
            let frame = xtls_padding(buf, command, &mut self.write_uuid, false, &DEFAULT_TESTSEED);
            self.write_state.outbound.is_padding = false;
            self.inner.write_all(&frame)?;
        } else {
            let frame = xtls_padding(
                buf,
                CMD_PADDING_CONTINUE,
                &mut self.write_uuid,
                long_padding,
                &DEFAULT_TESTSEED,
            );
            self.inner.write_all(&frame)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Tunnel for VisionTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Vision tunnel cannot be cloned (TrafficState is single-owner)",
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
