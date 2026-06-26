use super::*;

use crate::client::remote::{
    clear_timeouts, connect_tcp, host_header, http_upgrade_handshake, normalized_path,
    remote_http_connect, remote_socks5_connect, remote_socks5_udp_associate, websocket_handshake,
};
use crate::client::udp::{
    open_remote_socks5_udp_session, open_shadowsocks_udp_session, open_stream_udp_session,
    open_trojan_udp_session,
};
use crate::client::websocket::WebSocketTunnel;

impl WrongsvClient {
    pub(super) fn connect_vless(
        &self,
        target: &Target,
        opts: &VlessOptions,
    ) -> Result<Box<dyn Tunnel>> {
        if let Transport::Kcp(kcp_opts) = &self.server.endpoint.transport {
            return kcp::connect_kcp(
                &self.server.host,
                self.server.port,
                kcp_opts,
                &opts.uuid,
                target,
                &opts.flow,
                false,
            );
        }
        if let Transport::Meek(meek_opts) = &self.server.endpoint.transport {
            let mut stream = meek::connect_meek(
                &self.server.host,
                self.server.port,
                meek_opts,
                &self.server.endpoint.outer_security,
                &opts.uuid,
                target,
                &opts.flow,
                false,
            )?;
            if opts.flow.trim() == VISION_FLOW {
                stream = vision::wrap(stream, &opts.uuid)?;
            }
            return Ok(stream);
        }
        if let Transport::Gdocsviewer(gdocs_opts) = &self.server.endpoint.transport {
            let mut stream = gdocsviewer::connect_gdocsviewer(
                &self.server.host,
                self.server.port,
                gdocs_opts,
                &self.server.endpoint.outer_security,
                &opts.uuid,
                target,
                &opts.flow,
                false,
            )?;
            if opts.flow.trim() == VISION_FLOW {
                stream = vision::wrap(stream, &opts.uuid)?;
            }
            return Ok(stream);
        }
        if let Transport::Webtransport(webtransport_opts) = &self.server.endpoint.transport {
            return webtransport::connect_webtransport(
                &self.server.host,
                self.server.port,
                webtransport_opts,
                &opts.uuid,
                target,
                &opts.flow,
                false,
            );
        }
        if let Transport::Quic(quic_opts) = &self.server.endpoint.transport {
            return quic::connect_quic(
                &self.server.host,
                self.server.port,
                quic_opts,
                &opts.uuid,
                target,
                &opts.flow,
                false,
            );
        }
        let (mut stream, timeout_handle) = self.open_proxy_stack()?;
        let header = encode_raw_vless_header(&opts.uuid, target, &opts.flow)?;
        stream.write_all(&header)?;
        read_raw_vless_response(&mut stream)?;
        if let Some(handle) = timeout_handle {
            clear_timeouts(&handle)?;
        }
        let flow = opts.flow.trim();
        if flow == VISION_FLOW {
            stream = vision::wrap(stream, &opts.uuid)?;
        }
        Ok(stream)
    }

    pub(super) fn connect_vless_udp(
        &self,
        target: &Target,
        opts: &VlessOptions,
    ) -> Result<Box<dyn UdpSession>> {
        if let Transport::Kcp(kcp_opts) = &self.server.endpoint.transport {
            if opts.flow.trim() == VISION_FLOW {
                return Err(ClientError::UnsupportedProtocol(
                    "XTLS Vision does not support UDP".into(),
                ));
            }
            let stream = kcp::connect_kcp(
                &self.server.host,
                self.server.port,
                kcp_opts,
                &opts.uuid,
                target,
                &opts.flow,
                true,
            )?;
            return open_stream_udp_session(stream, target.clone());
        }
        if let Transport::Meek(meek_opts) = &self.server.endpoint.transport {
            if opts.flow.trim() == VISION_FLOW {
                return Err(ClientError::UnsupportedProtocol(
                    "XTLS Vision does not support UDP".into(),
                ));
            }
            let stream = meek::connect_meek(
                &self.server.host,
                self.server.port,
                meek_opts,
                &self.server.endpoint.outer_security,
                &opts.uuid,
                target,
                &opts.flow,
                true,
            )?;
            return open_stream_udp_session(stream, target.clone());
        }
        if let Transport::Gdocsviewer(gdocs_opts) = &self.server.endpoint.transport {
            if opts.flow.trim() == VISION_FLOW {
                return Err(ClientError::UnsupportedProtocol(
                    "XTLS Vision does not support UDP".into(),
                ));
            }
            let stream = gdocsviewer::connect_gdocsviewer(
                &self.server.host,
                self.server.port,
                gdocs_opts,
                &self.server.endpoint.outer_security,
                &opts.uuid,
                target,
                &opts.flow,
                true,
            )?;
            return open_stream_udp_session(stream, target.clone());
        }
        if let Transport::Webtransport(webtransport_opts) = &self.server.endpoint.transport {
            if opts.flow.trim() == VISION_FLOW {
                return Err(ClientError::UnsupportedProtocol(
                    "XTLS Vision does not support UDP".into(),
                ));
            }
            if !webtransport_opts.udp_enabled {
                return Err(ClientError::UnsupportedProtocol(
                    "WebTransport UDP relay is disabled for this wrongcl profile".into(),
                ));
            }
            let stream = webtransport::connect_webtransport(
                &self.server.host,
                self.server.port,
                webtransport_opts,
                &opts.uuid,
                target,
                &opts.flow,
                true,
            )?;
            return open_stream_udp_session(stream, target.clone());
        }
        if let Transport::Quic(quic_opts) = &self.server.endpoint.transport {
            if opts.flow.trim() == VISION_FLOW {
                return Err(ClientError::UnsupportedProtocol(
                    "XTLS Vision does not support UDP".into(),
                ));
            }
            if !quic_opts.udp_enabled {
                return Err(ClientError::UnsupportedProtocol(
                    "QUIC UDP relay is disabled for this wrongcl profile".into(),
                ));
            }
            let stream = quic::connect_quic(
                &self.server.host,
                self.server.port,
                quic_opts,
                &opts.uuid,
                target,
                &opts.flow,
                true,
            )?;
            return open_stream_udp_session(stream, target.clone());
        }
        if opts.flow.trim() == VISION_FLOW {
            return Err(ClientError::UnsupportedProtocol(
                "XTLS Vision does not support UDP".into(),
            ));
        }
        let (mut stream, timeout_handle) = self.open_proxy_stack()?;
        let header = encode_udp_vless_header(&opts.uuid, target, &opts.flow)?;
        stream.write_all(&header)?;
        read_raw_vless_response(&mut stream)?;
        if let Some(handle) = timeout_handle {
            clear_timeouts(&handle)?;
        }
        open_stream_udp_session(stream, target.clone())
    }

    pub(super) fn connect_naive(
        &self,
        target: &Target,
        opts: &crate::endpoint::NaiveOptions,
    ) -> Result<Box<dyn Tunnel>> {
        let tls_opts = match &self.server.endpoint.outer_security {
            OuterSecurity::Tls(opts) => opts,
            _ => {
                return Err(ClientError::Config(
                    "Naive requires TLS as outer security".into(),
                ));
            }
        };
        naive::connect_naive(
            &self.server.host,
            self.server.port,
            opts,
            tls_opts,
            &target.host,
            target.port,
        )
    }

    pub(super) fn connect_naive_udp(
        &self,
        _target: &Target,
        _opts: &crate::endpoint::NaiveOptions,
    ) -> Result<Box<dyn UdpSession>> {
        Err(ClientError::UnsupportedProtocol(
            "Naive does not support UDP relay".into(),
        ))
    }

    pub(super) fn connect_hysteria2(
        &self,
        target: &Target,
        opts: &Hysteria2Options,
    ) -> Result<Box<dyn Tunnel>> {
        hysteria2::connect_hysteria2(&self.server.host, self.server.port, opts, target.clone())
    }

    pub(super) fn connect_hysteria2_udp(
        &self,
        target: &Target,
        opts: &Hysteria2Options,
    ) -> Result<Box<dyn UdpSession>> {
        if !opts.udp_enabled {
            return Err(ClientError::UnsupportedProtocol(
                "Hysteria2 UDP relay is disabled for this wrongcl profile".into(),
            ));
        }
        hysteria2::connect_hysteria2_udp(&self.server.host, self.server.port, opts, target.clone())
    }

    pub(super) fn connect_tuic(
        &self,
        target: &Target,
        opts: &crate::endpoint::TuicOptions,
    ) -> Result<Box<dyn Tunnel>> {
        tuic::connect_tuic(&self.server.host, self.server.port, opts, target.clone())
    }

    pub(super) fn connect_tuic_udp(
        &self,
        target: &Target,
        opts: &crate::endpoint::TuicOptions,
    ) -> Result<Box<dyn UdpSession>> {
        tuic::connect_tuic_udp(&self.server.host, self.server.port, opts, target.clone())
    }

    pub(super) fn connect_trojan(
        &self,
        target: &Target,
        opts: &TrojanOptions,
    ) -> Result<Box<dyn Tunnel>> {
        let (mut stream, timeout_handle) = self.open_proxy_stack()?;
        let header = trojan::encode_handshake(&opts.password, target)?;
        stream.write_all(&header)?;
        stream.flush().ok();
        if let Some(handle) = timeout_handle {
            clear_timeouts(&handle)?;
        }
        Ok(stream)
    }

    pub(super) fn connect_trojan_udp(
        &self,
        target: &Target,
        opts: &TrojanOptions,
    ) -> Result<Box<dyn UdpSession>> {
        let (mut stream, timeout_handle) = self.open_proxy_stack()?;
        let header = trojan::encode_udp_associate_handshake(&opts.password)?;
        stream.write_all(&header)?;
        stream.flush().ok();
        if let Some(handle) = timeout_handle {
            clear_timeouts(&handle)?;
        }
        open_trojan_udp_session(stream, target.clone())
    }

    pub(super) fn connect_mixed(
        &self,
        target: &Target,
        opts: &MixedOptions,
    ) -> Result<Box<dyn Tunnel>> {
        let mut tcp = self.connect_tcp_with_timeouts()?;
        tcp.set_read_timeout(Some(MIXED_DETECT_TIMEOUT))?;
        tcp.set_write_timeout(Some(MIXED_DETECT_TIMEOUT))?;
        match remote_socks5_connect(&mut tcp, opts, target) {
            Ok(()) => {
                clear_timeouts(&tcp)?;
                Ok(Box::new(tcp))
            }
            Err(socks_err) => {
                let mut http = self.connect_tcp_with_timeouts()?;
                match remote_http_connect(&mut http, opts, target) {
                    Ok(()) => {
                        clear_timeouts(&http)?;
                        Ok(Box::new(http))
                    }
                    Err(http_err) => Err(ClientError::Config(format!(
                        "remote mixed proxy connect failed: SOCKS5 path: {socks_err}; HTTP CONNECT path: {http_err}"
                    ))),
                }
            }
        }
    }

    pub(super) fn connect_mixed_udp(
        &self,
        target: &Target,
        opts: &MixedOptions,
    ) -> Result<Box<dyn UdpSession>> {
        let mut tcp = self.connect_tcp_with_timeouts()?;
        let relay_target = remote_socks5_udp_associate(&mut tcp, opts)?;
        clear_timeouts(&tcp)?;
        open_remote_socks5_udp_session(tcp, relay_target, target.clone())
    }

    pub(super) fn connect_shadowsocks(
        &self,
        target: &Target,
        opts: &ShadowsocksOptions,
    ) -> Result<Box<dyn Tunnel>> {
        let tcp = self.connect_tcp_with_timeouts()?;
        let timeout_handle = tcp.try_clone()?;
        let inner: Box<dyn Tunnel> = Box::new(tcp);
        let tunnel = ss::open_tunnel(inner, opts, target)?;
        clear_timeouts(&timeout_handle)?;
        Ok(tunnel)
    }

    pub(super) fn connect_shadowsocks_udp(
        &self,
        target: &Target,
        opts: &ShadowsocksOptions,
    ) -> Result<Box<dyn UdpSession>> {
        let config = wrongsv_shadowsocks::ServerConfig::new(&opts.method, opts.password.clone())
            .map_err(|e| ClientError::Config(format!("Shadowsocks: {e}")))?;
        let server_addr = format!("{}:{}", self.server.host, self.server.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| {
                ClientError::Config("failed to resolve Shadowsocks server address".into())
            })?;
        open_shadowsocks_udp_session(config, server_addr, target.clone())
    }

    pub(super) fn connect_snell(
        &self,
        target: &Target,
        opts: &SnellOptions,
    ) -> Result<Box<dyn Tunnel>> {
        let tcp = self.connect_tcp_with_timeouts()?;
        let timeout_handle = tcp.try_clone()?;
        let inner: Box<dyn Tunnel> = Box::new(tcp);
        let tunnel = snell::open_tunnel(inner, opts, target)?;
        clear_timeouts(&timeout_handle)?;
        Ok(tunnel)
    }

    pub(super) fn connect_snell_udp(
        &self,
        _target: &Target,
        _opts: &SnellOptions,
    ) -> Result<Box<dyn UdpSession>> {
        Err(ClientError::UnsupportedProtocol(
            "Snell v1 does not support UDP relay in wrongcl".into(),
        ))
    }

    pub(super) fn connect_wireguard(
        &self,
        target: &Target,
        opts: &crate::endpoint::WireGuardOptions,
    ) -> Result<Box<dyn Tunnel>> {
        wireguard::connect_wireguard(&self.server.host, self.server.port, opts, target)
    }

    pub(super) fn connect_wireguard_udp(
        &self,
        target: &Target,
        opts: &crate::endpoint::WireGuardOptions,
    ) -> Result<Box<dyn UdpSession>> {
        wireguard::connect_wireguard_udp(&self.server.host, self.server.port, opts, target)
    }

    pub(super) fn open_proxy_stack(&self) -> Result<(Box<dyn Tunnel>, Option<TcpStream>)> {
        if let Transport::Xhttp(xopts) = &self.server.endpoint.transport {
            let stream = crate::xhttp::connect_xhttp(
                &self.server.host,
                self.server.port,
                xopts,
                &self.server.endpoint.outer_security,
            )?;
            return Ok((stream, None));
        }
        if let Transport::Grpc(gopts) = &self.server.endpoint.transport {
            let stream = crate::grpc::connect_grpc(
                &self.server.host,
                self.server.port,
                gopts,
                &self.server.endpoint.outer_security,
            )?;
            return Ok((stream, None));
        }
        let tcp = self.connect_tcp_with_timeouts()?;
        let timeout_handle = tcp.try_clone()?;
        let stream = self.wrap_outer_then_transport(tcp)?;
        Ok((stream, Some(timeout_handle)))
    }

    fn wrap_outer_then_transport(&self, tcp: TcpStream) -> Result<Box<dyn Tunnel>> {
        let outer = wrap_outer_security(tcp, &self.server.endpoint.outer_security)?;
        wrap_transport(
            outer,
            &self.server.endpoint.transport,
            &self.server.host,
            self.server.port,
        )
    }

    fn connect_tcp_with_timeouts(&self) -> Result<TcpStream> {
        let stream = connect_tcp(&self.server.host, self.server.port)?;
        stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
        stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;
        Ok(stream)
    }
}

fn wrap_outer_security(tcp: TcpStream, outer: &OuterSecurity) -> Result<Box<dyn Tunnel>> {
    match outer {
        OuterSecurity::None => Ok(Box::new(tcp)),
        OuterSecurity::Tls(opts) => tls::wrap(tcp, opts),
        OuterSecurity::Reality(opts) => reality::wrap(tcp, opts),
        OuterSecurity::AnyTls(opts) => anytls::wrap(tcp, opts),
        OuterSecurity::ShadowTls(opts) => shadowtls::wrap(tcp, opts),
    }
}

fn wrap_transport(
    inner: Box<dyn Tunnel>,
    transport: &Transport,
    server_host: &str,
    server_port: u16,
) -> Result<Box<dyn Tunnel>> {
    match transport {
        Transport::Raw => Ok(inner),
        Transport::Fragment(opts) => Ok(fragment::wrap(inner, opts)),
        Transport::Httpupgrade(opts) => connect_httpupgrade(inner, opts, server_host, server_port),
        Transport::Meek(_) => Err(ClientError::Config(
            "Meek transport must be opened directly, not wrap_transport".into(),
        )),
        Transport::Gdocsviewer(_) => Err(ClientError::Config(
            "Google Docs Viewer transport must be opened directly, not wrap_transport".into(),
        )),
        Transport::Websocket(opts) => connect_websocket(inner, opts, server_host, server_port),
        Transport::Xhttp(_) => Err(ClientError::Config(
            "XHTTP transport must be opened via open_proxy_stack, not wrap_transport".into(),
        )),
        Transport::Grpc(_) => Err(ClientError::Config(
            "gRPC transport must be opened via open_proxy_stack, not wrap_transport".into(),
        )),
        Transport::Kcp(_) => Err(ClientError::Config(
            "KCP transport must be opened directly, not wrap_transport".into(),
        )),
        Transport::Webtransport(_) => Err(ClientError::Config(
            "WebTransport transport must be opened directly, not wrap_transport".into(),
        )),
        Transport::Quic(_) => Err(ClientError::Config(
            "QUIC transport must be opened directly, not wrap_transport".into(),
        )),
    }
}

fn connect_httpupgrade(
    mut inner: Box<dyn Tunnel>,
    opts: &HuOptions,
    server_host: &str,
    server_port: u16,
) -> Result<Box<dyn Tunnel>> {
    let path = normalized_path(&opts.path, "/");
    let host = host_header(opts.host.as_deref(), server_host, server_port);
    http_upgrade_handshake(inner.as_mut(), &path, host)?;
    Ok(inner)
}

fn connect_websocket(
    mut inner: Box<dyn Tunnel>,
    opts: &WsOptions,
    server_host: &str,
    server_port: u16,
) -> Result<Box<dyn Tunnel>> {
    let path = normalized_path(&opts.path, "/");
    let host = host_header(opts.host.as_deref(), server_host, server_port);
    websocket_handshake(inner.as_mut(), &path, host)?;
    Ok(Box::new(WebSocketTunnel::new(inner)))
}
