use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::Duration;

use wrongsv_net_types::{Address, Port};
use wrongsv_shadowsocks::{ServerConfig, ShadowsocksReader, ShadowsocksWriter};

use crate::client::Tunnel;
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
    let read_inner = inner.try_clone_box()?;
    let address = address_from_host(&target.host);
    let port = Port(target.port);
    let (writer, request_salt) =
        ShadowsocksWriter::new_request(inner, &config, &address, port, b"")
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
        inner: Option<Box<dyn Tunnel>>,
        config: ServerConfig,
        request_salt: Vec<u8>,
    },
    Active(ShadowsocksReader<Box<dyn Tunnel>>),
}

struct ShadowsocksTunnel {
    reader: ReaderState,
    writer: Option<ShadowsocksWriter<Box<dyn Tunnel>>>,
    residual: VecDeque<u8>,
}

impl ShadowsocksTunnel {
    fn activate_reader(&mut self) -> io::Result<&mut ShadowsocksReader<Box<dyn Tunnel>>> {
        if let ReaderState::Pending {
            inner,
            config,
            request_salt,
        } = &mut self.reader
        {
            let stream = inner.take().expect("pending reader inner");
            let salt = if request_salt.is_empty() {
                None
            } else {
                Some(request_salt.as_slice())
            };
            let reader = ShadowsocksReader::new_response(stream, config, salt)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            self.reader = ReaderState::Active(reader);
        }
        match &mut self.reader {
            ReaderState::Active(reader) => Ok(reader),
            ReaderState::Pending { .. } => unreachable!("activated above"),
        }
    }
}

impl Read for ShadowsocksTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.residual.is_empty() {
            let n = self.residual.len().min(buf.len());
            for slot in buf.iter_mut().take(n) {
                *slot = self.residual.pop_front().expect("residual non-empty");
            }
            return Ok(n);
        }
        let reader = self.activate_reader()?;
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
            self.residual.extend(&chunk[n..]);
        }
        Ok(n)
    }
}

impl Write for ShadowsocksTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "writer closed"))?;
        writer
            .write_chunk(buf)
            .map_err(io_from_ss)
            .map(|()| buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for ShadowsocksTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Shadowsocks tunnel cannot be cloned (cipher state is single-threaded)",
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
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        if let ReaderState::Pending { inner, .. } = &self.reader {
            if let Some(inner) = inner.as_deref() {
                inner.set_socket_timeouts(read, write)?;
            }
        }
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
