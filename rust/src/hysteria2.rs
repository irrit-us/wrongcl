use std::collections::HashMap;
use std::io;
use std::sync::mpsc;

use bytes::Bytes;

use crate::client::{Tunnel, UdpPacket, UdpSession};
use crate::endpoint::Hysteria2Options;
use crate::error::{ClientError, Result};
use crate::protocol::Target;

mod session;

use session::{
    authenticated_connection, encode_hysteria2_udp_message, parse_hysteria2_udp_message,
    read_hysteria2_tcp_response, target_authority, write_hysteria2_tcp_request,
    Hysteria2DatagramSession, Hysteria2PacketAssembly, Hysteria2Tunnel,
};

pub fn connect_hysteria2(
    server_host: &str,
    server_port: u16,
    opts: &Hysteria2Options,
    target: Target,
) -> Result<Box<dyn Tunnel>> {
    let target_address = target_authority(&target.host, target.port);
    let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>();
    let (write_tx, write_rx) = mpsc::sync_channel::<Vec<u8>>(32);
    let (hs_tx, hs_rx) = mpsc::sync_channel::<std::result::Result<(), io::Error>>(1);
    let (tokio_write_tx, mut tokio_write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
    let server_host = server_host.to_string();
    let opts = opts.clone();

    let handle = std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                let _ = hs_tx.send(Err(io::Error::other(format!("tokio runtime: {err}"))));
                return;
            }
        };

        let bridge_tx = tokio_write_tx;
        std::thread::spawn(move || {
            while let Ok(data) = write_rx.recv() {
                if bridge_tx.blocking_send(data).is_err() {
                    break;
                }
            }
        });

        rt.block_on(async move {
            match authenticated_connection(&server_host, server_port, &opts).await {
                Ok((_endpoint, conn, _udp_enabled)) => match conn.open_bi().await {
                    Ok((mut send, mut recv)) => {
                        if let Err(err) =
                            write_hysteria2_tcp_request(&mut send, &target_address).await
                        {
                            let _ = hs_tx.send(Err(err));
                            return;
                        }
                        match read_hysteria2_tcp_response(&mut recv).await {
                            Ok((true, _message)) => {
                                let _ = hs_tx.send(Ok(()));
                            }
                            Ok((false, message)) => {
                                let _ = hs_tx.send(Err(io::Error::new(
                                    io::ErrorKind::PermissionDenied,
                                    format!("Hysteria2 TCP request failed: {message}"),
                                )));
                                return;
                            }
                            Err(err) => {
                                let _ = hs_tx.send(Err(err));
                                return;
                            }
                        }

                        let read_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                            let mut buf = vec![0u8; 65536];
                            loop {
                                match recv.read(&mut buf).await {
                                    Ok(Some(n)) => {
                                        if n == 0 {
                                            let _ = read_tx.send(Vec::new());
                                            break;
                                        }
                                        if read_tx.send(buf[..n].to_vec()).is_err() {
                                            break;
                                        }
                                    }
                                    Ok(None) => {
                                        let _ = read_tx.send(Vec::new());
                                        break;
                                    }
                                    Err(_) => {
                                        let _ = read_tx.send(Vec::new());
                                        break;
                                    }
                                }
                            }
                        });

                        while let Some(data) = tokio_write_rx.recv().await {
                            if send
                                .write_all(&data)
                                .await
                                .map_err(io::Error::other)
                                .is_err()
                            {
                                break;
                            }
                        }

                        let _ = send.finish();
                        read_task.abort();
                    }
                    Err(err) => {
                        let _ = hs_tx.send(Err(io::Error::other(format!(
                            "open Hysteria2 stream: {err}"
                        ))));
                    }
                },
                Err(err) => {
                    let _ = hs_tx.send(Err(err));
                }
            }
        });
    });

    hs_rx
        .recv()
        .map_err(|_| ClientError::Io(io::Error::other("Hysteria2 thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(Hysteria2Tunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

pub fn connect_hysteria2_udp(
    server_host: &str,
    server_port: u16,
    opts: &Hysteria2Options,
    target: Target,
) -> Result<Box<dyn UdpSession>> {
    let target_address = target_authority(&target.host, target.port);
    let session_id = rand::random::<u32>();
    let (response_tx, response_rx) = mpsc::channel::<std::result::Result<UdpPacket, ClientError>>();
    let (write_tx, write_rx) = mpsc::sync_channel::<Vec<u8>>(64);
    let (hs_tx, hs_rx) = mpsc::sync_channel::<std::result::Result<(), io::Error>>(1);
    let (tokio_write_tx, mut tokio_write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
    let server_host = server_host.to_string();
    let opts = opts.clone();
    let target_for_thread = target.clone();

    let handle = std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                let _ = hs_tx.send(Err(io::Error::other(format!("tokio runtime: {err}"))));
                return;
            }
        };

        let bridge_tx = tokio_write_tx;
        std::thread::spawn(move || {
            while let Ok(data) = write_rx.recv() {
                if bridge_tx.blocking_send(data).is_err() {
                    break;
                }
            }
        });

        rt.block_on(async move {
            match authenticated_connection(&server_host, server_port, &opts).await {
                Ok((_endpoint, conn, udp_enabled)) => {
                    if !udp_enabled {
                        let _ = hs_tx.send(Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "Hysteria2 server disabled UDP relay",
                        )));
                        return;
                    }
                    let _ = hs_tx.send(Ok(()));

                    let read_conn = conn.clone();
                    let read_target = target_for_thread.clone();
                    let response_tx_read = response_tx.clone();
                    let read_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                        let mut assemblies = HashMap::<u16, Hysteria2PacketAssembly>::new();
                        loop {
                            match read_conn.read_datagram().await {
                                Ok(packet) => match parse_hysteria2_udp_message(packet.as_ref()) {
                                    Ok((
                                        incoming_session_id,
                                        _packet_id,
                                        _fragment_id,
                                        fragment_count,
                                        _address,
                                        payload,
                                    )) if incoming_session_id == session_id
                                        && fragment_count == 1 =>
                                    {
                                        if response_tx_read
                                            .send(Ok(UdpPacket {
                                                target: read_target.clone(),
                                                payload,
                                            }))
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                    Ok((
                                        incoming_session_id,
                                        packet_id,
                                        fragment_id,
                                        fragment_count,
                                        address,
                                        payload,
                                    )) if incoming_session_id == session_id
                                        && fragment_count > 1 =>
                                    {
                                        let assembly = match assemblies.entry(packet_id) {
                                            std::collections::hash_map::Entry::Occupied(entry) => {
                                                entry.into_mut()
                                            }
                                            std::collections::hash_map::Entry::Vacant(entry) => {
                                                match Hysteria2PacketAssembly::new(fragment_count) {
                                                    Ok(assembly) => entry.insert(assembly),
                                                    Err(err) => {
                                                        let _ = response_tx_read
                                                            .send(Err(ClientError::Io(err)));
                                                        break;
                                                    }
                                                }
                                            }
                                        };
                                        if let Err(err) =
                                            assembly.insert(fragment_id, address, payload)
                                        {
                                            let _ =
                                                response_tx_read.send(Err(ClientError::Io(err)));
                                            break;
                                        }
                                        if assembly.is_complete() {
                                            match assembly.take_payload() {
                                                Ok((_address, payload)) => {
                                                    assemblies.remove(&packet_id);
                                                    if response_tx_read
                                                        .send(Ok(UdpPacket {
                                                            target: read_target.clone(),
                                                            payload,
                                                        }))
                                                        .is_err()
                                                    {
                                                        break;
                                                    }
                                                }
                                                Err(err) => {
                                                    let _ = response_tx_read
                                                        .send(Err(ClientError::Io(err)));
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Ok(_) => {}
                                    Err(err) => {
                                        let _ = response_tx_read.send(Err(ClientError::Io(err)));
                                        break;
                                    }
                                },
                                Err(err) => {
                                    let _ = response_tx_read.send(Err(ClientError::Io(
                                        io::Error::other(format!("Hysteria2 UDP read: {err}")),
                                    )));
                                    break;
                                }
                            }
                        }
                    });

                    let mut packet_id: u16 = 0;
                    while let Some(payload) = tokio_write_rx.recv().await {
                        match encode_hysteria2_udp_message(
                            session_id,
                            packet_id,
                            0,
                            1,
                            &target_address,
                            &payload,
                        ) {
                            Ok(packet) => {
                                if conn.send_datagram(Bytes::from(packet)).is_err() {
                                    break;
                                }
                                packet_id = packet_id.wrapping_add(1);
                            }
                            Err(err) => {
                                let _ = response_tx.send(Err(ClientError::Io(err)));
                                break;
                            }
                        }
                    }

                    read_task.abort();
                }
                Err(err) => {
                    let _ = hs_tx.send(Err(err));
                }
            }
        });
    });

    hs_rx
        .recv()
        .map_err(|_| ClientError::Io(io::Error::other("Hysteria2 thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(Hysteria2DatagramSession {
        write_tx,
        response_rx,
        _handle: handle,
    }))
}
