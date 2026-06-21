use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OpCode {
    Close = 0x08,
    Ping = 0x09,
    Pong = 0x0a,
    Binary = 0x02,
}

pub(super) struct WebSocketTunnel {
    inner: Box<dyn Tunnel>,
    read_buf: Vec<u8>,
}

impl WebSocketTunnel {
    pub(super) fn new(inner: Box<dyn Tunnel>) -> Self {
        Self {
            inner,
            read_buf: Vec::new(),
        }
    }
}

impl Tunnel for WebSocketTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(Self {
            inner: self.inner.try_clone_box()?,
            read_buf: Vec::new(),
        }))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        split_cloneable_tunnel(self)
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        let _ = write_ws_frame(self.inner.as_mut(), &[], OpCode::Close, true);
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

impl Read for WebSocketTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.read_buf.is_empty() {
            let n = self.read_buf.len().min(buf.len());
            buf[..n].copy_from_slice(&self.read_buf[..n]);
            self.read_buf.drain(..n);
            return Ok(n);
        }

        loop {
            let (opcode, payload) = read_ws_frame(self.inner.as_mut())?;
            match opcode {
                OpCode::Binary => {
                    let n = payload.len().min(buf.len());
                    buf[..n].copy_from_slice(&payload[..n]);
                    if n < payload.len() {
                        self.read_buf.extend_from_slice(&payload[n..]);
                    }
                    return Ok(n);
                }
                OpCode::Close => return Ok(0),
                OpCode::Ping => {
                    write_ws_frame(self.inner.as_mut(), &payload, OpCode::Pong, true)?;
                }
                OpCode::Pong => {}
            }
        }
    }
}

impl Write for WebSocketTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_ws_frame(self.inner.as_mut(), buf, OpCode::Binary, true)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub(super) fn write_ws_frame(
    stream: &mut dyn Write,
    payload: &[u8],
    opcode: OpCode,
    masked: bool,
) -> io::Result<()> {
    let mut header = Vec::with_capacity(14);
    header.push(0x80 | opcode as u8);
    let mask_bit = if masked { 0x80 } else { 0x00 };
    match payload.len() {
        len if len < 126 => header.push(mask_bit | len as u8),
        len if len <= u16::MAX as usize => {
            header.push(mask_bit | 126);
            header.extend_from_slice(&(len as u16).to_be_bytes());
        }
        len => {
            header.push(mask_bit | 127);
            header.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }

    if masked {
        let mut mask = [0u8; 4];
        rand::thread_rng().fill_bytes(&mut mask);
        header.extend_from_slice(&mask);
        let mut masked_payload = Vec::with_capacity(payload.len());
        for (idx, byte) in payload.iter().enumerate() {
            masked_payload.push(byte ^ mask[idx % 4]);
        }
        stream.write_all(&header)?;
        stream.write_all(&masked_payload)?;
    } else {
        stream.write_all(&header)?;
        stream.write_all(payload)?;
    }
    stream.flush()
}

pub(super) fn read_ws_frame(stream: &mut dyn Read) -> io::Result<(OpCode, Vec<u8>)> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;
    let opcode = match header[0] & 0x0f {
        0x02 => OpCode::Binary,
        0x08 => OpCode::Close,
        0x09 => OpCode::Ping,
        0x0a => OpCode::Pong,
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported WebSocket opcode {other:#x}"),
            ));
        }
    };
    let masked = header[1] & 0x80 != 0;
    let mut len = (header[1] & 0x7f) as u64;
    if len == 126 {
        let mut extended = [0u8; 2];
        stream.read_exact(&mut extended)?;
        len = u16::from_be_bytes(extended) as u64;
    } else if len == 127 {
        let mut extended = [0u8; 8];
        stream.read_exact(&mut extended)?;
        len = u64::from_be_bytes(extended);
    }

    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }

    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload)?;
    if masked {
        for (idx, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[idx % 4];
        }
    }
    Ok((opcode, payload))
}
