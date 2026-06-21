use std::io::{self, Read, Write};
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::client::{Tunnel, TunnelReader, TunnelWriter};

pub(super) struct NaiveTunnel {
    pub(super) read_rx: Receiver<Vec<u8>>,
    pub(super) write_tx: SyncSender<Vec<u8>>,
    pub(super) read_buf: Vec<u8>,
    pub(super) eof: bool,
    pub(super) _handle: JoinHandle<()>,
}

struct NaiveReadHalf {
    read_rx: Receiver<Vec<u8>>,
    read_buf: Vec<u8>,
    eof: bool,
    _handle: JoinHandle<()>,
}

#[derive(Clone)]
struct NaiveWriteHalf {
    write_tx: SyncSender<Vec<u8>>,
}

impl Read for NaiveTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for NaiveTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Naive write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for NaiveReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_channel(&self.read_rx, &mut self.read_buf, &mut self.eof, buf)
    }
}

impl Write for NaiveWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.write_tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Naive write channel closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TunnelWriter for NaiveWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Tunnel for NaiveTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Naive tunnel cannot be cloned (single h2 CONNECT stream)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let Self {
            read_rx,
            write_tx,
            read_buf,
            eof,
            _handle,
        } = *self;
        Ok((
            Box::new(NaiveReadHalf {
                read_rx,
                read_buf,
                eof,
                _handle,
            }),
            Box::new(NaiveWriteHalf { write_tx }),
        ))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn set_socket_timeouts(
        &self,
        _read: Option<Duration>,
        _write: Option<Duration>,
    ) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn read_channel(
    read_rx: &Receiver<Vec<u8>>,
    read_buf: &mut Vec<u8>,
    eof: &mut bool,
    buf: &mut [u8],
) -> io::Result<usize> {
    if !read_buf.is_empty() {
        let n = read_buf.len().min(buf.len());
        buf[..n].copy_from_slice(&read_buf[..n]);
        read_buf.drain(..n);
        return Ok(n);
    }
    if *eof {
        return Ok(0);
    }
    let data = match read_rx.recv() {
        Ok(data) => data,
        Err(_) => {
            *eof = true;
            return Ok(0);
        }
    };
    if data.is_empty() {
        *eof = true;
        return Ok(0);
    }
    let n = data.len().min(buf.len());
    buf[..n].copy_from_slice(&data[..n]);
    if n < data.len() {
        read_buf.extend_from_slice(&data[n..]);
    }
    Ok(n)
}
