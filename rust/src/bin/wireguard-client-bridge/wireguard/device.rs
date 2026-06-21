use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use bytes::{Bytes, BytesMut};
use smoltcp::phy::{DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use tokio::sync::{mpsc, Notify};
use tracing::error;

#[derive(Clone)]
pub struct DeviceFeed {
    queue: Arc<Mutex<VecDeque<Bytes>>>,
    notify: Arc<Notify>,
}

pub struct ChannelIpDevice {
    max_transmission_unit: usize,
    outbound: mpsc::UnboundedSender<Vec<u8>>,
    queue: Arc<Mutex<VecDeque<Bytes>>>,
    notify: Arc<Notify>,
}

impl ChannelIpDevice {
    pub fn new(
        max_transmission_unit: usize,
        outbound: mpsc::UnboundedSender<Vec<u8>>,
    ) -> (Self, DeviceFeed) {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let notify = Arc::new(Notify::new());
        (
            Self {
                max_transmission_unit,
                outbound,
                queue: Arc::clone(&queue),
                notify: Arc::clone(&notify),
            },
            DeviceFeed { queue, notify },
        )
    }

    pub fn notify(&self) -> Arc<Notify> {
        Arc::clone(&self.notify)
    }
}

impl DeviceFeed {
    pub fn push(&self, packet: Bytes) {
        if let Ok(mut guard) = self.queue.lock() {
            guard.push_back(packet);
            self.notify.notify_one();
        }
    }
}

impl smoltcp::phy::Device for ChannelIpDevice {
    type RxToken<'a>
        = RxToken
    where
        Self: 'a;
    type TxToken<'a>
        = TxToken
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let packet = self.queue.lock().ok()?.pop_front()?;
        Some((
            RxToken {
                buffer: BytesMut::from(packet.as_ref()),
            },
            TxToken {
                outbound: self.outbound.clone(),
            },
        ))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(TxToken {
            outbound: self.outbound.clone(),
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = self.max_transmission_unit;
        caps
    }
}

pub struct RxToken {
    buffer: BytesMut,
}

impl smoltcp::phy::RxToken for RxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        f(&self.buffer)
    }
}

pub struct TxToken {
    outbound: mpsc::UnboundedSender<Vec<u8>>,
}

impl smoltcp::phy::TxToken for TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = vec![0u8; len];
        let result = f(&mut buffer);
        if self.outbound.send(buffer).is_err() {
            error!("failed to enqueue outbound IP packet");
        }
        result
    }
}
