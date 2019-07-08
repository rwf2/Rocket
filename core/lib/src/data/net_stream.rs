use std::io;
use std::net::{SocketAddr, Shutdown};
use std::time::Duration;

#[cfg(feature = "tls")] use crate::http::tls::{WrappedStream, ServerSession};
use crate::http::hyper::net::{HttpStream, NetworkStream};

use self::NetStream::*;

#[cfg(feature = "tls")] pub type HttpsStream = WrappedStream<ServerSession>;
#[cfg(any(unix, windows))] use crate::http::unix::UnixSocketStream;
#[cfg(all(any(unix, windows), feature = "tls"))] use crate::http::unix::UnixHttpsStream;

// This is a representation of all of the possible network streams we might get.
// This really shouldn't be necessary, but, you know, Hyper.
#[derive(Clone)]
pub enum NetStream {
    Http(HttpStream),
    #[cfg(feature = "tls")]
    Https(HttpsStream),
    #[cfg(any(unix, windows))]
    UnixHttp(UnixSocketStream),
    #[cfg(all(any(unix, windows), feature = "tls"))]
    UnixHttps(UnixHttpsStream),
    Empty,
}

impl io::Read for NetStream {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        trace_!("NetStream::read()");
        let res = match *self {
            Http(ref mut stream) => stream.read(buf),
            #[cfg(feature = "tls")] Https(ref mut stream) => stream.read(buf),
            #[cfg(any(unix, windows))] UnixHttp(ref mut stream) => stream.read(buf),
            #[cfg(all(any(unix, windows), feature = "tls"))] UnixHttps(ref mut s) => s.read(buf),
            Empty => Ok(0),
        };

        trace_!("NetStream::read() -- complete");
        res
    }
}

impl io::Write for NetStream {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        trace_!("NetStream::write()");
        match *self {
            Http(ref mut stream) => stream.write(buf),
            #[cfg(feature = "tls")] Https(ref mut stream) => stream.write(buf),
            #[cfg(any(unix, windows))] UnixHttp(ref mut stream) => stream.write(buf),
            #[cfg(all(any(unix, windows), feature = "tls"))] UnixHttps(ref mut s) => s.write(buf),
            Empty => Ok(0),
        }
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        match *self {
            Http(ref mut stream) => stream.flush(),
            #[cfg(feature = "tls")] Https(ref mut stream) => stream.flush(),
            #[cfg(any(unix, windows))] UnixHttp(ref mut stream) => stream.flush(),
            #[cfg(all(any(unix, windows), feature = "tls"))] UnixHttps(ref mut s) => s.flush(),
            Empty => Ok(()),
        }
    }
}

impl NetworkStream for NetStream {
    #[inline(always)]
    fn peer_addr(&mut self) -> io::Result<SocketAddr> {
        match *self {
            Http(ref mut stream) => stream.peer_addr(),
            #[cfg(feature = "tls")] Https(ref mut stream) => stream.peer_addr(),
            #[cfg(any(unix, windows))] UnixHttp(ref mut stream) => stream.peer_addr(),
            #[cfg(all(any(unix, windows), feature = "tls"))] UnixHttps(ref mut s) => s.peer_addr(),
            Empty => Err(io::Error::from(io::ErrorKind::AddrNotAvailable)),
        }
    }

    #[inline(always)]
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        match *self {
            Http(ref stream) => stream.set_read_timeout(d),
            #[cfg(feature = "tls")] Https(ref stream) => stream.set_read_timeout(d),
            #[cfg(any(unix, windows))] UnixHttp(ref stream) => stream.set_read_timeout(d),
            #[cfg(all(any(unix, windows), feature = "tls"))] UnixHttps(ref s) => s.set_read_timeout(d),
            Empty => Ok(()),
        }
    }

    #[inline(always)]
    fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        match *self {
            Http(ref stream) => stream.set_write_timeout(d),
            #[cfg(feature = "tls")] Https(ref stream) => stream.set_write_timeout(d),
            #[cfg(any(unix, windows))] UnixHttp(ref stream) => stream.set_write_timeout(d),
            #[cfg(all(any(unix, windows), feature = "tls"))] UnixHttps(ref s) => s.set_write_timeout(d),
            Empty => Ok(()),
        }
    }

    #[inline(always)]
    fn close(&mut self, how: Shutdown) -> io::Result<()> {
        match *self {
            Http(ref mut stream) => stream.close(how),
            #[cfg(feature = "tls")] Https(ref mut stream) => stream.close(how),
            #[cfg(any(unix, windows))] UnixHttp(ref mut stream) => stream.close(how),
            #[cfg(all(any(unix, windows), feature = "tls"))] UnixHttps(ref mut s) => s.close(how),
            Empty => Ok(()),
        }
    }
}

#[inline]
#[cfg(all(any(unix, windows), not(feature = "tls")))]
crate fn try_socket_stream_downcast(stream: &mut dyn NetworkStream) -> Option<NetStream> {
    stream.downcast_ref::<UnixSocketStream>()
        .map(|s| NetStream::UnixHttp(s.clone()))
}

#[inline]
#[cfg(all(any(unix, windows), feature = "tls"))]
crate fn try_socket_stream_downcast(stream: &mut dyn NetworkStream) -> Option<NetStream> {
    stream.downcast_ref::<UnixHttpsStream>()
        .map(|s| NetStream::UnixHttps(s.clone()))
        .or_else(|| {
            stream.downcast_ref::<UnixSocketStream>()
                .map(|s| NetStream::UnixHttp(s.clone()))
        })
}

#[inline]
#[cfg(not(any(unix, windows)))]
crate fn try_socket_stream_downcast(_stream: &mut dyn NetworkStream) -> Option<NetStream> {
    None
}