use std::collections::HashMap;
use std::io;
use std::sync::mpsc;

use bytes::Bytes;

use crate::client::{Tunnel, UdpPacket, UdpSession};
use crate::endpoint::TuicOptions;
use crate::error::{ClientError, Result};
use crate::protocol::Target;

mod codec;
mod session;

use codec::{
    TuicCommand, TuicPacketAssembly, fragment_tuic_payload, parse_tuic_datagram_command,
    target_authority,
};
use session::{
    TuicDatagramSession, TuicTunnel, authenticated_connection, write_tuic_connect_request,
};

pub fn connect_tuic(
    server_host: &str,
    server_port: u16,
    opts: &TuicOptions,
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
                Ok((_endpoint, conn)) => match conn.open_bi().await {
                    Ok((mut send, mut recv)) => {
                        if let Err(err) =
                            write_tuic_connect_request(&mut send, &target_address).await
                        {
                            let _ = hs_tx.send(Err(err));
                            return;
                        }
                        let _ = hs_tx.send(Ok(()));

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
                        let _ =
                            hs_tx.send(Err(io::Error::other(format!("open TUIC stream: {err}"))));
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
        .map_err(|_| ClientError::Io(io::Error::other("TUIC thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(TuicTunnel {
        read_rx,
        write_tx,
        read_buf: Vec::new(),
        eof: false,
        _handle: handle,
    }))
}

pub fn connect_tuic_udp(
    server_host: &str,
    server_port: u16,
    opts: &TuicOptions,
    target: Target,
) -> Result<Box<dyn UdpSession>> {
    let target_address = target_authority(&target.host, target.port);
    let assoc_id = rand::random::<u16>();
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
                Ok((_endpoint, conn)) => {
                    let _ = hs_tx.send(Ok(()));

                    let read_conn = conn.clone();
                    let read_target = target_for_thread.clone();
                    let response_tx_read = response_tx.clone();
                    let read_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                        let mut assemblies: HashMap<(u16, u16), TuicPacketAssembly> =
                            HashMap::new();
                        loop {
                            match read_conn.read_datagram().await {
                                Ok(packet) => match parse_tuic_datagram_command(packet.as_ref()) {
                                    Ok(TuicCommand::Packet(packet))
                                        if packet.assoc_id == assoc_id =>
                                    {
                                        let key = (packet.assoc_id, packet.packet_id);
                                        let payload = if packet.frag_total <= 1 {
                                            Some(packet.payload)
                                        } else {
                                            let assembly =
                                                assemblies.entry(key).or_insert_with(|| {
                                                    TuicPacketAssembly::new(packet.frag_total)
                                                });
                                            if let Err(err) = assembly.insert(
                                                packet.fragment_index,
                                                packet.address.clone(),
                                                packet.payload,
                                            ) {
                                                let _ = response_tx_read.send(Err(
                                                    ClientError::Io(io::Error::new(
                                                        io::ErrorKind::InvalidData,
                                                        err,
                                                    )),
                                                ));
                                                break;
                                            }
                                            if assembly.is_complete() {
                                                match assembly.take_payload() {
                                                    Ok((_address, payload)) => {
                                                        assemblies.remove(&key);
                                                        Some(payload)
                                                    }
                                                    Err(err) => {
                                                        let _ = response_tx_read.send(Err(
                                                            ClientError::Io(io::Error::new(
                                                                io::ErrorKind::InvalidData,
                                                                err,
                                                            )),
                                                        ));
                                                        break;
                                                    }
                                                }
                                            } else {
                                                None
                                            }
                                        };

                                        if let Some(payload) = payload {
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
                                    }
                                    Ok(TuicCommand::Packet(_)) => {}
                                    Err(err) => {
                                        let _ = response_tx_read.send(Err(ClientError::Io(
                                            io::Error::new(io::ErrorKind::InvalidData, err),
                                        )));
                                        break;
                                    }
                                },
                                Err(err) => {
                                    let _ = response_tx_read.send(Err(ClientError::Io(
                                        io::Error::other(format!("TUIC UDP read: {err}")),
                                    )));
                                    break;
                                }
                            }
                        }
                    });

                    let mut packet_id: u16 = 0;
                    while let Some(payload) = tokio_write_rx.recv().await {
                        match fragment_tuic_payload(assoc_id, &target_address, &payload, packet_id)
                        {
                            Ok(packets) => {
                                for packet in packets {
                                    if conn.send_datagram(Bytes::from(packet)).is_err() {
                                        return;
                                    }
                                }
                                packet_id = packet_id.wrapping_add(1);
                            }
                            Err(err) => {
                                let _ = response_tx.send(Err(ClientError::Io(io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    err,
                                ))));
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
        .map_err(|_| ClientError::Io(io::Error::other("TUIC thread panicked")))?
        .map_err(ClientError::Io)?;

    Ok(Box::new(TuicDatagramSession {
        write_tx,
        response_rx,
        _handle: handle,
    }))
}
