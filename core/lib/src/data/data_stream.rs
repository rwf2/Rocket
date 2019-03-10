use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::AsyncRead;

// TODO.async: Consider storing the real type here instead of a Box to avoid
// the dynamic dispatch
/// Raw data stream of a request body.
///
/// This stream can only be obtained by calling
/// [`Data::open()`](crate::data::Data::open()). The stream contains all of the data
/// in the body of the request. It exposes no methods directly. Instead, it must
/// be used as an opaque [`Read`] structure.
pub struct DataStream(pub(crate) Vec<u8>, pub(crate) Box<dyn AsyncRead + Unpin + Send>);

// TODO.async: Consider implementing `AsyncBufRead`

// TODO: Have a `BufRead` impl for `DataStream`. At the moment, this isn't
// possible since Hyper's `HttpReader` doesn't implement `BufRead`.
impl AsyncRead for DataStream {
    #[inline(always)]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, std::io::Error>> {
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
