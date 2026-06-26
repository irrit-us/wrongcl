use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::Duration;

use wrongsv_snell::{
    COMMAND_ERROR, COMMAND_TUNNEL, SnellConfig, SnellReader, SnellVersion, SnellWriter,
    encode_connect_header,
};

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::endpoint::SnellOptions;
use crate::error::{ClientError, Result};
use crate::protocol::Target;

pub(crate) fn open_tunnel(
    inner: Box<dyn Tunnel>,
    opts: &SnellOptions,
    target: &Target,
) -> Result<Box<dyn Tunnel>> {
    let config = SnellConfig::new(opts.psk.as_bytes().to_vec(), opts.version)
        .map_err(|e| ClientError::Config(format!("Snell: {e}")))?;
    let version = SnellVersion::parse(opts.version)
        .map_err(|e| ClientError::Config(format!("Snell: {e}")))?;
    let (read_inner, write_inner) = inner.split_box()?;
    let mut writer = SnellWriter::new(write_inner, &config)
        .map_err(|e| ClientError::Config(format!("Snell writer: {e}")))?;
    let header = encode_connect_header(&target.host, target.port, version)
        .map_err(|e| ClientError::Config(format!("Snell header: {e}")))?;
    writer
        .write_chunk(&header)
        .map_err(|e| ClientError::Config(format!("Snell request: {e}")))?;

    let mut reader = SnellReader::new(read_inner, &config)
        .map_err(|e| ClientError::Config(format!("Snell reader: {e}")))?;
    let response = reader
        .read_chunk()
        .map_err(|e| ClientError::Config(format!("Snell response: {e}")))?;
    let Some((&command, rest)) = response.split_first() else {
        return Err(ClientError::Config(
            "Snell server returned an empty response".into(),
        ));
    };
    match command {
        COMMAND_TUNNEL => Ok(Box::new(SnellTunnel {
            reader,
            writer: Some(writer),
            residual: rest.iter().copied().collect(),
        })),
        COMMAND_ERROR => Err(ClientError::Config(format!(
            "Snell server rejected CONNECT: {}",
            decode_error_message(rest)
        ))),
        other => Err(ClientError::Config(format!(
            "Snell server returned unsupported response command {other}"
        ))),
    }
}

struct SnellTunnel {
    reader: SnellReader<Box<dyn TunnelReader>>,
    writer: Option<SnellWriter<Box<dyn TunnelWriter>>>,
    residual: VecDeque<u8>,
}

struct SnellReadHalf {
    reader: SnellReader<Box<dyn TunnelReader>>,
    residual: VecDeque<u8>,
}

struct SnellWriteHalf {
    writer: Option<SnellWriter<Box<dyn TunnelWriter>>>,
}

fn read_snell(
    reader: &mut SnellReader<Box<dyn TunnelReader>>,
    residual: &mut VecDeque<u8>,
    buf: &mut [u8],
) -> io::Result<usize> {
    if !residual.is_empty() {
        let n = residual.len().min(buf.len());
        for slot in buf.iter_mut().take(n) {
            *slot = residual.pop_front().expect("residual non-empty");
        }
        return Ok(n);
    }
    let chunk = match reader.read_chunk() {
        Ok(chunk) => chunk,
        Err(e) => {
            let io_err = io_from_snell(e);
            if io_err.kind() == io::ErrorKind::UnexpectedEof {
                return Ok(0);
            }
            return Err(io_err);
        }
    };
    if chunk.is_empty() {
        return Ok(0);
    }
    let n = chunk.len().min(buf.len());
    buf[..n].copy_from_slice(&chunk[..n]);
    if n < chunk.len() {
        residual.extend(&chunk[n..]);
    }
    Ok(n)
}

fn write_snell(
    writer: &mut Option<SnellWriter<Box<dyn TunnelWriter>>>,
    buf: &[u8],
) -> io::Result<usize> {
    if buf.is_empty() {
        return Ok(0);
    }
    let writer = writer
        .as_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "writer closed"))?;
    writer.write_chunk(buf).map_err(io_from_snell)?;
    Ok(buf.len())
}

impl Read for SnellTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_snell(&mut self.reader, &mut self.residual, buf)
    }
}

impl Read for SnellReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_snell(&mut self.reader, &mut self.residual, buf)
    }
}

impl Write for SnellTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_snell(&mut self.writer, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for SnellWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_snell(&mut self.writer, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for SnellWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer.get_mut().shutdown_write()
        } else {
            Ok(())
        }
    }
}

impl Tunnel for SnellTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Snell tunnel cannot be cloned (cipher state is single-threaded)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let SnellTunnel {
            reader,
            writer,
            residual,
        } = *self;
        Ok((
            Box::new(SnellReadHalf { reader, residual }),
            Box::new(SnellWriteHalf { writer }),
        ))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer.get_mut().shutdown_write()
        } else {
            Ok(())
        }
    }

    fn set_socket_timeouts(
        &self,
        _read: Option<Duration>,
        _write: Option<Duration>,
    ) -> io::Result<()> {
        Ok(())
    }
}

fn decode_error_message(data: &[u8]) -> String {
    if data.len() < 2 {
        return "malformed error response".into();
    }
    let len = data[1] as usize;
    let message = data.get(2..2 + len).unwrap_or_default();
    String::from_utf8_lossy(message).into_owned()
}

fn io_from_snell(e: wrongsv_snell::SnellError) -> io::Error {
    match e {
        wrongsv_snell::SnellError::Io(err) => err,
        other => io::Error::new(io::ErrorKind::InvalidData, other.to_string()),
    }
}
