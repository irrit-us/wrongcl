use rand::RngCore;

use super::{NAIVE_PAD_MAX_PAYLOAD, NAIVE_PAD_OPS};

pub(super) enum PadDecodeState {
    Header,
    Payload { remaining: usize, padding: usize },
    Padding(usize),
    Passthrough,
}

pub(super) struct NaivePadDecoder {
    buf: Vec<u8>,
    state: PadDecodeState,
    ops_remaining: usize,
}

impl NaivePadDecoder {
    pub(super) fn new() -> Self {
        Self {
            buf: Vec::new(),
            state: PadDecodeState::Header,
            ops_remaining: NAIVE_PAD_OPS,
        }
    }

    pub(super) fn feed_into(&mut self, input: &[u8], out: &mut Vec<u8>) {
        self.buf.extend_from_slice(input);
        loop {
            let next = match self.state {
                PadDecodeState::Passthrough => {
                    out.extend_from_slice(&self.buf);
                    self.buf.clear();
                    return;
                }
                PadDecodeState::Header => {
                    if self.buf.len() < 3 {
                        return;
                    }
                    let payload_len = u16::from_be_bytes([self.buf[0], self.buf[1]]) as usize;
                    let padding = self.buf[2] as usize;
                    self.buf.drain(..3);
                    PadDecodeState::Payload {
                        remaining: payload_len,
                        padding,
                    }
                }
                PadDecodeState::Payload { remaining, padding } => {
                    let take = remaining.min(self.buf.len());
                    out.extend_from_slice(&self.buf[..take]);
                    self.buf.drain(..take);
                    let remaining = remaining - take;
                    if remaining == 0 {
                        PadDecodeState::Padding(padding)
                    } else {
                        self.state = PadDecodeState::Payload { remaining, padding };
                        return;
                    }
                }
                PadDecodeState::Padding(remaining) => {
                    let take = remaining.min(self.buf.len());
                    self.buf.drain(..take);
                    let remaining = remaining - take;
                    if remaining == 0 {
                        self.ops_remaining = self.ops_remaining.saturating_sub(1);
                        if self.ops_remaining == 0 {
                            PadDecodeState::Passthrough
                        } else {
                            PadDecodeState::Header
                        }
                    } else {
                        self.state = PadDecodeState::Padding(remaining);
                        return;
                    }
                }
            };
            self.state = next;
        }
    }
}

pub(super) struct NaivePadEncoder {
    ops_remaining: usize,
}

impl NaivePadEncoder {
    pub(super) fn new() -> Self {
        Self {
            ops_remaining: NAIVE_PAD_OPS,
        }
    }

    pub(super) fn encode(&mut self, payload: &[u8]) -> Vec<u8> {
        if self.ops_remaining == 0 || payload.is_empty() {
            return payload.to_vec();
        }
        let mut rng = rand::thread_rng();
        let mut out = Vec::with_capacity(payload.len() + 256);
        let mut cursor = 0usize;
        while cursor < payload.len() && self.ops_remaining > 0 {
            let chunk_len = (payload.len() - cursor).min(NAIVE_PAD_MAX_PAYLOAD);
            let pad_len = (rng.next_u32() & 0xff) as u8;
            out.push((chunk_len >> 8) as u8);
            out.push(chunk_len as u8);
            out.push(pad_len);
            out.extend_from_slice(&payload[cursor..cursor + chunk_len]);
            if pad_len > 0 {
                let mut pad = vec![0u8; pad_len as usize];
                rng.fill_bytes(&mut pad);
                out.extend_from_slice(&pad);
            }
            cursor += chunk_len;
            self.ops_remaining -= 1;
        }
        if cursor < payload.len() {
            out.extend_from_slice(&payload[cursor..]);
        }
        out
    }
}

pub(super) fn random_padding_header_value() -> String {
    let mut rng = rand::thread_rng();
    let len = 30 + (rng.next_u32() as usize % 33);
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let digit = b'0' + (rng.next_u32() % 10) as u8;
        out.push(digit as char);
    }
    out
}
