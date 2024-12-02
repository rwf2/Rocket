//! TCP listener.
//!
//! # Configuration
//!
//! Reads the following configuration parameters:
//!
//! | parameter | type         | default     | note                            |
//! |-----------|--------------|-------------|---------------------------------|
//! | `address` | [`Endpoint`] | `127.0.0.1` | must be `tcp:ip`                |
//! | `port`    | `u16`        | `8000`      | replaces the port in `address ` |

use std::io;
use std::net::{Ipv4Addr, SocketAddr};

use either::{Either, Left, Right};

#[doc(inline)]
pub use tokio::net::{TcpListener, TcpStream};

use crate::{Ignite, Rocket};
use crate::listener::{Bind, Connection, Endpoint, Listener};

impl Bind for TcpListener {
    type Error = Either<figment::Error, io::Error>;

    async fn bind(rocket: &Rocket<Ignite>) -> Result<Self, Self::Error> {
        let endpoint = Self::bind_endpoint(rocket)?;
        let addr = endpoint.tcp()
            .ok_or_else(|| io::Error::other("internal error: invalid endpoint"))
            .map_err(Right)?;

        Self::bind(addr).await.map_err(Right)
    }

    fn bind_endpoint(rocket: &Rocket<Ignite>) -> Result<Endpoint, Self::Error> {
        let figment = rocket.figment();
        let mut address = Endpoint::fetch(figment, "tcp", "address", |e| {
            let default = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8000);
            e.map(|e| e.tcp()).unwrap_or(Some(default))
        }).map_err(Left)?;

        if figment.contains("port") {
            let port = figment.extract_inner("port").map_err(Left)?;
            address.set_port(port);
        }

        Ok(Endpoint::Tcp(address))
    }
}

impl Listener for TcpListener {
    type Accept = Self::Connection;

    type Connection = TcpStream;

    async fn accept(&self) -> io::Result<Self::Accept> {
        let conn = self.accept().await?.0;
        let _ = conn.set_nodelay(true);
        let _ = conn.set_linger(None);
        Ok(conn)
    }

    async fn connect(&self, conn: Self::Connection) -> io::Result<Self::Connection> {
        Ok(conn)
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        self.local_addr().map(Endpoint::Tcp)
    }
}

impl Connection for TcpStream {
    fn endpoint(&self) -> io::Result<Endpoint> {
        self.peer_addr().map(Endpoint::Tcp)
    }
}
