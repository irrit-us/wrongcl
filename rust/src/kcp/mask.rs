use std::io;

use aes_gcm::aead::{AeadInPlace, KeyInit};
use rand::RngCore;
use sha2::Digest;

use super::MKCP_ORIGINAL_OVERHEAD;

pub(super) enum KcpPacketMask {
    Original,
    Aes128Gcm { key: [u8; 16] },
}

impl KcpPacketMask {
    pub(super) fn from_seed(seed: &str) -> Self {
        if seed.trim().is_empty() {
            Self::Original
        } else {
            let digest = sha2::Sha256::digest(seed.as_bytes());
            let mut key = [0u8; 16];
            key.copy_from_slice(&digest[..16]);
            Self::Aes128Gcm { key }
        }
    }

    pub(super) fn overhead(&self) -> usize {
        match self {
            Self::Original => MKCP_ORIGINAL_OVERHEAD,
            Self::Aes128Gcm { .. } => 16,
        }
    }

    pub(super) fn wrap(&self, plaintext: &[u8]) -> io::Result<Vec<u8>> {
        match self {
            Self::Original => {
                let mut packet = Vec::with_capacity(MKCP_ORIGINAL_OVERHEAD + plaintext.len() + 3);
                packet.extend_from_slice(&[0u8; MKCP_ORIGINAL_OVERHEAD]);
                packet[4..6].copy_from_slice(&(plaintext.len() as u16).to_be_bytes());
                packet.extend_from_slice(plaintext);
                let auth = fnv1a_32(&packet[4..]);
                packet[..4].copy_from_slice(&auth.to_be_bytes());
                let padded_len = if packet.len() % 4 == 0 {
                    packet.len()
                } else {
                    packet.len() + (4 - packet.len() % 4)
                };
                packet.resize(padded_len, 0);
                xorfwd(&mut packet);
                packet.truncate(MKCP_ORIGINAL_OVERHEAD + plaintext.len());
                Ok(packet)
            }
            Self::Aes128Gcm { key } => {
                let cipher = aes_gcm::Aes128Gcm::new_from_slice(key).expect("AES-GCM key length");
                let mut packet = vec![0u8; 12];
                rand::rngs::OsRng.fill_bytes(&mut packet);
                let nonce = aes_gcm::Nonce::from_slice(&packet[..12]);
                let mut ciphertext = plaintext.to_vec();
                let tag = cipher
                    .encrypt_in_place_detached(nonce, b"", &mut ciphertext)
                    .map_err(|e| io::Error::other(format!("mkcp wrap: {e}")))?;
                packet.extend_from_slice(&ciphertext);
                packet.extend_from_slice(tag.as_slice());
                Ok(packet)
            }
        }
    }

    pub(super) fn unwrap(&self, packet: &[u8]) -> Option<Vec<u8>> {
        match self {
            Self::Original => {
                if packet.len() < MKCP_ORIGINAL_OVERHEAD {
                    return None;
                }
                let mut data = packet.to_vec();
                let padded_len = if data.len().is_multiple_of(4) {
                    data.len()
                } else {
                    data.len() + (4 - data.len() % 4)
                };
                data.resize(padded_len, 0);
                xorbkd(&mut data);
                let auth = u32::from_be_bytes(data[..4].try_into().ok()?);
                if fnv1a_32(&data[4..packet.len()]) != auth {
                    return None;
                }
                let length = u16::from_be_bytes(data[4..6].try_into().ok()?) as usize;
                if packet.len().checked_sub(MKCP_ORIGINAL_OVERHEAD)? != length {
                    return None;
                }
                Some(data[6..6 + length].to_vec())
            }
            Self::Aes128Gcm { key } => {
                if packet.len() < 12 + 16 {
                    return None;
                }
                let cipher = aes_gcm::Aes128Gcm::new_from_slice(key).ok()?;
                let nonce = aes_gcm::Nonce::from_slice(&packet[..12]);
                let split = packet.len() - 16;
                let mut plaintext = packet[12..split].to_vec();
                cipher
                    .decrypt_in_place_detached(
                        nonce,
                        b"",
                        &mut plaintext,
                        aes_gcm::Tag::from_slice(&packet[split..]),
                    )
                    .ok()?;
                Some(plaintext)
            }
        }
    }
}

fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn xorfwd(data: &mut [u8]) {
    for i in 4..data.len() {
        data[i] ^= data[i - 4];
    }
}

fn xorbkd(data: &mut [u8]) {
    for i in (4..data.len()).rev() {
        data[i] ^= data[i - 4];
    }
}
