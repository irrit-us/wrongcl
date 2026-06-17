//! REALITY outer security: TLS 1.3 with REALITY auth, then VLESS over the
//! application stream. Borrows REALITY primitives from `wrongsv-reality`;
//! the TLS 1.3 record framing is an inline RFC 8446 implementation.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aes_gcm::aead::Aead;
use aes_gcm::{Aes128Gcm, Key, KeyInit, Nonce};
use base64::Engine as _;
use hkdf::Hkdf;
use hmac::Mac;
use rand::RngCore;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::endpoint::RealityOptions;
use crate::error::{ClientError, Result};

pub fn wrap(socket: TcpStream, opts: &RealityOptions) -> Result<Box<dyn Tunnel>> {
    let server_pk_bytes = decode_server_pubkey(&opts.public_key)?;
    let short_id = decode_short_id(&opts.short_id)?;
    let raw_pubkey = decode_raw_pubkey(&opts.raw_pubkey)?;
    let conn = handshake(
        socket,
        &opts.server_name,
        server_pk_bytes,
        short_id,
        raw_pubkey,
    )
    .map_err(ClientError::Io)?;
    Ok(Box::new(conn))
}

fn decode_server_pubkey(b64: &str) -> Result<[u8; 32]> {
    let mut padded = b64.trim().to_string();
    while !padded.len().is_multiple_of(4) {
        padded.push('=');
    }
    let bytes = base64::engine::general_purpose::URL_SAFE
        .decode(&padded)
        .map_err(|e| ClientError::Config(format!("REALITY public-key base64: {e}")))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ClientError::Config("REALITY public-key must decode to 32 bytes".into()))?;
    if bytes == [0u8; 32] {
        return Err(ClientError::Config(
            "REALITY public-key is the identity element".into(),
        ));
    }
    Ok(bytes)
}

fn decode_short_id(hex: &str) -> Result<[u8; 4]> {
    let hex = hex.trim();
    if hex.len() != 8 {
        return Err(ClientError::Config(
            "REALITY short-id must be 8 hex chars".into(),
        ));
    }
    let mut out = [0u8; 4];
    for i in 0..4 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| ClientError::Config(format!("REALITY short-id hex: {e}")))?;
    }
    Ok(out)
}

fn decode_raw_pubkey(hex: &str) -> Result<Option<[u8; 32]>> {
    let hex = hex.trim();
    if hex.is_empty() {
        return Ok(None);
    }
    if hex.len() != 64 {
        return Err(ClientError::Config(
            "REALITY raw-pubkey must be 64 hex chars (or empty to skip cert verification)".into(),
        ));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| ClientError::Config(format!("REALITY raw-pubkey hex: {e}")))?;
    }
    Ok(Some(out))
}

fn handshake(
    mut sock: TcpStream,
    sni: &str,
    server_pk_bytes: [u8; 32],
    short_id: [u8; 4],
    raw_pubkey: Option<[u8; 32]>,
) -> io::Result<RealityTunnel> {
    sock.set_read_timeout(Some(Duration::from_secs(10)))?;
    sock.set_write_timeout(Some(Duration::from_secs(10)))?;

    let client_sk = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let client_pk = PublicKey::from(&client_sk);
    let server_pk = PublicKey::from(server_pk_bytes);
    let reality_shared = client_sk.diffie_hellman(&server_pk);

    let mut client_random = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut client_random);

    let auth_key =
        wrongsv_reality::derive_client_auth_key(&client_random, reality_shared.as_bytes())
            .map_err(|e| io::Error::other(format!("REALITY derive auth key: {e}")))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("system time: {e}")))?
        .as_secs() as u32;

    let temp_hello = wrongsv_reality::build_reality_client_hello(
        client_random,
        [0u8; 32],
        *client_pk.as_bytes(),
        sni,
    );
    let aad = &temp_hello[5..];
    let session_id =
        wrongsv_reality::build_session_id(&auth_key, &client_random, timestamp, &short_id, aad)
            .map_err(|e| io::Error::other(format!("REALITY session id: {e}")))?;
    let client_hello = wrongsv_reality::build_reality_client_hello(
        client_random,
        session_id,
        *client_pk.as_bytes(),
        sni,
    );
    let client_hello_body = client_hello[5..].to_vec();
    sock.write_all(&client_hello)?;

    let (ct_type, server_hello_payload, _) = read_tls_record(&mut sock)?;
    if ct_type != 0x16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected handshake record, got 0x{ct_type:02x}"),
        ));
    }
    let (_server_random, server_key_share) = parse_server_hello(&server_hello_payload)?;
    if server_key_share == [0u8; 32] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "TLS: server key_share is identity element",
        ));
    }
    let server_ks_pk = PublicKey::from(server_key_share);
    let tls_shared = client_sk.diffie_hellman(&server_ks_pk);

    let empty_hash = Sha256::digest([]);
    let early_secret = hkdf_extract(&[0u8; 32], &[0u8; 32]);
    let derived = hkdf_expand_label(&early_secret, "derived", &empty_hash, 32);
    let handshake_secret = hkdf_extract(&derived, tls_shared.as_bytes());

    let mut transcript = Vec::new();
    transcript.extend_from_slice(&client_hello_body);
    transcript.extend_from_slice(&server_hello_payload);
    let transcript_hash = Sha256::digest(&transcript);

    let client_hs_ts = hkdf_expand_label(&handshake_secret, "c hs traffic", &transcript_hash, 32);
    let server_hs_ts = hkdf_expand_label(&handshake_secret, "s hs traffic", &transcript_hash, 32);

    let client_hs_key = hkdf_expand_label(&client_hs_ts, "key", b"", 16);
    let client_hs_iv = iv_from(&hkdf_expand_label(&client_hs_ts, "iv", b"", 12));
    let server_hs_key = hkdf_expand_label(&server_hs_ts, "key", b"", 16);
    let server_hs_iv = iv_from(&hkdf_expand_label(&server_hs_ts, "iv", b"", 12));

    let mut hs_decrypt = AeadState::new(&server_hs_key, &server_hs_iv);
    let mut hs_encrypt = AeadState::new(&client_hs_key, &client_hs_iv);

    let mut cert_der = Vec::new();
    let mut got_finished = false;
    let mut handshake_data: Vec<u8> = Vec::new();

    while !got_finished {
        let (ct, payload, hdr) = read_tls_record(&mut sock)?;
        match ct {
            0x14 => continue,
            0x17 => {
                let pt = hs_decrypt.decrypt(&payload, &hdr)?;
                let mut pos = 0;
                while pos + 4 <= pt.len() {
                    let msg_type = pt[pos];
                    let msg_len =
                        u32::from_be_bytes([0, pt[pos + 1], pt[pos + 2], pt[pos + 3]]) as usize;
                    pos += 4;
                    if pos + msg_len > pt.len() {
                        break;
                    }
                    let msg = &pt[pos..pos + msg_len];
                    handshake_data.extend_from_slice(&pt[pos - 4..pos + msg_len]);
                    pos += msg_len;
                    if msg_type == 0x0b && !msg.is_empty() {
                        cert_der = extract_cert_der(msg).unwrap_or_default();
                    } else if msg_type == 0x14 {
                        got_finished = true;
                    }
                }
            }
            0x15 => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "TLS alert during handshake",
                ));
            }
            _ => {}
        }
    }
    transcript.extend_from_slice(&handshake_data);

    if let Some(raw) = raw_pubkey {
        verify_reality_cert(&auth_key, &raw, &cert_der)?;
    }

    let finished_key = hkdf_expand_label(&client_hs_ts, "finished", b"", 32);
    let full_transcript_hash = Sha256::digest(&transcript);
    let mut hmac = <hmac::Hmac<Sha256> as Mac>::new_from_slice(&finished_key)
        .map_err(|e| io::Error::other(format!("finished hmac: {e}")))?;
    hmac.update(&full_transcript_hash);
    let verify_data = hmac.finalize().into_bytes();
    let mut finished_msg = vec![0x14u8];
    finished_msg.extend_from_slice(&(verify_data.len() as u32).to_be_bytes()[1..]);
    finished_msg.extend_from_slice(&verify_data);
    let finished_record = hs_encrypt.encrypt(&finished_msg, 0x17, 0x16)?;
    sock.write_all(&finished_record)?;

    let app_transcript_hash = Sha256::digest(&transcript);
    let derived = hkdf_expand_label(&handshake_secret, "derived", &empty_hash, 32);
    let master_secret = hkdf_extract(&derived, &[0u8; 32]);
    let client_app_ts = hkdf_expand_label(&master_secret, "c ap traffic", &app_transcript_hash, 32);
    let server_app_ts = hkdf_expand_label(&master_secret, "s ap traffic", &app_transcript_hash, 32);
    let client_app_key = hkdf_expand_label(&client_app_ts, "key", b"", 16);
    let client_app_iv = iv_from(&hkdf_expand_label(&client_app_ts, "iv", b"", 12));
    let server_app_key = hkdf_expand_label(&server_app_ts, "key", b"", 16);
    let server_app_iv = iv_from(&hkdf_expand_label(&server_app_ts, "iv", b"", 12));

    Ok(RealityTunnel {
        sock,
        encrypt: AeadState::new(&client_app_key, &client_app_iv),
        decrypt: AeadState::new(&server_app_key, &server_app_iv),
        residual: Vec::new(),
    })
}

fn iv_from(slice: &[u8]) -> [u8; 12] {
    let mut iv = [0u8; 12];
    iv.copy_from_slice(slice);
    iv
}

fn extract_cert_der(msg: &[u8]) -> Option<Vec<u8>> {
    // RFC 8446 §4.4.2: context<0..255> + cert_list<0..2^24-1> + first CertificateEntry.
    let ctx_len = *msg.first()? as usize;
    let list_start = 1 + ctx_len;
    let entry_start = list_start + 3;
    if msg.len() < entry_start + 3 {
        return None;
    }
    let cert_len = u32::from_be_bytes([
        0,
        msg[entry_start],
        msg[entry_start + 1],
        msg[entry_start + 2],
    ]) as usize;
    if msg.len() < entry_start + 3 + cert_len {
        return None;
    }
    Some(msg[entry_start + 3..entry_start + 3 + cert_len].to_vec())
}

fn verify_reality_cert(auth_key: &[u8], raw_pubkey: &[u8; 32], cert_der: &[u8]) -> io::Result<()> {
    if cert_der.len() < 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "REALITY cert too short for HMAC tag",
        ));
    }
    let sig = &cert_der[cert_der.len() - 64..];
    let expected = wrongsv_reality::compute_cert_hmac(auth_key, raw_pubkey)
        .map_err(|e| io::Error::other(format!("REALITY cert hmac: {e}")))?;
    if sig == expected.as_slice() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "REALITY cert HMAC mismatch",
        ))
    }
}

fn read_tls_record(stream: &mut TcpStream) -> io::Result<(u8, Vec<u8>, [u8; 5])> {
    let mut hdr = [0u8; 5];
    stream.read_exact(&mut hdr)?;
    let ct = hdr[0];
    let len = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
    if len > 65536 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "TLS record too large",
        ));
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    Ok((ct, payload, hdr))
}

fn parse_server_hello(payload: &[u8]) -> io::Result<([u8; 32], [u8; 32])> {
    if payload.len() < 4 || payload[0] != 0x02 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected ServerHello",
        ));
    }
    let body = &payload[4..];
    if body.len() < 34 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ServerHello too short",
        ));
    }
    let mut server_random = [0u8; 32];
    server_random.copy_from_slice(&body[2..34]);

    let session_id_len = body[34] as usize;
    let mut pos = 35 + session_id_len;
    if pos + 3 > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ServerHello truncated at cipher_suite",
        ));
    }
    pos += 3;

    if pos + 2 > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ServerHello truncated at extensions",
        ));
    }
    let ext_len = u16::from_be_bytes([body[pos], body[pos + 1]]) as usize;
    pos += 2;
    let ext_data = &body[pos..pos + ext_len];

    let mut ext_pos = 0;
    let mut key_share = None;
    while ext_pos + 4 <= ext_data.len() {
        let ext_type = u16::from_be_bytes([ext_data[ext_pos], ext_data[ext_pos + 1]]);
        let ext_size = u16::from_be_bytes([ext_data[ext_pos + 2], ext_data[ext_pos + 3]]) as usize;
        ext_pos += 4;
        if ext_pos + ext_size > ext_data.len() {
            break;
        }
        if ext_type == 0x0033 && ext_size >= 4 {
            let group = u16::from_be_bytes([ext_data[ext_pos], ext_data[ext_pos + 1]]);
            let key_len =
                u16::from_be_bytes([ext_data[ext_pos + 2], ext_data[ext_pos + 3]]) as usize;
            if group == 0x001D && key_len == 32 && ext_size >= 4 + key_len {
                let mut ks = [0u8; 32];
                ks.copy_from_slice(&ext_data[ext_pos + 4..ext_pos + 4 + 32]);
                key_share = Some(ks);
            }
        }
        ext_pos += ext_size;
    }
    let key_share = key_share.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "ServerHello missing key_share extension",
        )
    })?;
    Ok((server_random, key_share))
}

fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let mut mac = <hmac::Hmac<Sha256> as Mac>::new_from_slice(salt).expect("HMAC key len");
    mac.update(ikm);
    mac.finalize().into_bytes().into()
}

fn hkdf_expand_label(secret: &[u8], label: &str, context: &[u8], length: usize) -> Vec<u8> {
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

fn tls13_nonce(iv: &[u8; 12], seq: u64) -> [u8; 12] {
    let mut n = *iv;
    let be = seq.to_be_bytes();
    for i in 0..8 {
        n[4 + i] ^= be[i];
    }
    n
}

struct AeadState {
    cipher: Aes128Gcm,
    iv: [u8; 12],
    seq: u64,
}

impl AeadState {
    fn new(key: &[u8], iv: &[u8; 12]) -> Self {
        Self {
            cipher: Aes128Gcm::new(Key::<Aes128Gcm>::from_slice(key)),
            iv: *iv,
            seq: 0,
        }
    }

    fn decrypt(&mut self, payload: &[u8], aad: &[u8; 5]) -> io::Result<Vec<u8>> {
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

    fn encrypt(&mut self, plaintext: &[u8], record_type: u8, inner_ct: u8) -> io::Result<Vec<u8>> {
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

struct RealityTunnel {
    sock: TcpStream,
    encrypt: AeadState,
    decrypt: AeadState,
    residual: Vec<u8>,
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
        let (ct, payload, hdr) = read_tls_record(sock)?;
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
                ));
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
