use std::io::{self, Read, Cursor, Chain};
use std::net::Shutdown;

use super::data::BodyReader;
use crate::http::hyper::net::NetworkStream;
use crate::http::hyper::h1::HttpReader;

//                          |-- peek buf --|
pub type InnerStream = Chain<Cursor<Vec<u8>>, BodyReader>;

/// Raw data stream of a request body.
///
/// This stream can only be obtained by calling
/// [`Data::open()`](crate::data::Data::open()). The stream contains all of the data
/// in the body of the request. It exposes no methods directly. Instead, it must
/// be used as an opaque [`Read`] structure.
pub struct DataStream(crate InnerStream);

// TODO: Have a `BufRead` impl for `DataStream`. At the moment, this isn't
// possible since Hyper's `HttpReader` doesn't implement `BufRead`.
impl Read for DataStream {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        trace_!("DataStream::read()");
        self.0.read(buf)
    }
}

pub fn kill_stream(stream: &mut BodyReader) {
    // Only do the expensive reading if we're not sure we're done.
    use self::HttpReader::*;
    match *stream {
        SizedReader(_, n) | ChunkedReader(_, Some(n)) if n > 0 => { /* continue */ },
        _ => return
    };

    // Take <= 1k from the stream. If there might be more data, force close.
    const FLUSH_LEN: u64 = 1024;
    match io::copy(&mut stream.take(FLUSH_LEN), &mut io::sink()) {
        Ok(FLUSH_LEN) | Err(_) => {
            warn_!("Data left unread. Force closing network stream.");
            let (_, network) = stream.get_mut().get_mut();
            if let Err(e) = network.close(Shutdown::Read) {
                error_!("Failed to close network stream: {:?}", e);
            }
        }
        Ok(n) => debug!("flushed {} unread bytes", n)
    }
}

impl Drop for DataStream {
    fn drop(&mut self) {
        kill_stream(&mut self.0.get_mut().1);
    }
}
