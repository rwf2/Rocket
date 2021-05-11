//! Potentially infinite async [`Stream`] response types.
//!
//! A [`Stream<Item = T>`] is the async analog of an `Iterator<Item = T>`: it
//! generates a sequence of values asynchronously, otherwise known as an async
//! _generator_. Types in this module allow for returning responses that are
//! streams.
//!
//! [`Stream<Item = T>`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
//! [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
//!
//! # Raw Streams
//!
//! Rust does not yet natively support syntax for creating arbitrary generators,
//! and as such, for creating streams. To ameliorate this, Rocket exports
//! [`stream!`], which retrofit generator syntax, allowing raw `impl Stream`s to
//! be defined using `yield` and `for await` syntax:
//!
//! ```rust
//! use rocket::futures::stream::Stream;
//! use rocket::response::stream::stream;
//!
//! fn make_stream() -> impl Stream<Item = u8> {
//!     stream! {
//!         for i in 0..3 {
//!             yield i;
//!         }
//!     }
//! }
//! ```
//!
//! See [`stream!`] for full usage details.
//!
//! # Typed Streams
//!
//! A raw stream is not a `Responder`, so it cannot be directly returned from a
//! route handler. Instead, one of three _typed_ streams may be used. Each typed
//! stream places type bounds on the `Item` of the stream, allowing for
//! `Responder` implementation on the stream itself.
//!
//! Each typed stream exists both as a type and as a macro. They are:
//!
//!   * [`struct@ReaderStream`] ([`ReaderStream!`]) - streams of `T: AsyncRead`
//!   * [`struct@ByteStream`] ([`ByteStream!`]) - streams of `T: AsRef<[u8]>`
//!   * [`struct@TextStream`] ([`TextStream!`]) - streams of `T: AsRef<str>`
//!
//! Each type implements `Responder`; each macro can be invoked to generate a
//! typed stream, exactly like [`stream!`] above. Additionally, each macro is
//! also a _type_ macro, expanding to a wrapped `impl Stream<Item = $T>`, where
//! `$T` is the input to the macro.
//!
//! As a concrete example, the route below produces an infinite series of
//! `"hello"`s, one per second:
//!
//! ```rust
//! # use rocket::get;
//! use rocket::tokio::time::{self, Duration};
//! use rocket::response::stream::TextStream;
//!
//! /// Produce an infinite series of `"hello"`s, one per second.
//! #[get("/infinite-hellos")]
//! fn hello() -> TextStream![&'static str] {
//!     TextStream! {
//!         let mut interval = time::interval(Duration::from_secs(1));
//!         loop {
//!             yield "hello";
//!             interval.tick().await;
//!         }
//!     }
//! }
//! ```
//!
//! The `TextStream![&'static str]` invocation expands to:
//!
//! ```rust
//! # use rocket::response::stream::TextStream;
//! # use rocket::futures::stream::Stream;
//! # use rocket::response::stream::stream;
//! # fn f() ->
//! TextStream<impl Stream<Item = &'static str>>
//! # { TextStream::from(stream! { yield "hi" }) }
//! ```
//!
//! While the inner `TextStream! { .. }` invocation expands to:
//!
//! ```rust
//! # use rocket::response::stream::{TextStream, stream};
//! TextStream::from(stream! { /* .. */ })
//! # ;
//! ```
//!
//! The expansions are identical for `ReaderStream` and `ByteStream`, with
//! `TextStream` replaced with `ReaderStream` and `ByteStream`, respectively.
//!
//! # Graceful Shutdown
//!
//! Infinite responders, like the one defined in `hello` above, will prolong
//! shutdown initiated via [`Shutdown::notify()`](crate::Shutdown::notify()) for
//! the defined grace period. After the grace period has elapsed, Rocket will
//! abruptly terminate the responder.
//!
//! To avoid abrupt termination, graceful shutdown can be detected via the
//! [`Shutdown`](crate::Shutdown) future, allowing the infinite responder to
//! gracefully shut itself down. The following example modifies the previous
//! `hello` with shutdown detection:
//!
//! ```rust
//! # use rocket::get;
//! use rocket::Shutdown;
//! use rocket::response::stream::TextStream;
//! use rocket::tokio::select;
//! use rocket::tokio::time::{self, Duration};
//!
//! /// Produce an infinite series of `"hello"`s, 1/second, until shutdown.
//! #[get("/infinite-hellos")]
//! fn hello(mut shutdown: Shutdown) -> TextStream![&'static str] {
//!     TextStream! {
//!         let mut interval = time::interval(Duration::from_secs(1));
//!         loop {
//!             select! {
//!                 _ = interval.tick() => yield "hello",
//!                 _ = &mut shutdown => {
//!                     yield "goodbye";
//!                     break;
//!                 }
//!             };
//!         }
//!     }
//! }
//! ```

use std::{fmt, io};
use std::task::{Context, Poll};
use std::pin::Pin;

use futures::stream::{Stream, StreamExt};
use tokio::io::{AsyncRead, ReadBuf};
use pin_project_lite::pin_project;

use crate::request::Request;
use crate::response::{self, Response, Responder};
use crate::http::ContentType;

pin_project! {
    /// A potentially infinite stream of readers: `T: AsyncRead`.
    ///
    /// A `ReaderStream` can be constructed from any [`Stream`] of items of type
    /// `T` where `T: AsyncRead`, or from a single `AsyncRead` type using
    /// [`ReaderStream::one()`]. Because a `ReaderStream` is itself `AsyncRead`,
    /// it can be used as a building-block for other stream-based responders as
    /// a `streamed_body`, though it may also be used as a responder itself.
    ///
    /// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
    ///
    /// ```rust
    /// use std::io::Cursor;
    ///
    /// use rocket::{Request, Response};
    /// use rocket::futures::stream::{Stream, StreamExt};
    /// use rocket::response::{self, Responder, stream::ReaderStream};
    /// use rocket::http::ContentType;
    ///
    /// struct MyStream<S>(S);
    ///
    /// impl<'r, S: Stream<Item = String>> Responder<'r, 'r> for MyStream<S>
    ///     where S: Send + 'r
    /// {
    ///     fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
    ///         Response::build()
    ///             .header(ContentType::Text)
    ///             .streamed_body(ReaderStream::from(self.0.map(Cursor::new)))
    ///             .ok()
    ///     }
    /// }
    /// ```
    ///
    /// # Responder
    ///
    /// `ReaderStream` is a (potentially infinite) responder. No `Content-Type`
    /// is set. The body is [unsized](crate::response::Body#unsized), and values
    /// are sent as soon as they are yielded by the internal iterator.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::*;
    /// use rocket::response::stream::ReaderStream;
    /// use rocket::futures::stream::{repeat, StreamExt};
    /// use rocket::tokio::time::{self, Duration};
    /// use rocket::tokio::fs::File;
    ///
    /// #[get("/reader/stream")]
    /// fn stream() -> ReaderStream![File] {
    ///     ReaderStream! {
    ///         let paths = &["safe/path", "another/safe/path"];
    ///         for path in paths {
    ///             if let Ok(file) = File::open(path).await {
    ///                 yield file;
    ///             }
    ///         }
    ///     }
    /// }
    ///
    /// #[get("/reader/stream/one")]
    /// async fn stream_one() -> std::io::Result<ReaderStream![File]> {
    ///     let file = File::open("safe/path").await?;
    ///     Ok(ReaderStream::one(file))
    /// }
    /// ```
    ///
    /// The syntax of `ReaderStream` as an expression is identical to that of
    /// [`stream!`].
    pub struct ReaderStream<S: Stream> {
        #[pin]
        stream: S,
        #[pin]
        state: State<S::Item>,
    }
}

pin_project! {
    #[project = StateProjection]
    #[derive(Debug)]
    enum State<R> {
        Pending,
        Reading { #[pin] reader: R },
        Done,
    }
}

/// A potentially infinite stream of bytes: any `T: AsRef<[u8]>`.
///
/// A `ByteStream` can be constructed from any [`Stream`] of items of type `T`
/// where `T: AsRef<[u8]>`. This includes `Vec<u8>`, `&[u8]`, `&str`, `&RawStr`,
/// and more. The stream can be constructed directly, via `ByteStream(..)` or
/// [`ByteStream::from()`], or through generator syntax via [`ByteStream!`].
///
/// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
///
/// # Responder
///
/// `ByteStream` is a (potentially infinite) responder. The response
/// `Content-Type` is set to [`Binary`](ContentType::Binary). The body is
/// [unsized](crate::response::Body#unsized), and values are sent as soon as
/// they are yielded by the internal iterator.
///
/// # Example
///
/// ```rust
/// # use rocket::*;
/// use rocket::response::stream::ByteStream;
/// use rocket::futures::stream::{repeat, StreamExt};
/// use rocket::tokio::time::{self, Duration};
///
/// #[get("/bytes")]
/// fn bytes() -> ByteStream![&'static [u8]] {
///     ByteStream(repeat(&[1, 2, 3][..]))
/// }
///
/// #[get("/byte/stream")]
/// fn stream() -> ByteStream![Vec<u8>] {
///     ByteStream! {
///         let mut interval = time::interval(Duration::from_secs(1));
///         for i in 0..10u8 {
///             yield vec![i, i + 1, i + 2];
///             interval.tick().await;
///         }
///     }
/// }
/// ```
///
/// The syntax of `ByteStream` as an expression is identical to that of
/// [`stream!`].
#[derive(Debug, Clone)]
pub struct ByteStream<S>(pub S);

/// A potentially infinite stream of text: `T: AsRef<str>`.
///
/// A `TextStream` can be constructed from any [`Stream`] of items of type `T`
/// where `T: AsRef<str>`. This includes `&str`, `String`, `Cow<str>`,
/// `&RawStr`, and more. The stream can be constructed directly, via
/// `TextStream(..)` or [`TextStream::from()`], or through generator syntax via
/// [`TextStream!`].
///
/// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
///
/// # Responder
///
/// `TextStream` is a (potentially infinite) responder. The response
/// `Content-Type` is set to [`Text`](ContentType::Text). The body is
/// [unsized](crate::response::Body#unsized), and values are sent as soon as
/// they are yielded by the internal iterator.
///
/// # Example
///
/// ```rust
/// # use rocket::*;
/// use rocket::response::stream::TextStream;
/// use rocket::futures::stream::{repeat, StreamExt};
/// use rocket::tokio::time::{self, Duration};
///
/// #[get("/text")]
/// fn text() -> TextStream![&'static str] {
///     TextStream(repeat("hi"))
/// }
///
/// #[get("/text/stream")]
/// fn stream() -> TextStream![String] {
///     TextStream! {
///         let mut interval = time::interval(Duration::from_secs(1));
///         for i in 0..10 {
///             yield format!("n: {}", i);
///             interval.tick().await;
///         }
///     }
/// }
/// ```
///
/// The syntax of `TextStream` as an expression is identical to that of
/// [`stream!`].
#[derive(Debug, Clone)]
pub struct TextStream<S>(pub S);

impl<S: Stream> From<S> for ReaderStream<S> {
    fn from(stream: S) -> Self {
        ReaderStream { stream, state: State::Pending }
    }
}

impl<S> From<S> for TextStream<S> {
    /// Creates a `TextStream` from any [`S: Stream`](Stream).
    fn from(stream: S) -> Self {
        TextStream(stream)
    }
}

impl<S> From<S> for ByteStream<S> {
    /// Creates a `ByteStream` from any [`S: Stream`](Stream).
    fn from(stream: S) -> Self {
        ByteStream(stream)
    }
}

impl<'r, S: Stream> Responder<'r, 'r> for ReaderStream<S>
    where S: Send + 'r, S::Item: AsyncRead + Send,
{
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .streamed_body(self)
            .ok()
    }
}

/// A stream that yields a value exactly once.
///
/// A `ReaderStream` which wraps this type and yields one `AsyncRead` type can
/// be created via [`ReaderStream::one()`].
///
/// # Example
///
/// ```rust
/// use rocket::response::stream::Once;
/// use rocket::futures::stream::StreamExt;
///
/// # rocket::async_test(async {
/// let mut stream = Once::from("hello!");
/// let values: Vec<_> = stream.collect().await;
/// assert_eq!(values, ["hello!"]);
/// # });
/// ```
pub struct Once<T: Unpin>(Option<T>);

impl<T: Unpin> From<T> for Once<T> {
    fn from(value: T) -> Self {
        Once(Some(value))
    }
}

impl<T: Unpin> Stream for Once<T> {
    type Item = T;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.take())
    }
}

impl<R: Unpin> ReaderStream<Once<R>> {
    /// Create a `ReaderStream` that yields exactly one reader, streaming the
    /// contents of the reader itself.
    ///
    /// # Example
    ///
    /// Stream the bytes from a remote TCP connection:
    ///
    /// ```rust
    /// # use rocket::*;
    /// use std::io;
    /// use std::net::SocketAddr;
    ///
    /// use rocket::tokio::net::TcpStream;
    /// use rocket::response::stream::ReaderStream;
    ///
    /// #[get("/stream")]
    /// async fn stream() -> io::Result<ReaderStream![TcpStream]> {
    ///     let addr = SocketAddr::from(([127, 0, 0, 1], 9999));
    ///     let stream = TcpStream::connect(addr).await?;
    ///     Ok(ReaderStream::one(stream))
    /// }
    /// ```
    pub fn one(reader: R) -> Self {
        ReaderStream::from(Once::from(reader))
    }
}

impl<'r, S: Stream> Responder<'r, 'r> for TextStream<S>
    where S: Send + 'r, S::Item: AsRef<str> + Send + Unpin + 'r
{
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        struct ByteStr<T>(T);

        impl<T: AsRef<str>> AsRef<[u8]> for ByteStr<T> {
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref().as_bytes()
            }
        }

        let inner = self.0.map(ByteStr).map(io::Cursor::new);
        Response::build()
            .header(ContentType::Text)
            .streamed_body(ReaderStream::from(inner))
            .ok()
    }
}

impl<'r, S: Stream> Responder<'r, 'r> for ByteStream<S>
    where S: Send + 'r, S::Item: AsRef<[u8]> + Send + Unpin + 'r
{
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .header(ContentType::Binary)
            .streamed_body(ReaderStream::from(self.0.map(io::Cursor::new)))
            .ok()
    }
}

impl<S: Stream> AsyncRead for ReaderStream<S>
    where S::Item: AsyncRead + Send
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>
    ) -> Poll<Result<(), io::Error>> {
        let mut me = self.project();
        loop {
            match me.state.as_mut().project() {
                StateProjection::Pending => match me.stream.as_mut().poll_next(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(None) => me.state.set(State::Done),
                    Poll::Ready(Some(reader)) => me.state.set(State::Reading { reader }),
                },
                StateProjection::Reading { reader } => {
                    let init = buf.filled().len();
                    match reader.poll_read(cx, buf) {
                        Poll::Ready(Ok(())) if buf.filled().len() == init => {
                            me.state.set(State::Pending);
                        },
                        Poll::Ready(Ok(())) => return Poll::Ready(Ok(())),
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Pending => return Poll::Pending,
                    }
                },
                StateProjection::Done => return Poll::Ready(Ok(())),
            }
        }
    }
}

impl<S: Stream + fmt::Debug> fmt::Debug for ReaderStream<S>
    where S::Item: fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReaderStream")
            .field("stream", &self.stream)
            .field("state", &self.state)
            .finish()
    }
}

crate::export! {
    /// Retrofitted support for [`Stream`]s with `yield`, `for await` syntax.
    ///
    /// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
    ///
    /// This macro takes any series of statements and expands them into an
    /// expression of type `impl Stream<Item = T>`, a stream that `yield`s
    /// elements of type `T`. It supports any Rust statement syntax with the
    /// following additions:
    ///
    ///   * `yield expr`
    ///
    ///      Yields the result of evaluating `expr` to the caller (the stream
    ///      consumer). `expr` must be of type `T`.
    ///
    ///   * `for await x in stream { .. }`
    ///
    ///      `await`s the next element in `stream`, binds it to `x`, and
    ///      executes the block with the binding. `stream` must implement
    ///      `Stream<Item = T>`; the type of `x` is `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::response::stream::stream;
    /// use rocket::futures::stream::Stream;
    ///
    /// fn f(stream: impl Stream<Item = u8>) -> impl Stream<Item = String> {
    ///     stream! {
    ///         for s in &["hi", "there"]{
    ///             yield s.to_string();
    ///         }
    ///
    ///         for await n in stream {
    ///             yield format!("n: {}", n);
    ///         }
    ///     }
    /// }
    ///
    /// # rocket::async_test(async {
    /// use rocket::futures::stream::{self, StreamExt};
    ///
    /// let stream = f(stream::iter(vec![3, 7, 11]));
    /// let strings: Vec<_> = stream.collect().await;
    /// assert_eq!(strings, ["hi", "there", "n: 3", "n: 7", "n: 11"]);
    /// # });
    /// ```
    ///
    /// Using `?` on an `Err` short-cicuits stream termination:
    ///
    /// ```rust
    /// use std::io;
    ///
    /// use rocket::response::stream::stream;
    /// use rocket::futures::stream::Stream;
    ///
    /// fn g<S>(stream: S) -> impl Stream<Item = io::Result<u8>>
    ///     where S: Stream<Item = io::Result<&'static str>>
    /// {
    ///     stream! {
    ///         for await s in stream {
    ///             let num = s?.parse();
    ///             let num = num.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    ///             yield Ok(num);
    ///         }
    ///     }
    /// }
    ///
    /// # rocket::async_test(async {
    /// use rocket::futures::stream::{self, StreamExt};
    ///
    /// let e = io::Error::last_os_error();
    /// let stream = g(stream::iter(vec![Ok("3"), Ok("four"), Err(e), Ok("2")]));
    /// let results: Vec<_> = stream.collect().await;
    /// assert!(matches!(results.as_slice(), &[Ok(3), Err(_)]));
    /// # });
    /// ```
    macro_rules! stream {
        ($($t:tt)*) => ($crate::async_stream::stream!($($t)*));
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! _typed_stream {
    ($S:ident, $T:ty) => (
        $crate::response::stream::$S<impl $crate::futures::stream::Stream<Item = $T>>
    );
    ($S:ident, $($t:tt)*) => (
        $crate::response::stream::$S::from($crate::response::stream::stream!($($t)*))
    );
}

crate::export! {
    /// Type and stream expression macro for [`struct@ReaderStream`].
    ///
    /// See [`struct@ReaderStream`] and the [module level
    /// docs](crate::response::stream#typed-streams) for usage details.
    macro_rules! ReaderStream {
        ($T:ty) => ($crate::_typed_stream!(ReaderStream, $T));
        ($($s:tt)*) => ($crate::_typed_stream!(ReaderStream, $($s)*));
    }
}

crate::export! {
    /// Type and stream expression macro for [`struct@ByteStream`].
    ///
    /// See [`struct@ByteStream`] and the [module level
    /// docs](crate::response::stream#typed-streams) for usage details.
    macro_rules! ByteStream {
        ($T:ty) => ($crate::_typed_stream!(ByteStream, $T));
        ($($s:tt)*) => ($crate::_typed_stream!(ByteStream, $($s)*));
    }
}

crate::export! {
    /// Type and stream expression macro for [`struct@TextStream`].
    ///
    /// See [`struct@TextStream`] and the [module level
    /// docs](crate::response::stream#typed-streams) for usage details.
    macro_rules! TextStream {
        ($T:ty) => ($crate::_typed_stream!(TextStream, $T));
        ($($s:tt)*) => ($crate::_typed_stream!(TextStream, $($s)*));
    }
}
