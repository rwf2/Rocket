use std::io::{self, Cursor};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::ready;
use futures::stream::Stream;
use hyper::body::{Body, Bytes, Incoming as HyperBody};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf, Take};
use tokio_util::io::StreamReader;

use crate::data::transform::Transform;
use crate::data::{Capped, N};
use crate::util::Chain;

use super::peekable::Peekable;
use super::transform::TransformBuf;

/// Raw data stream of a request body.
///
/// This stream can only be obtained by calling
/// [`Data::open()`](crate::data::Data::open()) with a data limit. The stream
/// contains all of the data in the body of the request.
///
/// Reading from a `DataStream` is accomplished via the various methods on the
/// structure. In general, methods exists in two variants: those that _check_
/// whether the entire stream was read and those that don't. The former either
/// directly or indirectly (via [`Capped`]) return an [`N`] which allows
/// checking if the stream was read to completion while the latter do not.
///
/// | Read Into | Method                               | Notes                            |
/// |-----------|--------------------------------------|----------------------------------|
/// | `String`  | [`DataStream::into_string()`]        | Completeness checked. Preferred. |
/// | `String`  | [`AsyncReadExt::read_to_string()`]   | Unchecked w/existing `String`.   |
/// | `Vec<u8>` | [`DataStream::into_bytes()`]         | Checked. Preferred.              |
/// | `Vec<u8>` | [`DataStream::stream_to(&mut vec)`]  | Checked w/existing `Vec`.        |
/// | `Vec<u8>` | [`DataStream::stream_precise_to()`]  | Unchecked w/existing `Vec`.      |
/// | `File`    | [`DataStream::into_file()`]          | Checked. Preferred.              |
/// | `File`    | [`DataStream::stream_to(&mut file)`] | Checked w/ existing `File`.      |
/// | `File`    | [`DataStream::stream_precise_to()`]  | Unchecked w/ existing `File`.    |
/// | `T`       | [`DataStream::stream_to()`]          | Checked. Any `T: AsyncWrite`.    |
/// | `T`       | [`DataStream::stream_precise_to()`]  | Unchecked. Any `T: AsyncWrite`.  |
///
/// [`DataStream::stream_to(&mut vec)`]: DataStream::stream_to()
/// [`DataStream::stream_to(&mut file)`]: DataStream::stream_to()
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
pub enum DataStream<'r> {
    #[doc(hidden)]
    Base(BaseReader<'r>),
    #[doc(hidden)]
    Transform(TransformReader<'r>),
}

/// A data stream that has a `transformer` applied to it.
pub struct TransformReader<'r> {
    transformer: Pin<Box<dyn Transform + Send + Sync + 'r>>,
    stream: Pin<Box<DataStream<'r>>>,
    inner_done: bool,
}

/// Limited, pre-buffered reader to the underlying data stream.
pub type BaseReader<'r> = Take<Chain<Cursor<Vec<u8>>, RawReader<'r>>>;

/// Direct reader to the underlying data stream. Not limited in any manner.
pub type RawReader<'r> = StreamReader<RawStream<'r>, Bytes>;

/// Raw underlying data stream.
#[allow(clippy::large_enum_variant)]
pub enum RawStream<'r> {
    Empty,
    Body(HyperBody),
    #[cfg(feature = "http3-preview")]
    H3Body(crate::listener::Cancellable<crate::listener::quic::QuicRx>),
    Multipart(multer::Field<'r>),
}

impl<'r> TransformReader<'r> {
    /// Returns the underlying `BaseReader`.
    fn base_mut(&mut self) -> &mut BaseReader<'r> {
        match self.stream.as_mut().get_mut() {
            DataStream::Base(base) => base,
            DataStream::Transform(inner) => inner.base_mut(),
        }
    }

    /// Returns the underlying `BaseReader`.
    fn base(&self) -> &BaseReader<'r> {
        match self.stream.as_ref().get_ref() {
            DataStream::Base(base) => base,
            DataStream::Transform(inner) => inner.base(),
        }
    }
}

impl<'r> DataStream<'r> {
    pub(crate) fn new(
        transformers: Vec<Pin<Box<dyn Transform + Send + Sync + 'r>>>,
        Peekable { buffer, reader, .. }: Peekable<512, RawReader<'r>>,
        limit: u64,
    ) -> Self {
        let mut stream = DataStream::Base(Chain::new(Cursor::new(buffer), reader).take(limit));
        for transformer in transformers {
            stream = DataStream::Transform(TransformReader {
                transformer,
                stream: Box::pin(stream),
                inner_done: false,
            });
        }

        stream
    }

    /// Returns the underlying `BaseReader`.
    fn base_mut(&mut self) -> &mut BaseReader<'r> {
        match self {
            DataStream::Base(base) => base,
            DataStream::Transform(transform) => transform.base_mut(),
        }
    }

    /// Returns the underlying `BaseReader`.
    fn base(&self) -> &BaseReader<'r> {
        match self {
            DataStream::Base(base) => base,
            DataStream::Transform(transform) => transform.base(),
        }
    }

    /// Whether a previous read exhausted the set limit _and then some_.
    async fn limit_exceeded(&mut self) -> io::Result<bool> {
        let base = self.base_mut();

        #[cold]
        async fn _limit_exceeded(base: &mut BaseReader<'_>) -> io::Result<bool> {
            // Read one more byte after reaching limit to see if we cut early.
            base.set_limit(1);
            let mut buf = [0u8; 1];
            let exceeded = base.read(&mut buf).await? != 0;
            base.set_limit(0);
            Ok(exceeded)
        }

        Ok(base.limit() == 0 && _limit_exceeded(base).await?)
    }

    /// Number of bytes a full read from `self` will _definitely_ read.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket_community as rocket;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn f(data: Data<'_>) {
    ///     let definitely_have_n_bytes = data.open(1.kibibytes()).hint();
    /// }
    /// ```
    pub fn hint(&self) -> usize {
        let base = self.base();
        if let (Some(cursor), _) = base.get_ref().get_ref() {
            let len = cursor.get_ref().len() as u64;
            let position = cursor.position().min(len);
            let remaining = len - position;
            remaining.min(base.limit()) as usize
        } else {
            0
        }
    }

    /// A helper method to write the body of the request to any `AsyncWrite`
    /// type. Returns an [`N`] which indicates how many bytes were written and
    /// whether the entire stream was read. An additional read from `self` may
    /// be required to check if all of the stream has been read. If that
    /// information is not needed, use [`DataStream::stream_precise_to()`].
    ///
    /// This method is identical to `tokio::io::copy(&mut self, &mut writer)`
    /// except in that it returns an `N` to check for completeness.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket_community as rocket;
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn data_guard(mut data: Data<'_>) -> io::Result<String> {
    ///     // write all of the data to stdout
    ///     let written = data.open(512.kibibytes())
    ///         .stream_to(tokio::io::stdout()).await?;
    ///
    ///     Ok(format!("Wrote {} bytes.", written))
    /// }
    /// ```
    #[inline(always)]
    pub async fn stream_to<W>(mut self, mut writer: W) -> io::Result<N>
    where
        W: AsyncWrite + Unpin,
    {
        let written = tokio::io::copy(&mut self, &mut writer).await?;
        Ok(N {
            written,
            complete: !self.limit_exceeded().await?,
        })
    }

    /// Like [`DataStream::stream_to()`] except that no end-of-stream check is
    /// conducted and thus read/write completeness is unknown.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket_community as rocket;
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn data_guard(mut data: Data<'_>) -> io::Result<String> {
    ///     // write all of the data to stdout
    ///     let written = data.open(512.kibibytes())
    ///         .stream_precise_to(tokio::io::stdout()).await?;
    ///
    ///     Ok(format!("Wrote {} bytes.", written))
    /// }
    /// ```
    #[inline(always)]
    pub async fn stream_precise_to<W>(mut self, mut writer: W) -> io::Result<u64>
    where
        W: AsyncWrite + Unpin,
    {
        tokio::io::copy(&mut self, &mut writer).await
    }

    /// A helper method to write the body of the request to a `Vec<u8>`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket_community as rocket;
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn data_guard(data: Data<'_>) -> io::Result<Vec<u8>> {
    ///     let bytes = data.open(4.kibibytes()).into_bytes().await?;
    ///     if !bytes.is_complete() {
    ///         println!("there are bytes remaining in the stream");
    ///     }
    ///
    ///     Ok(bytes.into_inner())
    /// }
    /// ```
    pub async fn into_bytes(self) -> io::Result<Capped<Vec<u8>>> {
        let mut vec = Vec::with_capacity(self.hint());
        let n = self.stream_to(&mut vec).await?;
        Ok(Capped { value: vec, n })
    }

    /// A helper method to write the body of the request to a `String`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket_community as rocket;
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn data_guard(data: Data<'_>) -> io::Result<String> {
    ///     let string = data.open(10.bytes()).into_string().await?;
    ///     if !string.is_complete() {
    ///         println!("there are bytes remaining in the stream");
    ///     }
    ///
    ///     Ok(string.into_inner())
    /// }
    /// ```
    pub async fn into_string(mut self) -> io::Result<Capped<String>> {
        let mut string = String::with_capacity(self.hint());
        let written = self.read_to_string(&mut string).await?;
        let n = N {
            written: written as u64,
            complete: !self.limit_exceeded().await?,
        };
        Ok(Capped { value: string, n })
    }

    /// A helper method to write the body of the request to a file at the path
    /// determined by `path`. If a file at the path already exists, it is
    /// overwritten. The opened file is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket_community as rocket;
    /// use std::io;
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// async fn data_guard(mut data: Data<'_>) -> io::Result<String> {
    ///     let file = data.open(1.megabytes()).into_file("/static/file").await?;
    ///     if !file.is_complete() {
    ///         println!("there are bytes remaining in the stream");
    ///     }
    ///
    ///     Ok(format!("Wrote {} bytes to /static/file", file.n))
    /// }
    /// ```
    pub async fn into_file<P: AsRef<Path>>(self, path: P) -> io::Result<Capped<File>> {
        let mut file = File::create(path).await?;
        let n = self
            .stream_to(&mut tokio::io::BufWriter::new(&mut file))
            .await?;
        Ok(Capped { value: file, n })
    }
}

impl AsyncRead for DataStream<'_> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            DataStream::Base(inner) => Pin::new(inner).poll_read(cx, buf),
            DataStream::Transform(inner) => Pin::new(inner).poll_read(cx, buf),
        }
    }
}

impl AsyncRead for TransformReader<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let init_fill = buf.filled().len();
        if !self.inner_done {
            ready!(Pin::new(&mut self.stream).poll_read(cx, buf))?;
            self.inner_done = init_fill == buf.filled().len();
        }

        if self.inner_done {
            return self.transformer.as_mut().poll_finish(cx, buf);
        }

        let mut tbuf = TransformBuf {
            buf,
            cursor: init_fill,
        };
        self.transformer.as_mut().transform(&mut tbuf)?;
        if buf.filled().len() == init_fill {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        Poll::Ready(Ok(()))
    }
}

impl Stream for RawStream<'_> {
    type Item = io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            // TODO: Expose trailer headers, somehow.
            RawStream::Body(body) => Pin::new(body)
                .poll_frame(cx)
                .map_ok(|frame| frame.into_data().unwrap_or_else(|_| Bytes::new()))
                .map_err(io::Error::other),
            #[cfg(feature = "http3-preview")]
            RawStream::H3Body(stream) => Pin::new(stream).poll_next(cx),
            RawStream::Multipart(s) => Pin::new(s).poll_next(cx).map_err(io::Error::other),
            RawStream::Empty => Poll::Ready(None),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            RawStream::Body(body) => {
                let hint = body.size_hint();
                let (lower, upper) = (hint.lower(), hint.upper());
                (lower as usize, upper.map(|x| x as usize))
            }
            #[cfg(feature = "http3-preview")]
            RawStream::H3Body(_) => (0, Some(0)),
            RawStream::Multipart(mp) => mp.size_hint(),
            RawStream::Empty => (0, Some(0)),
        }
    }
}

impl std::fmt::Display for RawStream<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RawStream::Empty => f.write_str("empty stream"),
            RawStream::Body(_) => f.write_str("request body"),
            #[cfg(feature = "http3-preview")]
            RawStream::H3Body(_) => f.write_str("http3 quic stream"),
            RawStream::Multipart(_) => f.write_str("multipart form field"),
        }
    }
}

impl<'r> From<HyperBody> for RawStream<'r> {
    fn from(value: HyperBody) -> Self {
        Self::Body(value)
    }
}

#[cfg(feature = "http3-preview")]
impl<'r> From<crate::listener::Cancellable<crate::listener::quic::QuicRx>> for RawStream<'r> {
    fn from(value: crate::listener::Cancellable<crate::listener::quic::QuicRx>) -> Self {
        Self::H3Body(value)
    }
}

impl<'r> From<multer::Field<'r>> for RawStream<'r> {
    fn from(value: multer::Field<'r>) -> Self {
        Self::Multipart(value)
    }
}
