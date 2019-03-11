use std::pin::Pin;

use futures::io::{AsyncRead, Error as IoError};
use futures::task::{Poll, Context};

// TODO.async: Consider storing the real type here instead of a Box to avoid
// the dynamic dispatch
/// Raw data stream of a request body.
///
/// This stream can only be obtained by calling
/// [`Data::open()`](crate::data::Data::open()). The stream contains all of the data
/// in the body of the request. It exposes no methods directly. Instead, it must
/// be used as an opaque [`Read`] structure.
pub struct DataStream(crate Vec<u8>, crate Box<dyn AsyncRead + Unpin + Send>);

// TODO.async: Consider implementing `AsyncBufRead`

// TODO: Have a `BufRead` impl for `DataStream`. At the moment, this isn't
// possible since Hyper's `HttpReader` doesn't implement `BufRead`.
impl AsyncRead for DataStream {
    #[inline(always)]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, IoError>> {
        trace_!("DataStream::poll_read()");
        if self.0.len() > 0 {
            let count = std::cmp::min(buf.len(), self.0.len());
            trace_!("Reading peeked {} into dest {} = {} bytes", self.0.len(), buf.len(), count);
            let next = self.0.split_off(count);
            (&mut buf[..count]).copy_from_slice(&self.0[..]);
            self.0 = next;
            Poll::Ready(Ok(count))
        } else {
            trace_!("Delegating to remaining stream");
            Pin::new(&mut self.1).poll_read(cx, buf)
        }
    }
}

// TODO.async: Either implement this somehow, or remove the
// `Drop` impl and other references to kill_stream
pub fn kill_stream(_stream: &mut dyn AsyncRead) {
//    // Only do the expensive reading if we're not sure we're done.
//
//    // Take <= 1k from the stream. If there might be more data, force close.
//    const FLUSH_LEN: u64 = 1024;
//    match io::copy(&mut stream.take(FLUSH_LEN), &mut io::sink()) {
//        Ok(FLUSH_LEN) | Err(_) => {
//            warn_!("Data left unread. Force closing network stream.");
//            let (_, network) = stream.get_mut().get_mut();
//            if let Err(e) = network.close(Shutdown::Read) {
//                error_!("Failed to close network stream: {:?}", e);
//            }
//        }
//        Ok(n) => debug!("flushed {} unread bytes", n)
//    }
}

impl Drop for DataStream {
    fn drop(&mut self) {
        kill_stream(&mut self.1);
    }
}
