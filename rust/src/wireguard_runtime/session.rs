use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use tokio::sync::mpsc;

use super::port_pool::{PortPool, VirtualPort};
use super::tcp::TcpCommand;
use super::udp::UdpCommand;

pub struct TcpSession {
    pub(crate) writer: TcpSessionWriter,
    pub(crate) inbound_rx: mpsc::UnboundedReceiver<Bytes>,
}

#[derive(Clone)]
pub struct TcpSessionWriter {
    pub(crate) state: Arc<TcpSessionState>,
}

pub(crate) struct TcpSessionState {
    pub(crate) port: Mutex<Option<VirtualPort>>,
    pub(crate) commands: mpsc::UnboundedSender<TcpCommand>,
    pub(crate) pool: Arc<PortPool>,
}

pub struct TcpSessionReader {
    pub(crate) inbound_rx: mpsc::UnboundedReceiver<Bytes>,
}

pub struct UdpSession {
    pub(crate) writer: UdpSessionWriter,
    pub(crate) inbound_rx: mpsc::UnboundedReceiver<Bytes>,
}

#[derive(Clone)]
pub struct UdpSessionWriter {
    pub(crate) state: Arc<UdpSessionState>,
}

pub(crate) struct UdpSessionState {
    pub(crate) port: Mutex<Option<VirtualPort>>,
    pub(crate) commands: mpsc::UnboundedSender<UdpCommand>,
    pub(crate) pool: Arc<PortPool>,
}

impl TcpSession {
    pub fn split(self) -> (TcpSessionWriter, TcpSessionReader) {
        (
            self.writer,
            TcpSessionReader {
                inbound_rx: self.inbound_rx,
            },
        )
    }
}

impl TcpSessionWriter {
    pub fn send(&self, data: Bytes) -> Result<()> {
        let Some(port) = self.current_port()? else {
            bail!("TCP session is closed");
        };
        self.state
            .commands
            .send(TcpCommand::Send { port, data })
            .map_err(|_| anyhow!("TCP interface command channel is closed"))
    }

    pub fn shutdown(&self) {
        if let Ok(Some(port)) = self.current_port() {
            let _ = self.state.commands.send(TcpCommand::Close { port });
        }
    }

    fn current_port(&self) -> Result<Option<VirtualPort>> {
        self.state
            .port
            .lock()
            .map(|guard| *guard)
            .map_err(|_| anyhow!("TCP session state lock is poisoned"))
    }
}

impl Drop for TcpSessionState {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.port.lock() {
            if let Some(port) = guard.take() {
                let _ = self.commands.send(TcpCommand::Close { port });
                self.pool.release(port);
            }
        }
    }
}

impl TcpSessionReader {
    pub fn blocking_recv(&mut self) -> Option<Bytes> {
        self.inbound_rx.blocking_recv()
    }
}

impl UdpSession {
    pub fn send(&self, data: Bytes) -> Result<()> {
        self.writer.send(data)
    }

    pub fn try_recv(&mut self) -> Result<Option<Bytes>> {
        match self.inbound_rx.try_recv() {
            Ok(bytes) => Ok(Some(bytes)),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => Ok(None),
        }
    }
}

impl UdpSessionWriter {
    fn send(&self, data: Bytes) -> Result<()> {
        let Some(port) = self.current_port()? else {
            bail!("UDP session is closed");
        };
        self.state
            .commands
            .send(UdpCommand::Send { port, data })
            .map_err(|_| anyhow!("UDP interface command channel is closed"))
    }

    fn current_port(&self) -> Result<Option<VirtualPort>> {
        self.state
            .port
            .lock()
            .map(|guard| *guard)
            .map_err(|_| anyhow!("UDP session state lock is poisoned"))
    }
}

impl Drop for UdpSessionState {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.port.lock() {
            if let Some(port) = guard.take() {
                let _ = self.commands.send(UdpCommand::Close { port });
                self.pool.release(port);
            }
        }
    }
}
