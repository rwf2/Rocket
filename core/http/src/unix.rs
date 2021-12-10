use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::{UnixListener, UnixStream};

use crate::listener::{Connection, Listener};

impl Listener for UnixListener {
    type Connection = UnixStream;

    fn local_addr(&self) -> Option<SocketAddr> {
        Some("0.0.0.0:0".parse().ok()?)
    }

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<Self::Connection>> {
        (*self).poll_accept(cx).map_ok(|(stream, _addr)| stream)
    }
}

impl Connection for UnixStream {
    fn peer_address(&self) -> Option<SocketAddr> {
        Some("0.0.0.0:0".parse().ok()?)
    }
}
