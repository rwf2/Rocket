use std::io::{self, Read, Write, Cursor, BufReader};
use std::path::Path;
use std::fs::File;
use std::time::Duration;
use std::mem::transmute;

use super::data_stream::{DataStream, StreamReader, kill_stream};

use ext::ReadExt;

use http::hyper::h1::HttpReader;
use http::hyper::buffer;
use http::hyper::h1::HttpReader::*;
use http::hyper::net::{HttpStream, NetworkStream};

pub type BodyReader<'a, 'b> =
    self::HttpReader<&'a mut self::buffer::BufReader<&'b mut NetworkStream>>;

/// Type representing the data in the body of an incoming request.
///
/// This type is the only means by which the body of a request can be retrieved.
/// This type is not usually used directly. Instead, types that implement
/// [FromData](/rocket/data/trait.FromData.html) are used via code generation by
/// specifying the `data = "<param>"` route parameter as follows:
///
/// ```rust,ignore
/// #[post("/submit", data = "<var>")]
/// fn submit(var: T) -> ... { ... }
/// ```
///
/// Above, `T` can be any type that implements `FromData`. Note that `Data`
/// itself implements `FromData`.
///
/// # Reading Data
///
/// Data may be read from a `Data` object by calling either the
/// [open](#method.open) or [peek](#method.peek) methods.
///
/// The `open` method consumes the `Data` object and returns the raw data
/// stream. The `Data` object is consumed for safety reasons: consuming the
/// object ensures that holding a `Data` object means that all of the data is
/// available for reading.
///
/// The `peek` method returns a slice containing at most 4096 bytes of buffered
/// body data. This enables partially or fully reading from a `Data` object
/// without consuming the `Data` object.
pub struct Data {
    buffer: Vec<u8>,
    is_done: bool,
    // TODO: This sucks as it depends on a TCPStream. Oh, hyper.
    stream: StreamReader,
    // Ideally we wouldn't have these, but Hyper forces us to.
    position: usize,
    capacity: usize,
}

impl Data {
    /// Returns the raw data stream.
    ///
    /// The stream contains all of the data in the body of the request,
    /// including that in the `peek` buffer. The method consumes the `Data`
    /// instance. This ensures that a `Data` type _always_ represents _all_ of
    /// the data in a request.
    pub fn open(mut self) -> DataStream {
        // Swap out the buffer and stream for empty ones so we can move.
        let mut buffer = vec![];
        let mut stream = EmptyReader(self.stream.get_ref().clone());
        ::std::mem::swap(&mut buffer, &mut self.buffer);
        ::std::mem::swap(&mut stream, &mut self.stream);

        // Setup the underlying reader at the correct pointers.
        let mut cursor = Cursor::new(buffer);
        cursor.set_position(self.position as u64);

        // Get a reference to the underlying network stream.
        let network = stream.get_ref().clone();

        // The first part of the stream is the buffer. Then the real steam.
        let buf = cursor.take((self.capacity - self.position) as u64);
        let stream = buf.chain(BufReader::new(stream));

        DataStream::new(stream, network)
    }

    pub(crate) fn from_hyp(mut h_body: BodyReader) -> Result<Data, &'static str> {
        // FIXME: This is asolutely terrible, thanks to Hyper.

        // Retrieve the underlying HTTPStream from Hyper.
        let mut stream = match h_body.get_ref().get_ref()
                                     .downcast_ref::<HttpStream>() {
            Some(s) => {
                let owned_stream = s.clone();
                let buf_len = h_body.get_ref().get_buf().len() as u64;
                match h_body {
                    SizedReader(_, n) => SizedReader(owned_stream, n - buf_len),
                    EofReader(_) => EofReader(owned_stream),
                    EmptyReader(_) => EmptyReader(owned_stream),
                    ChunkedReader(_, n) =>
                        ChunkedReader(owned_stream, n.map(|k| k - buf_len)),
                }
            },
            None => return Err("Stream is not an HTTP stream!"),
        };

        // Set the read timeout to 5 seconds.
        stream.get_mut().set_read_timeout(Some(Duration::from_secs(5))).unwrap();

        // Create the Data object from hyper's buffer.
        let (vec, pos, cap) = h_body.get_mut().take_buf();
        Ok(Data::new(vec, pos, cap, stream))
    }

    /// Retrieve the `peek` buffer.
    ///
    /// The peek buffer contains at most 4096 bytes of the body of the request.
    /// The actual size of the returned buffer varies by web request. The
    /// [peek_complete](#method.peek_complete) can be used to determine if this
    /// buffer contains _all_ of the data in the body of the request.
    #[inline(always)]
    pub fn peek(&self) -> &[u8] {
        &self.buffer[self.position..self.capacity]
    }

    /// Returns true if the `peek` buffer contains all of the data in the body
    /// of the request. Returns `false` if it does not or if it is not known if
    /// it does.
    #[inline(always)]
    pub fn peek_complete(&self) -> bool {
        self.is_done
    }

    /// A helper method to write the body of the request to any `Write` type.
    ///
    /// This method is identical to `io::copy(&mut data.open(), writer)`.
    #[inline(always)]
    pub fn stream_to<W: Write>(self, writer: &mut W) -> io::Result<u64> {
        io::copy(&mut self.open(), writer)
    }

    /// A helper method to write the body of the request to a file at the path
    /// determined by `path`.
    ///
    /// This method is identical to
    /// `io::copy(&mut self.open(), &mut File::create(path)?)`.
    #[inline(always)]
    pub fn stream_to_file<P: AsRef<Path>>(self, path: P) -> io::Result<u64> {
        io::copy(&mut self.open(), &mut File::create(path)?)
    }

    // Creates a new data object with an internal buffer `buf`, where the cursor
    // in the buffer is at `pos` and the buffer has `cap` valid bytes. The
    // remainder of the data bytes can be read from `stream`.
    pub(crate) fn new(mut buf: Vec<u8>,
               pos: usize,
               mut cap: usize,
               mut stream: StreamReader)
               -> Data {
        // Make sure the buffer is large enough for the bytes we want to peek.
        const PEEK_BYTES: usize = 4096;
        if buf.len() < PEEK_BYTES {
            trace_!("Resizing peek buffer from {} to {}.", buf.len(), PEEK_BYTES);
            buf.resize(PEEK_BYTES, 0);
        }

        // Fill the buffer with as many bytes as possible. If we read less than
        // that buffer's length, we know we reached the EOF. Otherwise, it's
        // unclear, so we just say we didn't reach EOF.
        trace!("Init buffer cap: {}", cap);
        let eof = match stream.read_max(&mut buf[cap..]) {
            Ok(n) => {
                trace_!("Filled peek buf with {} bytes.", n);
                cap += n;
                cap < buf.len()
            }
            Err(e) => {
                error_!("Failed to read into peek buffer: {:?}.", e);
                false
            },
        };

        trace_!("Peek buffer size: {}, remaining: {}", buf.len(), buf.len() - cap);
        Data {
            buffer: buf,
            stream: stream,
            is_done: eof,
            position: pos,
            capacity: cap,
        }
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        // This is okay since the network stream expects to be shared mutably.
        unsafe {
            let stream: &mut StreamReader = transmute(self.stream.by_ref());
            kill_stream(stream, self.stream.get_mut());
        }
    }
}

