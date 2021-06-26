use std::{fmt, io};
use std::task::{Context, Poll};
use std::pin::Pin;

use futures::stream::Stream;
use tokio::io::{AsyncRead, ReadBuf};
use pin_project_lite::pin_project;

use crate::request::Request;
use crate::response::{self, Response, Responder};
use crate::response::stream::One;

pin_project! {
    /// An async reader that reads from a stream of async readers.
    ///
    /// A `ReaderStream` can be constructed from any [`Stream`] of items of type
    /// `T` where `T: AsyncRead`, or from a single `AsyncRead` type using
    /// [`ReaderStream::one()`]. The `AsyncRead` implementation of
    /// `ReaderStream` progresses the stream forward, returning the contents of
    /// the inner readers. Thus, a `ReaderStream` can be thought of as a
    /// _flattening_ of async readers.
    ///
    /// `ReaderStream` is designed to be used as a building-block for
    /// stream-based responders by acting as the `streamed_body` of a
    /// `Response`, though it may also be used as a responder itself.
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
    /// are sent as soon as they are yielded by the internal stream.
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
    /// // Stream the contents of `safe/path` followed by `another/safe/path`.
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
    /// // Stream the contents of the file `safe/path`. This is identical to
    /// // returning `File` directly; Rocket responders stream and never buffer.
    /// #[get("/reader/stream/one")]
    /// async fn stream_one() -> std::io::Result<ReaderStream![File]> {
    ///     let file = File::open("safe/path").await?;
    ///     Ok(ReaderStream::one(file))
    /// }
    /// ```
    ///
    /// The syntax of [`ReaderStream!`] as an expression is identical to that of
    /// [`stream!`](crate::response::stream::stream).
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

impl<R: Unpin> ReaderStream<One<R>> {
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
        ReaderStream::from(One::from(reader))
    }
}

impl<S: Stream> From<S> for ReaderStream<S> {
    fn from(stream: S) -> Self {
        ReaderStream { stream, state: State::Pending }
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

impl<S: Stream> AsyncRead for ReaderStream<S>
    where S::Item: AsyncRead + Send
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>
    ) -> Poll<io::Result<()>> {
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
    /// Type and stream expression macro for [`struct@ReaderStream`].
    ///
    /// See [`stream!`](crate::response::stream::stream) for the syntax
    /// supported by this macro.
    ///
    /// See [`struct@ReaderStream`] and the [module level
    /// docs](crate::response::stream#typed-streams) for usage details.
    macro_rules! ReaderStream {
        ($($s:tt)*) => ($crate::_typed_stream!(ReaderStream, $($s)*));
    }
}
