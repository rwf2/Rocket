//! WebSocket support for Rocket.
//!
//! This crate implements support for WebSockets via Rocket's [connection
//! upgrade API](rocket::Response#upgrading) and
//! [tungstenite](tokio_tungstenite).
//!
//! # Usage
//!
//! Depend on the crate. Here, we rename the dependency to `ws` for convenience:
//!
//! ```toml
//! [dependencies]
//! ws = { package = "rocket_ws", version ="=0.1.0-rc.3" }
//! ```
//!
//! Then, use [`WebSocket`] as a request guard in any route and either call
//! [`WebSocket::channel()`] or return a stream via [`Stream!`] or
//! [`WebSocket::stream()`] in the handler. The examples below are equivalent:
//!
//! ```rust
//! # use rocket::get;
//! # use rocket_ws as ws;
//! #
//! #[get("/echo?channel")]
//! fn echo_channel(ws: ws::WebSocket) -> ws::Channel<'static> {
//!     use rocket::futures::{SinkExt, StreamExt};
//!
//!     ws.channel(move |mut stream| Box::pin(async move {
//!         while let Some(message) = stream.next().await {
//!             let _ = stream.send(message?).await;
//!         }
//!
//!         Ok(())
//!     }))
//! }
//!
//! #[get("/echo?stream")]
//! fn echo_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
//!     ws::Stream! { ws =>
//!         for await message in ws {
//!             yield message?;
//!         }
//!     }
//! }
//!
//! #[get("/echo?compose")]
//! fn echo_compose(ws: ws::WebSocket) -> ws::Stream!['static] {
//!     ws.stream(|io| io)
//! }
//! ```
//!
//! WebSocket connections are configurable via [`WebSocket::config()`]:
//!
//! ```rust
//! # use rocket::get;
//! # use rocket_ws as ws;
//! #
//! #[get("/echo")]
//! fn echo_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
//!     let ws = ws.config(ws::Config {
//!         max_send_queue: Some(5),
//!         ..Default::default()
//!     });
//!
//!     ws::Stream! { ws =>
//!         for await message in ws {
//!             yield message?;
//!         }
//!     }
//! }
//! ```

#![doc(html_root_url = "https://api.rocket.rs/v0.5-rc/rocket_ws")]
#![doc(html_favicon_url = "https://rocket.rs/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/images/logo-boxed.png")]

mod tungstenite {
    #[doc(inline)] pub use tokio_tungstenite::tungstenite::*;
}

mod duplex;
mod websocket;

pub use self::websocket::{WebSocket, Channel};

/// A WebSocket message.
///
/// A value of this type is typically constructed by calling `.into()` on a
/// supported message type. This includes strings via `&str` and `String` and
/// bytes via `&[u8]` and `Vec<u8>`:
///
/// ```rust
/// # use rocket::get;
/// # use rocket_ws as ws;
/// #
/// #[get("/echo")]
/// fn echo_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
///     ws::Stream! { ws =>
///         yield "Hello".into();
///         yield String::from("Hello").into();
///         yield (&[1u8, 2, 3][..]).into();
///         yield vec![1u8, 2, 3].into();
///     }
/// }
/// ```
///
/// Other kinds of messages can be constructed directly:
///
/// ```rust
/// # use rocket::get;
/// # use rocket_ws as ws;
/// #
/// #[get("/echo")]
/// fn echo_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
///     ws::Stream! { ws =>
///         yield ws::Message::Ping(vec![b'h', b'i'])
///     }
/// }
/// ```
pub use self::tungstenite::Message;

/// WebSocket connection configuration.
///
/// The default configuration for a [`WebSocket`] can be changed by calling
/// [`WebSocket::config()`] with a value of this type. The defaults are obtained
/// via [`Default::default()`]. You don't generally need to reconfigure a
/// `WebSocket` unless you're certain you need different values. In other words,
/// this structure should rarely be used.
///
/// # Example
///
/// ```rust
/// # use rocket::get;
/// # use rocket_ws as ws;
/// use rocket::data::ToByteUnit;
///
/// #[get("/echo")]
/// fn echo_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
///     let ws = ws.config(ws::Config {
///         // Enable backpressure with a max send queue size of `5`.
///         max_send_queue: Some(5),
///         // Decrease the maximum (complete) message size to 4MiB.
///         max_message_size: Some(4.mebibytes().as_u64() as usize),
///         // Decrease the maximum size of _one_ frame (not messag) to 1MiB.
///         max_frame_size: Some(1.mebibytes().as_u64() as usize),
///         // Use the default values for the rest.
///         ..Default::default()
///     });
///
///     ws::Stream! { ws =>
///         for await message in ws {
///             yield message?;
///         }
///     }
/// }
/// ```
///
/// **Original `tungstenite` Documentation Follows**
///
pub use self::tungstenite::protocol::WebSocketConfig as Config;

/// Structures for constructing raw WebSocket frames.
pub mod frame {
    #[doc(hidden)] pub use crate::Message;
    pub use crate::tungstenite::protocol::frame::{CloseFrame, Frame};
    pub use crate::tungstenite::protocol::frame::coding::CloseCode;
}

/// Types representing incoming and/or outgoing `async` [`Message`] streams.
pub mod stream {
    pub use crate::duplex::DuplexStream;
    pub use crate::websocket::MessageStream;
}

/// Library [`Error`](crate::result::Error) and
/// [`Result`](crate::result::Result) types.
pub mod result {
    pub use crate::tungstenite::error::{Result, Error};
}

/// Type and expression macro for `async` WebSocket [`Message`] streams.
///
/// This macro can be used both where types are expected or
/// where expressions are expected.
///
/// # Type Position
///
/// When used in a type position, the macro invoked as `Stream['r]` expands to:
///
/// - [`MessageStream`]`<'r, impl `[`Stream`]`<Item = `[`Result`]`<`[`Message`]`>>> + 'r>`
///
/// The lifetime need not be specified as `'r`. For instance, `Stream['request]`
/// is valid and expands as expected:
///
/// - [`MessageStream`]`<'request, impl `[`Stream`]`<Item = `[`Result`]`<`[`Message`]`>>> + 'request>`
///
/// As a convenience, when the macro is invoked as `Stream![]`, the lifetime
/// defaults to `'static`. That is, `Stream![]` is equivalent to
/// `Stream!['static]`.
///
/// [`MessageStream`]: crate::stream::MessageStream
/// [`Stream`]: rocket::futures::stream::Stream
/// [`Result`]: crate::result::Result
/// [`Message`]: crate::Message
///
/// # Expression Position
///
/// When invoked as an expression, the macro behaves similarly to Rocket's
/// [`stream!`](rocket::response::stream::stream) macro. Specifically, it
/// supports `yield` and `for await` syntax. It is invoked as follows:
///
/// ```rust
/// # use rocket::get;
/// use rocket_ws as ws;
///
/// #[get("/")]
/// fn echo(ws: ws::WebSocket) -> ws::Stream![] {
///     ws::Stream! { ws =>
///         for await message in ws {
///             yield message?;
///             yield "foo".into();
///             yield vec![1, 2, 3, 4].into();
///         }
///     }
/// }
/// ```
///
/// It enjoins the following type requirements:
///
///   * The type of `ws` _must_ be [`WebSocket`]. `ws` can be any ident.
///   * The type of yielded expressions (`expr` in `yield expr`) _must_ be [`Message`].
///   * The `Err` type of expressions short-circuited with `?` _must_ be [`Error`].
///
/// [`Error`]: crate::result::Error
///
/// The macro takes any series of statements and expands them into an expression
/// of type `impl Stream<Item = `[`Result`]`<T>>`, a stream that `yield`s elements of
/// type [`Result`]`<T>`. It automatically converts yielded items of type `T` into
/// `Ok(T)`. It supports any Rust statement syntax with the following
/// extensions:
///
///   * `?` short-circuits stream termination on `Err`
///
///     The type of the error value must be [`Error`].
///     <br /> <br />
///
///   * `yield expr`
///
///     Yields the result of evaluating `expr` to the caller (the stream
///     consumer) wrapped in `Ok`.
///
///     `expr` must be of type `T`.
///     <br /> <br />
///
///   * `for await x in stream { .. }`
///
///     `await`s the next element in `stream`, binds it to `x`, and executes the
///     block with the binding.
///
///     `stream` must implement `Stream<Item = T>`; the type of `x` is `T`.
///
/// ### Examples
///
/// Borrow from the request. Send a single message and close:
///
/// ```rust
/// # use rocket::get;
/// use rocket_ws as ws;
///
/// #[get("/hello/<user>")]
/// fn ws_hello(ws: ws::WebSocket, user: &str) -> ws::Stream!['_] {
///     ws::Stream! { ws =>
///         yield user.into();
///     }
/// }
/// ```
///
/// Borrow from the request with explicit lifetime:
///
/// ```rust
/// # use rocket::get;
/// use rocket_ws as ws;
///
/// #[get("/hello/<user>")]
/// fn ws_hello<'r>(ws: ws::WebSocket, user: &'r str) -> ws::Stream!['r] {
///     ws::Stream! { ws =>
///         yield user.into();
///     }
/// }
/// ```
///
/// Emit several messages and short-circuit if the client sends a bad message:
///
/// ```rust
/// # use rocket::get;
/// use rocket_ws as ws;
///
/// #[get("/")]
/// fn echo(ws: ws::WebSocket) -> ws::Stream![] {
///     ws::Stream! { ws =>
///         for await message in ws {
///             for i in 0..5u8 {
///                 yield i.to_string().into();
///             }
///
///             yield message?;
///         }
///     }
/// }
/// ```
///
#[macro_export]
macro_rules! Stream {
    () => ($crate::Stream!['static]);
    ($l:lifetime) => (
        $crate::stream::MessageStream<$l, impl rocket::futures::Stream<
            Item = $crate::result::Result<$crate::Message>
        > + $l>
    );
    ($channel:ident => $($token:tt)*) => (
        let ws: $crate::WebSocket = $channel;
        ws.stream(move |$channel| rocket::async_stream::try_stream! {
            $($token)*
        })
    );
}
