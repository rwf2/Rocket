use std::io;
use std::net::{SocketAddr, Shutdown};
use std::time::Duration;

#[cfg(feature = "tls")] use crate::http::tls::{WrappedStream, ServerSession};
use crate::http::hyper::net::{HttpStream, NetworkStream};

use self::NetStream::*;

#[cfg(feature = "tls")] pub type HttpsStream = WrappedStream<ServerSession>;

// This is a representation of all of the possible network streams we might get.
// This really shouldn't be necessary, but, you know, Hyper.
#[derive(Clone)]
pub enum NetStream {
    Http(HttpStream),
    #[cfg(feature = "tls")]
    Https(HttpsStream),
    Empty,
}

impl io::Read for NetStream {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        trace_!("NetStream::read()");
        let res = match *self {
            Http(ref mut stream) => stream.read(buf),
            #[cfg(feature = "tls")] Https(ref mut stream) => stream.read(buf),
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
            Empty => Ok(0),
        }
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        match *self {
            Http(ref mut stream) => stream.flush(),
            #[cfg(feature = "tls")] Https(ref mut stream) => stream.flush(),
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
            Empty => Err(io::Error::from(io::ErrorKind::AddrNotAvailable)),
        }
    }

    #[inline(always)]
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        match *self {
            Http(ref stream) => stream.set_read_timeout(dur),
            #[cfg(feature = "tls")] Https(ref stream) => stream.set_read_timeout(dur),
            Empty => Ok(()),
        }
    }

    #[inline(always)]
    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        match *self {
            Http(ref stream) => stream.set_write_timeout(dur),
            #[cfg(feature = "tls")] Https(ref stream) => stream.set_write_timeout(dur),
            Empty => Ok(()),
        }
    }

    #[inline(always)]
    fn close(&mut self, how: Shutdown) -> io::Result<()> {
        match *self {
            Http(ref mut stream) => stream.close(how),
            #[cfg(feature = "tls")] Https(ref mut stream) => stream.close(how),
            Empty => Ok(()),
        }
    }
}
