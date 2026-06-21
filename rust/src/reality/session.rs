use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

use aes_gcm::aead::Aead;
use aes_gcm::{Aes128Gcm, Key, KeyInit, Nonce};
use hkdf::Hkdf;
use hmac::Mac;
use sha2::Sha256;

use crate::client::{Tunnel, TunnelReader, TunnelWriter};

pub(super) fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let mut mac = <hmac::Hmac<Sha256> as Mac>::new_from_slice(salt).expect("HMAC key len");
    mac.update(ikm);
    mac.finalize().into_bytes().into()
}

pub(super) fn hkdf_expand_label(
    secret: &[u8],
    label: &str,
    context: &[u8],
    length: usize,
) -> Vec<u8> {
    let hkdf = Hkdf::<Sha256>::from_prk(secret).expect("valid prk");
    let mut out = vec![0u8; length];
    let full_label = format!("tls13 {label}");
    let mut info = Vec::new();
    info.extend_from_slice(&(length as u16).to_be_bytes());
    info.push(full_label.len() as u8);
    info.extend_from_slice(full_label.as_bytes());
    info.push(context.len() as u8);
    info.extend_from_slice(context);
    hkdf.expand(&info, &mut out).expect("expand ok");
    out
}

pub(super) fn iv_from(slice: &[u8]) -> [u8; 12] {
    let mut iv = [0u8; 12];
    iv.copy_from_slice(slice);
    iv
}

fn tls13_nonce(iv: &[u8; 12], seq: u64) -> [u8; 12] {
    let mut n = *iv;
    let be = seq.to_be_bytes();
    for i in 0..8 {
        n[4 + i] ^= be[i];
    }
    n
}

pub(super) struct AeadState {
    cipher: Aes128Gcm,
    iv: [u8; 12],
    seq: u64,
}

impl AeadState {
    pub(super) fn new(key: &[u8], iv: &[u8; 12]) -> Self {
        Self {
            cipher: Aes128Gcm::new(Key::<Aes128Gcm>::from_slice(key)),
            iv: *iv,
            seq: 0,
        }
    }

    pub(super) fn decrypt(&mut self, payload: &[u8], aad: &[u8; 5]) -> io::Result<Vec<u8>> {
        let nonce_arr = tls13_nonce(&self.iv, self.seq);
        let mut pt = self
            .cipher
            .decrypt(
                Nonce::from_slice(&nonce_arr),
                aes_gcm::aead::Payload {
                    msg: payload,
                    aad: aad.as_slice(),
                },
            )
            .map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("AEAD decrypt: {e}"))
            })?;
        self.seq += 1;
        while pt.last() == Some(&0) {
            pt.pop();
        }
        pt.pop();
        Ok(pt)
    }

    pub(super) fn encrypt(
        &mut self,
        plaintext: &[u8],
        record_type: u8,
        inner_ct: u8,
    ) -> io::Result<Vec<u8>> {
        let nonce_arr = tls13_nonce(&self.iv, self.seq);
        let mut inner = plaintext.to_vec();
        inner.push(inner_ct);
        let record_len = inner.len() + 16;
        let hdr: [u8; 5] = [
            record_type,
            0x03,
            0x03,
            (record_len >> 8) as u8,
            record_len as u8,
        ];
        let ct = self
            .cipher
            .encrypt(
                Nonce::from_slice(&nonce_arr),
                aes_gcm::aead::Payload {
                    msg: &inner,
                    aad: hdr.as_slice(),
                },
            )
            .map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("AEAD encrypt: {e}"))
            })?;
        self.seq += 1;
        let mut out = Vec::with_capacity(5 + ct.len());
        out.extend_from_slice(&hdr);
        out.extend_from_slice(&ct);
        Ok(out)
    }
}

pub(super) struct RealityTunnel {
    pub(super) sock: TcpStream,
    pub(super) encrypt: AeadState,
    pub(super) decrypt: AeadState,
    pub(super) residual: Vec<u8>,
}

struct RealityReadHalf {
    sock: TcpStream,
    decrypt: AeadState,
    residual: Vec<u8>,
}

struct RealityWriteHalf {
    sock: TcpStream,
    encrypt: AeadState,
}

impl Read for RealityTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_reality(&mut self.sock, &mut self.decrypt, &mut self.residual, buf)
    }
}

impl Read for RealityReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_reality(&mut self.sock, &mut self.decrypt, &mut self.residual, buf)
    }
}

fn read_reality(
    sock: &mut TcpStream,
    decrypt: &mut AeadState,
    residual: &mut Vec<u8>,
    buf: &mut [u8],
) -> io::Result<usize> {
    if !residual.is_empty() {
        let n = residual.len().min(buf.len());
        buf[..n].copy_from_slice(&residual[..n]);
        residual.drain(..n);
        return Ok(n);
    }
    loop {
        let (ct, payload, hdr) = super::read_tls_record(sock)?;
        match ct {
            0x17 => {
                let pt = decrypt.decrypt(&payload, &hdr)?;
                let n = pt.len().min(buf.len());
                buf[..n].copy_from_slice(&pt[..n]);
                if n < pt.len() {
                    residual.extend_from_slice(&pt[n..]);
                }
                return Ok(n);
            }
            0x15 => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "TLS alert",
                ))
            }
            0x14 => continue,
            _ => continue,
        }
    }
}

impl Write for RealityTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_reality(&mut self.sock, &mut self.encrypt, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sock.flush()
    }
}

impl Write for RealityWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_reality(&mut self.sock, &mut self.encrypt, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sock.flush()
    }
}

fn write_reality(sock: &mut TcpStream, encrypt: &mut AeadState, buf: &[u8]) -> io::Result<usize> {
    const MAX_CHUNK: usize = 16384;
    let mut written = 0;
    while written < buf.len() {
        let end = (written + MAX_CHUNK).min(buf.len());
        let record = encrypt.encrypt(&buf[written..end], 0x17, 0x17)?;
        sock.write_all(&record)?;
        written = end;
    }
    sock.flush()?;
    Ok(buf.len())
}

impl TunnelWriter for RealityWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        self.sock.shutdown(Shutdown::Write)
    }
}

impl Tunnel for RealityTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "REALITY tunnel cannot be cloned (AEAD seq counters are single-owner)",
        ))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let RealityTunnel {
            sock,
            encrypt,
            decrypt,
            residual,
        } = *self;
        let read_sock = sock.try_clone()?;
        Ok((
            Box::new(RealityReadHalf {
                sock: read_sock,
                decrypt,
                residual,
            }),
            Box::new(RealityWriteHalf { sock, encrypt }),
        ))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        self.sock.shutdown(Shutdown::Write)
    }

    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        self.sock.set_read_timeout(read)?;
        self.sock.set_write_timeout(write)?;
        Ok(())
    }
}
