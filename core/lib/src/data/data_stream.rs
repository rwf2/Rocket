use std::pin::Pin;
use std::task::{Context, Poll};
use std::path::Path;
use std::io::{self, Cursor};

use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, ReadBuf, Take};

use crate::ext::AsyncReadBody;

/// Raw data stream of a request body.
///
/// This stream can only be obtained by calling
/// [`Data::open()`](crate::data::Data::open()). The stream contains all of the
/// data in the body of the request. It exposes no methods directly. Instead, it
/// must be used as an opaque [`AsyncRead`] structure.
pub struct DataStream {
    pub(crate) buffer: Take<Cursor<Vec<u8>>>,
    pub(crate) stream: Take<AsyncReadBody>
}

impl DataStream {
    /// A helper method to write the body of the request to any `AsyncWrite`
    /// type.
    ///
    /// This method is identical to `tokio::io::copy(&mut self, &mut writer)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn handler(mut data: Data) -> io::Result<String> {
    ///     // write all of the data to stdout
    ///     let written = data.open(512.kibibytes()).stream_to(tokio::io::stdout()).await?;
    ///     Ok(format!("Wrote {} bytes.", written))
    /// }
    /// ```
    #[inline(always)]
    pub async fn stream_to<W>(mut self, mut writer: W) -> io::Result<u64>
        where W: AsyncWrite + Unpin
    {
        tokio::io::copy(&mut self, &mut writer).await
    }

    /// A helper method to write the body of the request to a file at the path
    /// determined by `path`.
    ///
    /// This method is identical to `self.stream_to(&mut
    /// File::create(path).await?)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn handler(mut data: Data) -> io::Result<String> {
    ///     let written = data.open(1.megabytes()).stream_to_file("/static/file").await?;
    ///     Ok(format!("Wrote {} bytes to /static/file", written))
    /// }
    /// ```
    #[inline(always)]
    pub async fn stream_to_file<P: AsRef<Path>>(self, path: P) -> io::Result<u64> {
        let mut file = tokio::fs::File::create(path).await?;
        self.stream_to(&mut file).await
    }

    /// A helper method to write the body of the request to a `String`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn handler(data: Data) -> io::Result<String> {
    ///     data.open(10.bytes()).stream_to_string().await
    /// }
    /// ```
    pub async fn stream_to_string(mut self) -> io::Result<String> {
        let buf_len = self.buffer.get_ref().get_ref().len();
        let max_from_buf = std::cmp::min(buf_len, self.buffer.limit() as usize);
        let capacity = std::cmp::min(max_from_buf, 1024);
        let mut string = String::with_capacity(capacity);
        self.read_to_string(&mut string).await?;
        Ok(string)
    }

    /// A helper method to write the body of the request to a `Vec<u8>`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn handler(data: Data) -> io::Result<Vec<u8>> {
    ///     data.open(4.kibibytes()).stream_to_vec().await
    /// }
    /// ```
    pub async fn stream_to_vec(mut self) -> io::Result<Vec<u8>> {
        let buf_len = self.buffer.get_ref().get_ref().len();
        let max_from_buf = std::cmp::min(buf_len, self.buffer.limit() as usize);
        let capacity = std::cmp::min(max_from_buf, 1024);
        let mut vec = Vec::with_capacity(capacity);
        self.read_to_end(&mut vec).await?;
        Ok(vec)
    }
}

// TODO.async: Consider implementing `AsyncBufRead`.

impl AsyncRead for DataStream {
    #[inline(always)]
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.buffer.limit() > 0 {
            trace_!("DataStream::buffer_read()");
            match Pin::new(&mut self.buffer).poll_read(cx, buf) {
                Poll::Ready(Ok(())) if buf.filled().is_empty() => { /* fall through */ },
                poll => return poll,
            }
        }

        trace_!("DataStream::stream_read()");
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}
