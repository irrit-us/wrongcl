use std::io::Write;
use std::net::TcpStream;

use sha2::{Digest, Sha256};

use crate::client::Tunnel;
use crate::endpoint::{AnyTlsOptions, TlsOptions};
use crate::error::Result;
use crate::tls;

pub fn wrap(socket: TcpStream, opts: &AnyTlsOptions) -> Result<Box<dyn Tunnel>> {
    let tls_opts = TlsOptions {
        server_name: opts.server_name.clone(),
        insecure_skip_verify: opts.insecure_skip_verify,
        alpn: opts.alpn.clone(),
    };
    let mut tunnel = tls::wrap(socket, &tls_opts)?;
    let password_hash: [u8; 32] = Sha256::digest(opts.password.as_bytes()).into();
    tunnel.write_all(&password_hash)?;
    tunnel.write_all(&[0x00, 0x00])?;
    tunnel.flush().ok();
    Ok(tunnel)
}
