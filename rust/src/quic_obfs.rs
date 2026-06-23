#[allow(dead_code, clippy::manual_saturating_arithmetic)]
#[path = "../../../wrongsv/crates/server/src/handler/quic_obfs.rs"]
mod shared;

pub(crate) use shared::{
    GECKO_DEFAULT_MAX_PACKET_SIZE, GECKO_DEFAULT_MIN_PACKET_SIZE, wrap_async_udp_socket_gecko,
    wrap_async_udp_socket_salamander,
};
