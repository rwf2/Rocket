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
//!   * [`struct@EventStream`] ([`EventStream!`]) - Server-Sent [`Event`] stream
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
//! ## Borrowing
//!
//! A stream can _yield_ borrowed values with no extra effort:
//!
//! ```rust
//! # use rocket::get;
//! use rocket::State;
//! use rocket::response::stream::TextStream;
//!
//! /// Produce a single string borrowed from the request.
//! #[get("/infinite-hellos")]
//! fn hello(string: &State<String>) -> TextStream![&str] {
//!     TextStream! {
//!         yield string.as_str();
//!     }
//! }
//! ```
//!
//! If the stream _contains_ a borrowed value or uses one internally, Rust
//! requires this fact be explicit with a lifetime annotation:
//!
//! ```rust
//! # use rocket::get;
//! use rocket::State;
//! use rocket::response::stream::TextStream;
//!
//! #[get("/")]
//! fn borrow1(ctxt: &State<bool>) -> TextStream![&'static str + '_] {
//!     TextStream! {
//!         // By using `ctxt` in the stream, the borrow is moved into it. Thus,
//!         // the stream object contains a borrow, prompting the '_ annotation.
//!         if *ctxt.inner() {
//!             yield "hello";
//!         }
//!     }
//! }
//!
//! // Just as before but yielding an owned yield value.
//! #[get("/")]
//! fn borrow2(ctxt: &State<bool>) -> TextStream![String + '_] {
//!     TextStream! {
//!         if *ctxt.inner() {
//!             yield "hello".to_string();
//!         }
//!     }
//! }
//!
//! // As before but _also_ return a borrowed value. Without it, Rust gives:
//! // - lifetime `'r` is missing in item created through this procedural macro
//! #[get("/")]
//! fn borrow3<'r>(ctxt: &'r State<bool>, s: &'r State<String>) -> TextStream![&'r str + 'r] {
//!     TextStream! {
//!         if *ctxt.inner() {
//!             yield s.as_str();
//!         }
//!     }
//! }
//! ```
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

mod reader;
mod bytes;
mod text;
mod one;
mod sse;
mod raw_sse;

pub(crate) use self::raw_sse::*;

pub use self::one::One;
pub use self::text::TextStream;
pub use self::bytes::ByteStream;
pub use self::reader::ReaderStream;
pub use self::sse::{Event, EventStream};

crate::export! {
    /// Retrofitted support for [`Stream`]s with `yield`, `for await` syntax.
    ///
    /// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
    ///
    /// This macro takes any series of statements and expands them into an
    /// expression of type `impl Stream<Item = T>`, a stream that `yield`s
    /// elements of type `T`. It supports any Rust statement syntax with the
    /// following extensions:
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
    ///   * `?` short-cicuits stream termination on `Err`
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
    ($S:ident, $($t:tt)*) => (
        $crate::__typed_stream! {
            $crate::response::stream::$S,
            $crate::response::stream::stream,
            $crate::futures::stream::Stream,
            $($t)*
        }
    )
}
