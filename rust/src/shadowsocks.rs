use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::Duration;

use wrongsv_net_types::{Address, Port};
use wrongsv_shadowsocks::{ServerConfig, ShadowsocksReader, ShadowsocksWriter};

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::endpoint::ShadowsocksOptions;
use crate::error::{ClientError, Result};
use crate::protocol::Target;

pub(crate) fn open_tunnel(
    inner: Box<dyn Tunnel>,
    opts: &ShadowsocksOptions,
    target: &Target,
) -> Result<Box<dyn Tunnel>> {
    let config = ServerConfig::new(&opts.method, opts.password.clone())
        .map_err(|e| ClientError::Config(format!("Shadowsocks: {e}")))?;
    let (read_inner, write_inner) = inner.split_box()?;
    let address = address_from_host(&target.host);
    let port = Port(target.port);
    let (writer, request_salt) =
        ShadowsocksWriter::new_request(write_inner, &config, &address, port, b"")
            .map_err(|e| ClientError::Config(format!("Shadowsocks request: {e}")))?;
    Ok(Box::new(ShadowsocksTunnel {
        reader: ReaderState::Pending {
            inner: Some(read_inner),
            config,
            request_salt,
        },
        writer: Some(writer),
        residual: VecDeque::new(),
    }))
}

fn address_from_host(host: &str) -> Address {
    Address::parse(host)
}

enum ReaderState {
    Pending {
        inner: Option<Box<dyn TunnelReader>>,
        config: ServerConfig,
        request_salt: Vec<u8>,
    },
    Active(ShadowsocksReader<Box<dyn TunnelReader>>),
}

struct ShadowsocksTunnel {
    reader: ReaderState,
    writer: Option<ShadowsocksWriter<Box<dyn TunnelWriter>>>,
    residual: VecDeque<u8>,
}

struct ShadowsocksReadHalf {
    reader: ReaderState,
    residual: VecDeque<u8>,
}

struct ShadowsocksWriteHalf {
    writer: Option<ShadowsocksWriter<Box<dyn TunnelWriter>>>,
}

fn activate_reader(
    reader_state: &mut ReaderState,
) -> io::Result<&mut ShadowsocksReader<Box<dyn TunnelReader>>> {
    if let ReaderState::Pending {
        inner,
        config,
        request_salt,
    } = reader_state
    {
        let stream = inner.take().expect("pending reader inner");
        let salt = if request_salt.is_empty() {
            None
        } else {
            Some(request_salt.as_slice())
        };
        let reader = ShadowsocksReader::new_response(stream, config, salt)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        *reader_state = ReaderState::Active(reader);
    }
    match reader_state {
        ReaderState::Active(reader) => Ok(reader),
        ReaderState::Pending { .. } => unreachable!("activated above"),
    }
}

fn read_shadowsocks(
    reader_state: &mut ReaderState,
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
    let reader = activate_reader(reader_state)?;
    let chunk = match reader.read_chunk() {
        Ok(chunk) => chunk,
        Err(e) => {
            let io_err = io_from_ss(e);
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

fn write_shadowsocks(
    writer: &mut Option<ShadowsocksWriter<Box<dyn TunnelWriter>>>,
    buf: &[u8],
) -> io::Result<usize> {
    if buf.is_empty() {
        return Ok(0);
    }
    let writer = writer
        .as_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "writer closed"))?;
    writer
        .write_chunk(buf)
        .map_err(io_from_ss)
        .map(|()| buf.len())
}

impl Read for ShadowsocksTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_shadowsocks(&mut self.reader, &mut self.residual, buf)
    }
}

impl Read for ShadowsocksReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_shadowsocks(&mut self.reader, &mut self.residual, buf)
    }
}

impl Write for ShadowsocksTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_shadowsocks(&mut self.writer, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for ShadowsocksWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_shadowsocks(&mut self.writer, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for ShadowsocksWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer.get_mut().shutdown_write()
        } else {
            Ok(())
        }
    }
}

impl Tunnel for ShadowsocksTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Shadowsocks tunnel cannot be cloned (cipher state is single-threaded)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let ShadowsocksTunnel {
            reader,
            writer,
            residual,
        } = *self;
        Ok((
            Box::new(ShadowsocksReadHalf { reader, residual }),
            Box::new(ShadowsocksWriteHalf { writer }),
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

fn io_from_ss(e: wrongsv_shadowsocks::ShadowsocksError) -> io::Error {
    use wrongsv_shadowsocks::ShadowsocksError;
    match e {
        ShadowsocksError::Io(err) => err,
        other => io::Error::new(io::ErrorKind::InvalidData, other.to_string()),
    }
}
