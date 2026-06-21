use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;
use rand::thread_rng;

const MIN_PORT: u16 = 1000;
const MAX_PORT: u16 = 60_999;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VirtualPort {
    number: u16,
    protocol: PortProtocol,
}

pub struct PortPool {
    protocol: PortProtocol,
    ports: Mutex<VecDeque<u16>>,
}

impl PortPool {
    pub fn new(protocol: PortProtocol) -> Self {
        let mut ports: Vec<u16> = (MIN_PORT..MAX_PORT).collect();
        ports.shuffle(&mut thread_rng());
        Self {
            protocol,
            ports: Mutex::new(ports.into_iter().collect()),
        }
    }

    pub fn acquire(&self) -> Result<VirtualPort> {
        let mut guard = self
            .ports
            .lock()
            .map_err(|_| anyhow!("virtual port pool lock is poisoned"))?;
        let number = guard
            .pop_front()
            .ok_or_else(|| anyhow!("virtual port pool is exhausted"))?;
        Ok(VirtualPort {
            number,
            protocol: self.protocol,
        })
    }

    pub fn release(&self, port: VirtualPort) {
        if port.protocol != self.protocol {
            return;
        }
        if let Ok(mut guard) = self.ports.lock() {
            guard.push_back(port.number);
        }
    }
}

impl VirtualPort {
    pub fn number(self) -> u16 {
        self.number
    }
}

impl Display for PortProtocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp => write!(f, "tcp"),
            Self::Udp => write!(f, "udp"),
        }
    }
}

impl Display for VirtualPort {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}]", self.number, self.protocol)
    }
}
