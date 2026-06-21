use std::io::{self, Read};
use std::sync::mpsc::Receiver;

use rand::RngCore;

use crate::endpoint::OuterSecurity;

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
        Ok(d) => d,
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

pub(super) fn build_http_get(path: &str, host: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Connection: close\r\n\
         \r\n"
    )
}

pub(super) fn read_http_response(stream: &mut dyn Read) -> io::Result<Vec<u8>> {
    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    let mut header_end = None;
    let mut content_length = None;

    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);
        if header_end.is_none() {
            if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
                header_end = Some(index + 4);
                let headers = std::str::from_utf8(&response[..index]).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP response")
                })?;
                let mut lines = headers.lines();
                let status = lines.next().unwrap_or_default();
                if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unexpected Google Docs Viewer HTTP status: {status}"),
                    ));
                }
                for line in lines {
                    if let Some((key, value)) = line.split_once(':') {
                        if key.trim().eq_ignore_ascii_case("content-length") {
                            content_length = Some(value.trim().parse::<usize>().map_err(|_| {
                                io::Error::new(io::ErrorKind::InvalidData, "invalid content-length")
                            })?);
                        }
                    }
                }
            }
        }
        if let (Some(header_end), Some(content_length)) = (header_end, content_length) {
            if response.len() >= header_end + content_length {
                break;
            }
        }
    }

    let header_end = header_end
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "short HTTP response"))?;
    let body = if let Some(content_length) = content_length {
        response[header_end..header_end + content_length.min(response.len() - header_end)].to_vec()
    } else {
        response[header_end..].to_vec()
    };
    Ok(body)
}

pub(super) fn normalized_path_prefix(value: &str, default: &str) -> String {
    let raw = value.trim();
    let raw = if raw.is_empty() { default } else { raw };
    let normalized = if raw.starts_with('/') {
        raw.to_string()
    } else {
        format!("/{raw}")
    };
    if normalized.len() > 1 {
        normalized.trim_end_matches('/').to_string()
    } else {
        normalized
    }
}

pub(super) fn request_host_header(
    outer_security: &OuterSecurity,
    server_host: &str,
    server_port: u16,
) -> String {
    match outer_security {
        OuterSecurity::Tls(opts) if !opts.server_name.trim().is_empty() => opts.server_name.clone(),
        _ => format!("{server_host}:{server_port}"),
    }
}

pub(super) fn random_path_segment() -> String {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub(super) fn random_session_bytes() -> Vec<u8> {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.to_vec()
}
