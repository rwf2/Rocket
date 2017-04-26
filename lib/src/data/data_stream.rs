use std::io::{self, Read, Cursor, Chain};

use super::data::BodyReader;

// It's very unfortunate that we have to wrap `BodyReader` in a `BufReader`
// since it already contains another `BufReader`. The issue is that Hyper's
// `HttpReader` doesn't implement `BufRead`. Unfortunately, this will likely
// stay "double buffered" until we switch HTTP libraries.
//                          |-- peek buf --|
// pub type InnerStream = Chain<Cursor<Vec<u8>>, BufReader<BodyReader>>;
pub type InnerStream = Chain<Cursor<Vec<u8>>, BodyReader>;

/// Raw data stream of a request body.
///
/// This stream can only be obtained by calling
/// [Data::open](/rocket/data/struct.Data.html#method.open). The stream contains
/// all of the data in the body of the request. It exposes no methods directly.
/// Instead, it must be used as an opaque `Read` or `BufRead` structure.
pub struct DataStream(pub(crate) InnerStream);

impl Read for DataStream {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        trace_!("DataStream::read()");
        self.0.read(buf)
    }
}

// impl BufRead for DataStream {
//     #[inline(always)]
//     fn fill_buf(&mut self) -> io::Result<&[u8]> {
//         self.0.fill_buf()
//     }

//     #[inline(always)]
//     fn consume(&mut self, amt: usize) {
//         self.0.consume(amt)
//     }
// }

// impl Drop for DataStream {
//     fn drop(&mut self) {
//         // FIXME: Do a read; if > 1024, kill the stream. Need access to the
//         // internals of `Chain` to do this efficiently/without crazy baggage.
//         // https://github.com/rust-lang/rust/pull/41463
//         let _ = io::copy(&mut self.0, &mut io::sink());
//     }
// }
