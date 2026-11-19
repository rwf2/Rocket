use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures::stream::Stream;
use pin_project_lite::pin_project;
use tokio::io::AsyncRead;

pin_project! {
    /// Convert an [`AsyncRead`] into a [`Stream`] of byte chunks.
    ///
    /// This stream is fused. It performs the inverse operation of
    /// [`StreamReader`].
    ///
    /// # Example
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// use tokio_stream::StreamExt;
    /// use tokio_util::io::ReaderStream;
    ///
    /// // Create a stream of data.
    /// let data = b"hello, world!";
    /// let mut stream = ReaderStream::new(&data[..]);
    ///
    /// // Read all of the chunks into a vector.
    /// let mut stream_contents = Vec::new();
    /// while let Some(chunk) = stream.next().await {
    ///    stream_contents.extend_from_slice(&chunk?);
    /// }
    ///
    /// // Once the chunks are concatenated, we should have the
    /// // original data.
    /// assert_eq!(stream_contents, data);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`AsyncRead`]: tokio::io::AsyncRead
    /// [`StreamReader`]: crate::io::StreamReader
    /// [`Stream`]: futures_core::Stream
    #[derive(Debug)]
    pub struct ReaderStream<R> {
        // Reader itself.
        //
        // This value is `None` if the stream has terminated.
        #[pin]
        reader: R,
        // Working buffer, used to optimize allocations.
        buf: BytesMut,
        capacity: usize,
        done: bool,
    }
}

impl<R: AsyncRead> ReaderStream<R> {
    /// Convert an [`AsyncRead`] into a [`Stream`] with item type
    /// `Result<Bytes, std::io::Error>`,
    /// with a specific read buffer initial capacity.
    ///
    /// [`AsyncRead`]: tokio::io::AsyncRead
    /// [`Stream`]: futures_core::Stream
    pub fn with_capacity(reader: R, capacity: usize) -> Self {
        ReaderStream {
            reader,
            buf: BytesMut::with_capacity(capacity),
            capacity,
            done: false,
        }
    }

    /// Creates a stream that ends immediately without reading from `reader`.
    pub fn empty(reader: R) -> Self {
        ReaderStream { reader, buf: BytesMut::new(), capacity: 0, done: true }
    }
}

impl<R: AsyncRead> Stream for ReaderStream<R> {
    type Item = std::io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use tokio_util::io::poll_read_buf;

        let mut this = self.as_mut().project();

        if *this.done {
            return Poll::Ready(None);
        }

        if this.buf.capacity() == 0 {
            this.buf.reserve(*this.capacity);
        }

        let reader = this.reader;
        match poll_read_buf(reader, cx, &mut this.buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(err)) => {
                *this.done = true;
                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(Ok(0)) => {
                *this.done = true;
                Poll::Ready(None)
            }
            Poll::Ready(Ok(_)) => {
                let chunk = this.buf.split();
                Poll::Ready(Some(Ok(chunk.freeze())))
            }
        }
    }

    // fn size_hint(&self) -> (usize, Option<usize>) {
    //     self.reader.size_hint()
    // }
}

impl<R: AsyncRead> hyper::body::Body for ReaderStream<R> {
    type Data = bytes::Bytes;

    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        self.poll_next(cx).map_ok(hyper::body::Frame::data)
    }

    fn is_end_stream(&self) -> bool {
        self.done
    }
}
