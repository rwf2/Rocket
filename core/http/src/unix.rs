//! Provides hyper client and server bindings for unix domain sockets.

use std::io::{self, Read, Write};
use std::path::Path;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use std::os::unix::net::{UnixListener, UnixStream};

use crate::hyper;
use crate::hyper::net::{NetworkStream, NetworkListener};
use crate::hyper::Server;

/// A type which implements hyper's NetworkStream trait.
pub struct UnixSocketStream(pub UnixStream);

impl Clone for UnixSocketStream {
    #[inline]
    fn clone(&self) -> UnixSocketStream {
        UnixSocketStream(self.0.try_clone().unwrap())
    }
}

impl NetworkStream for UnixSocketStream {
    #[inline]
    fn peer_addr(&mut self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
            .map(|_| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)))
    }

    #[inline]
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.0.set_read_timeout(dur)
    }

    #[inline]
    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.0.set_write_timeout(dur)
    }
}

impl Read for UnixSocketStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for UnixSocketStream {
    #[inline]
    fn write(&mut self, msg: &[u8]) -> io::Result<usize> {
        self.0.write(msg)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// A type which implements hyper's NetworkListener trait
#[derive(Debug)]
pub struct UnixSocketListener(pub UnixListener);

impl Clone for UnixSocketListener {
    #[inline]
    fn clone(&self) -> UnixSocketListener {
        UnixSocketListener(self.0.try_clone().unwrap())
    }
}

impl UnixSocketListener {
    /// Start listening to an address over HTTP.
    pub fn new<P: AsRef<Path>>(addr: P) -> hyper::Result<UnixSocketListener> {
        Ok(UnixSocketListener(UnixListener::bind(addr)?))
    }
}

impl NetworkListener for UnixSocketListener {
    type Stream = UnixSocketStream;

    #[inline]
    fn accept(&mut self) -> hyper::Result<UnixSocketStream> {
        Ok(UnixSocketStream(self.0.accept()?.0))
    }

    #[inline]
    fn local_addr(&mut self) -> io::Result<SocketAddr> {
        // return a dummy addr
        self.0.local_addr().map(|_| {
            SocketAddr::V4(
                SocketAddrV4::new(
                    Ipv4Addr::new(0, 0, 0, 0), 0
                )
            )
        })
    }
}

/// A type that provides a factory interface for creating unix socket based
/// hyper servers.
pub struct UnixSocketServer;

impl UnixSocketServer {
    /// creates a new hyper Server from a unix socket path
    pub fn http<P>(path: P) -> hyper::Result<Server<UnixSocketListener>>
        where P: AsRef<Path>
    {
        UnixSocketListener::new(path).map(Server::new)
    }
}

#[cfg(feature = "tls")]
mod tls {
    use super::*;

    use crate::hyper::{self, net::SslServer};
    use crate::tls::{TlsStream, ServerSession, TlsServer, WrappedStream};
    use crate::unix::UnixSocketStream;

    pub type UnixHttpsStream = WrappedStream<ServerSession, UnixSocketStream>;

    impl UnixSocketServer {
        pub fn https<P, S>(path: P, ssl: S) -> hyper::Result<Server<HttpsListener<S>>>
            where P: AsRef<Path>, S: SslServer<UnixSocketStream> + Clone
        {
            HttpsListener::new(path, ssl).map(Server::new)
        }
    }

    #[derive(Clone)]
    pub struct HttpsListener<S: SslServer<UnixSocketStream>> {
        listener: UnixSocketListener,
        ssl: S,
    }

    impl<S: SslServer<UnixSocketStream>> HttpsListener<S> {
        /// Start listening to an address over HTTPS.
        pub fn new<P>(path: P, ssl: S) -> hyper::Result<HttpsListener<S>>
            where P: AsRef<Path>
        {
            UnixSocketListener::new(path)
                .map(|listener| HttpsListener { listener, ssl })
        }
    }

    impl<S> NetworkListener for HttpsListener<S>
        where S: SslServer<UnixSocketStream> + Clone
    {
        type Stream = S::Stream;

        #[inline]
        fn accept(&mut self) -> hyper::Result<S::Stream> {
            self.listener.accept().and_then(|s| self.ssl.wrap_server(s))
        }

        #[inline]
        fn local_addr(&mut self) -> io::Result<SocketAddr> {
            self.listener.local_addr()
        }

        fn set_read_timeout(&mut self, duration: Option<Duration>) {
            self.listener.set_read_timeout(duration)
        }

        fn set_write_timeout(&mut self, duration: Option<Duration>) {
            self.listener.set_write_timeout(duration)
        }
    }

    impl SslServer<UnixSocketStream> for TlsServer {
        type Stream = WrappedStream<ServerSession, UnixSocketStream>;

        fn wrap_server(
            &self,
            stream: UnixSocketStream
        ) -> hyper::Result<WrappedStream<ServerSession, UnixSocketStream>> {
            let tls = TlsStream::new(rustls::ServerSession::new(&self.cfg), stream);
            Ok(WrappedStream::new(tls))
        }
    }
}

#[cfg(feature = "tls")]
pub use self::tls::*;
