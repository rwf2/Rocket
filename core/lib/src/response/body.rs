use std::{io, fmt};
use std::task::{Context, Poll};
use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, ReadBuf};

/// The body of a [`Response`].
///
/// A `Body` is never created directly, but instead, through the following
/// methods on `Response` and `Builder`:
///
///   * [`Builder::sized_body()`]
///   * [`Response::set_sized_body()`]
///   * [`Builder::streamed_body()`]
///   * [`Response::set_streamed_body()`]
///
/// [`Response`]: crate::Response
/// [`Builder`]: crate::response::Builder
/// [`Response::set_sized_body()`]: crate::Response::set_sized_body
/// [`Response::set_streamed_body()`]: crate::Response::set_streamed_body
/// [`Builder::sized_body()`]: crate::response::Builder::sized_body
/// [`Builder::streamed_body()`]: crate::response::Builder::streamed_body
///
/// An unset body in a `Response` begins as a [`Body::default()`], a `None`
/// body with a preset size of `0`.
///
/// # Sizing
///
/// A response body may be sized or unsized ("streamed"). A "sized" body is
/// transferred with a `Content-Length` equal to its size while an "unsized"
/// body is chunk-encoded. The body data is streamed in _all_ cases and is never
/// buffered in memory beyond a minimal amount for efficient transmission.
///
/// ## Sized
///
/// A sized body may have a _preset_ size ([`Body::preset_size()`]) or may have
/// its size computed on the fly by seeking ([`Body::size()`]). As such, sized
/// bodies must implement [`AsyncSeek`]. If a body does not have a preset size
/// and the fails to be computed dynamically, a sized body is treated as an
/// unsized body when written out to the network.
///
/// ## Unsized
///
/// An unsized body's data is streamed as it arrives. In other words, as soon as
/// the body's [`AsyncRead`] implementation returns bytes, the bytes are written
/// to the network. Individual unsized bodies may use an internal buffer to
/// curtail writes to the network.
///
/// The maximum number of bytes written to the network at once is controlled via
/// the [`Body::max_chunk_size()`] parameter which can be set via
/// [`Response::set_max_chunk_size()`] and [`Builder::max_chunk_size()`].
///
/// [`Response::set_max_chunk_size()`]: crate::Response::set_max_chunk_size
/// [`Builder::max_chunk_size()`]: crate::response::Builder::max_chunk_size
///
/// # Reading
///
/// The contents of a body, decoded, can be read through [`Body::to_bytes()`],
/// [`Body::to_string()`], or directly though `Body`'s [`AsyncRead`]
/// implementation.
#[derive(Debug)]
pub struct Body<'r> {
    /// The size of the body, if it is known.
    size: Option<usize>,
    /// The body itself.
    inner: Inner<'r>,
    /// The maximum chunk size.
    max_chunk: usize,
}

/// A "trait alias" of sorts so we can use `AsyncRead + AsyncSeek` in `dyn`.
pub trait AsyncReadSeek: AsyncRead + AsyncSeek { }

/// Implemented for all `AsyncRead + AsyncSeek`, of course.
impl<T: AsyncRead + AsyncSeek> AsyncReadSeek for T {  }

/// A pinned `AsyncRead + AsyncSeek` body type.
type SizedBody<'r> = Pin<Box<dyn AsyncReadSeek + Send + 'r>>;

/// A pinned `AsyncRead` (not `AsyncSeek`) body type.
type UnsizedBody<'r> = Pin<Box<dyn AsyncRead + Send + 'r>>;

enum Inner<'r> {
    /// A body that can be seeked to determine it's size.
    Seekable(SizedBody<'r>),
    /// A body that has no known size.
    Unsized(UnsizedBody<'r>),
    /// A body that "exists" but only for metadata calculations.
    Phantom(SizedBody<'r>),
    /// An empty body: no body at all.
    None,
}

impl Default for Body<'_> {
    fn default() -> Self {
        Body {
            size: Some(0),
            inner: Inner::None,
            max_chunk: Body::DEFAULT_MAX_CHUNK,
        }
    }
}

impl<'r> Body<'r> {
    /// The default max size, in bytes, of chunks for streamed responses.
    ///
    /// The present value is `4096`.
    pub const DEFAULT_MAX_CHUNK: usize = 4096;

    pub(crate) fn with_sized<T>(body: T, preset_size: Option<usize>) -> Self
        where T: AsyncReadSeek + Send + 'r
    {
        Body {
            size: preset_size,
            inner: Inner::Seekable(Box::pin(body)),
            max_chunk: Body::DEFAULT_MAX_CHUNK,
        }
    }

    pub(crate) fn with_unsized<T>(body: T) -> Self
        where T: AsyncRead + Send + 'r
    {
        Body {
            size: None,
            inner: Inner::Unsized(Box::pin(body)),
            max_chunk: Body::DEFAULT_MAX_CHUNK,
        }
    }

    pub(crate) fn set_max_chunk_size(&mut self, max_chunk: usize) {
        self.max_chunk = max_chunk;
    }

    pub(crate) fn strip(&mut self) {
        let body = std::mem::take(self);
        *self = match body.inner {
            Inner::Seekable(b) | Inner::Phantom(b) => Body {
                size: body.size,
                inner: Inner::Phantom(b),
                max_chunk: body.max_chunk,
            },
            Inner::Unsized(_) | Inner::None => Body::default()
        };
    }

    /// Returns `true` if the body is `None` or unset, the default.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::Response;
    ///
    /// let r = Response::build().finalize();
    /// assert!(r.body().is_none());
    /// ```
    #[inline(always)]
    pub fn is_none(&self) -> bool {
        matches!(self.inner, Inner::None)
    }

    /// Returns `true` if the body is _not_ `None`, anything other than the
    /// default.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::response::Response;
    ///
    /// let body = "Brewing the best coffee!";
    /// let r = Response::build()
    ///     .sized_body(body.len(), Cursor::new(body))
    ///     .finalize();
    ///
    /// assert!(r.body().is_some());
    /// ```
    #[inline(always)]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// A body's preset size, which may have been computed by a previous call to
    /// [`Body::size()`].
    ///
    /// Unsized bodies _always_ return `None`, while sized bodies return `Some`
    /// if the body size was supplied directly on creation or a call to
    /// [`Body::size()`] successfully computed the size and `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::response::Response;
    ///
    /// # rocket::async_test(async {
    /// let body = "Brewing the best coffee!";
    /// let r = Response::build()
    ///     .sized_body(body.len(), Cursor::new(body))
    ///     .finalize();
    ///
    /// // This will _always_ return `Some`.
    /// assert_eq!(r.body().preset_size(), Some(body.len()));
    ///
    /// let r = Response::build()
    ///     .streamed_body(Cursor::new(body))
    ///     .finalize();
    ///
    /// // This will _never_ return `Some`.
    /// assert_eq!(r.body().preset_size(), None);
    ///
    /// let mut r = Response::build()
    ///     .sized_body(None, Cursor::new(body))
    ///     .finalize();
    ///
    /// // This returns `Some` only after a call to `size()`.
    /// assert_eq!(r.body().preset_size(), None);
    /// assert_eq!(r.body_mut().size().await, Some(body.len()));
    /// assert_eq!(r.body().preset_size(), Some(body.len()));
    /// # });
    /// ```
    pub fn preset_size(&self) -> Option<usize> {
        self.size
    }

    /// Returns the maximum chunk size for chunked transfers.
    ///
    /// If none is explicitly set, defaults to [`Body::DEFAULT_MAX_CHUNK`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::response::{Response, Body};
    ///
    /// let body = "Brewing the best coffee!";
    /// let r = Response::build()
    ///     .sized_body(body.len(), Cursor::new(body))
    ///     .finalize();
    ///
    /// assert_eq!(r.body().max_chunk_size(), Body::DEFAULT_MAX_CHUNK);
    ///
    /// let r = Response::build()
    ///     .sized_body(body.len(), Cursor::new(body))
    ///     .max_chunk_size(1024)
    ///     .finalize();
    ///
    /// assert_eq!(r.body().max_chunk_size(), 1024);
    /// ```
    pub fn max_chunk_size(&self) -> usize {
        self.max_chunk
    }

    /// Attempts to compute the body's size and returns it if the body is sized.
    ///
    /// If the size was preset (see [`Body::preset_size()`]), the value is
    /// returned immediately as `Some`. If the body is unsized or computing the
    /// size fails, returns `None`. Otherwise, the size is computed by seeking,
    /// and the `preset_size` is updated to reflect the known value.
    ///
    /// **Note:** the number of bytes read from the reader and/or written to the
    /// network may differ from the value returned by this method. Some examples
    /// include:
    ///
    ///   * bodies in response to `HEAD` requests are never read or written
    ///   * the client may close the connection before the body is read fully
    ///   * reading the body may fail midway
    ///   * a preset size may differ from the actual body size
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::response::Response;
    ///
    /// # rocket::async_test(async {
    /// let body = "Hello, Rocketeers!";
    /// let mut r = Response::build()
    ///     .sized_body(None, Cursor::new(body))
    ///     .finalize();
    ///
    /// assert_eq!(r.body().preset_size(), None);
    /// assert_eq!(r.body_mut().size().await, Some(body.len()));
    /// assert_eq!(r.body().preset_size(), Some(body.len()));
    /// # });
    /// ```
    pub async fn size(&mut self) -> Option<usize> {
        if let Some(size) = self.size {
            return Some(size);
        }

        if let Inner::Seekable(ref mut body) | Inner::Phantom(ref mut body) = self.inner {
            let pos = body.seek(io::SeekFrom::Current(0)).await.ok()?;
            let end = body.seek(io::SeekFrom::End(0)).await.ok()?;
            body.seek(io::SeekFrom::Start(pos)).await.ok()?;

            let size = end as usize - pos as usize;
            self.size = Some(size);
            return Some(size);
        }

        None
    }

    /// Moves the body out of `self` and returns it, leaving a
    /// [`Body::default()`] in its place.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::response::Response;
    ///
    /// let mut r = Response::build()
    ///     .sized_body(None, Cursor::new("Hi"))
    ///     .finalize();
    ///
    /// assert!(r.body().is_some());
    ///
    /// let body = r.body_mut().take();
    /// assert!(body.is_some());
    /// assert!(r.body().is_none());
    /// ```
    #[inline(always)]
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    /// Reads all of `self` into a vector of bytes, consuming the contents.
    ///
    /// If reading fails, returns `Err`. Otherwise, returns `Ok`. Calling this
    /// method may partially or fully consume the body's content. As such,
    /// subsequent calls to `to_bytes()` will likely return different result.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::response::Response;
    ///
    /// # let ok: io::Result<()> = rocket::async_test(async {
    /// let mut r = Response::build()
    ///     .streamed_body(io::Cursor::new(&[1, 2, 3, 11, 13, 17]))
    ///     .finalize();
    ///
    /// assert_eq!(r.body_mut().to_bytes().await?, &[1, 2, 3, 11, 13, 17]);
    /// # Ok(())
    /// # });
    /// # assert!(ok.is_ok());
    /// ```
    pub async fn to_bytes(&mut self) -> io::Result<Vec<u8>> {
        let mut vec = Vec::new();
        let n = match self.read_to_end(&mut vec).await {
            Ok(n) => n,
            Err(e) => {
                error_!("Error reading body: {:?}", e);
                return Err(e);
            }
        };

        if let Some(ref mut size) = self.size {
            *size = size.checked_sub(n).unwrap_or(0);
        }

        Ok(vec)
    }

    /// Reads all of `self` into a string, consuming the contents.
    ///
    /// If reading fails, or the body contains invalid UTF-8 characters, returns
    /// `Err`. Otherwise, returns `Ok`. Calling this method may partially or
    /// fully consume the body's content. As such, subsequent calls to
    /// `to_string()` will likely return different result.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::response::Response;
    ///
    /// # let ok: io::Result<()> = rocket::async_test(async {
    /// let mut r = Response::build()
    ///     .streamed_body(io::Cursor::new("Hello, Rocketeers!"))
    ///     .finalize();
    ///
    /// assert_eq!(r.body_mut().to_string().await?, "Hello, Rocketeers!");
    /// # Ok(())
    /// # });
    /// # assert!(ok.is_ok());
    /// ```
    pub async fn to_string(&mut self) -> io::Result<String> {
        String::from_utf8(self.to_bytes().await?).map_err(|e| {
            error_!("Body is invalid UTF-8: {}", e);
            io::Error::new(io::ErrorKind::InvalidData, e)
        })
    }
}

impl AsyncRead for Body<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let reader = match self.inner {
            Inner::Seekable(ref mut b) => b as &mut (dyn AsyncRead + Unpin),
            Inner::Unsized(ref mut b) => b as &mut (dyn AsyncRead + Unpin),
            Inner::Phantom(_) | Inner::None => return Poll::Ready(Ok(())),
        };

        Pin::new(reader).poll_read(cx, buf)
    }
}

impl fmt::Debug for Inner<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inner::Seekable(_) => "seekable".fmt(f),
            Inner::Unsized(_) => "unsized".fmt(f),
            Inner::Phantom(_) => "phantom".fmt(f),
            Inner::None => "none".fmt(f),
        }
    }
}
