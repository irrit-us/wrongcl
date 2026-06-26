use super::*;

impl Endpoint {
    pub fn stack_summary(&self) -> String {
        if matches!(self.proxy, ProxyProtocol::Hysteria2(_)) {
            return "Hysteria2 → QUIC → TLS → TCP".into();
        }
        if matches!(self.proxy, ProxyProtocol::Tuic(_)) {
            return "TUIC → QUIC → TLS → TCP".into();
        }
        if matches!(self.proxy, ProxyProtocol::Naive(_)) {
            return "Naive → h2 CONNECT → TLS → TCP".into();
        }
        if matches!(self.proxy, ProxyProtocol::Snell(_)) {
            return "Snell → raw TCP".into();
        }
        if matches!(self.proxy, ProxyProtocol::Wireguard(_)) {
            return "Payload IP → WireGuard → UDP".into();
        }
        if matches!(self.transport, Transport::Quic(_)) {
            return "VLESS → QUIC → TLS → TCP".into();
        }
        if matches!(self.transport, Transport::Kcp(_)) {
            return "VLESS → KCP → TCP".into();
        }
        if matches!(self.transport, Transport::Webtransport(_)) {
            return "VLESS → WebTransport → QUIC → TLS → TCP".into();
        }
        let mut parts: Vec<&str> = Vec::new();
        parts.push(self.proxy.display_name());
        match self.transport {
            Transport::Raw => parts.push("raw"),
            Transport::Kcp(_) => parts.push("KCP"),
            Transport::Meek(_) => parts.push("Meek"),
            Transport::Gdocsviewer(_) => parts.push("Google Docs Viewer"),
            Transport::Webtransport(_) => parts.push("WebTransport"),
            Transport::Websocket(_) => parts.push("WebSocket"),
            Transport::Httpupgrade(_) => parts.push("HTTPUpgrade"),
            Transport::Xhttp(_) => parts.push("XHTTP"),
            Transport::Grpc(_) => parts.push("gRPC"),
            Transport::Quic(_) => parts.push("QUIC"),
        }
        match self.outer_security {
            OuterSecurity::Tls(_) => parts.push("TLS"),
            OuterSecurity::Reality(_) => parts.push("REALITY"),
            OuterSecurity::AnyTls(_) => parts.push("AnyTLS"),
            OuterSecurity::ShadowTls(_) => parts.push("ShadowTLS"),
            OuterSecurity::None => {}
        }
        parts.push("TCP");
        parts.join(" → ")
    }
}
