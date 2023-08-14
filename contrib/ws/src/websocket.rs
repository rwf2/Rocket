use std::io;
use std::pin::Pin;

use rocket::data::{IoHandler, IoStream};
use rocket::futures::{self, StreamExt, SinkExt, future::BoxFuture, stream::SplitStream};
use rocket::response::{self, Responder, Response};
use rocket::request::{FromRequest, Request, Outcome};
use rocket::http::Status;

use crate::{Config, Message};
use crate::stream::DuplexStream;
use crate::result::{Result, Error};

/// A request guard identifying WebSocket requests. Converts into a [`Channel`]
/// or [`MessageStream`].
///
/// For example usage, see the [crate docs](crate#usage).
///
/// ## Details
///
/// This is the entrypoint to the library. Every WebSocket response _must_
/// initiate via the `WebSocket` request guard. The guard identifies valid
/// WebSocket connection requests and, if the request is valid, succeeds to be
/// converted into a streaming WebSocket response via
/// [`Stream!`](crate::Stream!), [`WebSocket::channel()`], or
/// [`WebSocket::stream()`]. The connection can be configured via
/// [`WebSocket::config()`]; see [`Config`] for details on configuring a
/// connection.
///
/// ### Forwarding
///
/// If the incoming request is not a valid WebSocket request, the guard
/// forwards. The guard never fails.
pub struct WebSocket {
    config: Config,
    key: String,
}

impl WebSocket {
    fn new(key: String) -> WebSocket {
        WebSocket { config: Config::default(), key }
    }

    /// Change the default connection configuration to `config`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::get;
    /// # use rocket_ws as ws;
    /// #
    /// #[get("/echo")]
    /// fn echo_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
    ///     let ws = ws.config(ws::Config {
    ///         max_send_queue: Some(5),
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
    pub fn config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Create a read/write channel to the client and call `handler` with it.
    ///
    /// This method takes a `FnOnce`, `handler`, that consumes a read/write
    /// WebSocket channel, [`DuplexStream`] to the client. See [`DuplexStream`]
    /// for details on how to make use of the channel.
    ///
    /// The `handler` must return a `Box`ed and `Pin`ned future: calling
    /// [`Box::pin()`] with a future does just this as is the preferred
    /// mechanism to create a `Box<Pin<Future>>`. The future must return a
    /// [`Result<()>`](crate::result::Result). The WebSocket connection is
    /// closed successfully if the future returns `Ok` and with an error if
    /// the future returns `Err`.
    ///
    /// # Lifetimes
    ///
    /// The `Channel` may borrow from the request. If it does, the lifetime
    /// should be specified as something other than `'static`. Otherwise, the
    /// `'static` lifetime should be used.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::get;
    /// # use rocket_ws as ws;
    /// use rocket::futures::{SinkExt, StreamExt};
    ///
    /// #[get("/hello/<name>")]
    /// fn hello(ws: ws::WebSocket, name: &str) -> ws::Channel<'_> {
    ///     ws.channel(move |mut stream| Box::pin(async move {
    ///         let message = format!("Hello, {}!", name);
    ///         let _ = stream.send(message.into()).await;
    ///         Ok(())
    ///     }))
    /// }
    ///
    /// #[get("/echo")]
    /// fn echo(ws: ws::WebSocket) -> ws::Channel<'static> {
    ///     ws.channel(move |mut stream| Box::pin(async move {
    ///         while let Some(message) = stream.next().await {
    ///             let _ = stream.send(message?).await;
    ///         }
    ///
    ///         Ok(())
    ///     }))
    /// }
    /// ```
    pub fn channel<'r, F: Send + 'r>(self, handler: F) -> Channel<'r>
        where F: FnOnce(DuplexStream) -> BoxFuture<'r, Result<()>> + 'r
    {
        Channel { ws: self, handler: Box::new(handler), }
    }

    /// Create a stream that consumes client [`Message`]s and emits its own.
    ///
    /// This method takes a `FnOnce` `stream` that consumes a read-only stream
    /// and returns a stream of [`Message`]s. While the returned stream can be
    /// constructed in any manner, the [`Stream!`](crate::Stream!) macro is the
    /// preferred method. In any case, the stream must be `Send`.
    ///
    /// The returned stream must emit items of type `Result<Message>`. Items
    /// that are `Ok(Message)` are sent to the client while items of type
    /// `Err(Error)` result in the connection being closed and the remainder of
    /// the stream discarded.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::get;
    /// # use rocket_ws as ws;
    ///
    /// // Use `Stream!`, which internally calls `WebSocket::stream()`.
    /// #[get("/echo?stream")]
    /// fn echo_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
    ///     ws::Stream! { ws =>
    ///         for await message in ws {
    ///             yield message?;
    ///         }
    ///     }
    /// }
    ///
    /// // Use a raw stream.
    /// #[get("/echo?compose")]
    /// fn echo_compose(ws: ws::WebSocket) -> ws::Stream!['static] {
    ///     ws.stream(|io| io)
    /// }
    /// ```
    pub fn stream<'r, F, S>(self, stream: F) -> MessageStream<'r, S>
        where F: FnOnce(SplitStream<DuplexStream>) -> S + Send + 'r,
              S: futures::Stream<Item = Result<Message>> + Send + 'r
    {
        MessageStream { ws: self, handler: Box::new(stream), }
    }
}

/// A streaming channel, returned by [`WebSocket::channel()`].
///
/// `Channel` has no methods or functionality beyond its trait implementations.
pub struct Channel<'r> {
    ws: WebSocket,
    handler: Box<dyn FnOnce(DuplexStream) -> BoxFuture<'r, Result<()>> + Send + 'r>,
}

/// A [`Stream`](futures::Stream) of [`Message`]s, returned by
/// [`WebSocket::stream()`], used via [`Stream!`].
///
/// This type should not be used directly. Instead, it is used via the
/// [`Stream!`] macro, which expands to both the type itself and an expression
/// which evaluates to this type. See [`Stream!`] for details.
///
/// [`Stream!`]: crate::Stream!
// TODO: Get rid of this or `Channel` via a single `enum`.
pub struct MessageStream<'r, S> {
    ws: WebSocket,
    handler: Box<dyn FnOnce(SplitStream<DuplexStream>) -> S + Send + 'r>
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for WebSocket {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        use crate::tungstenite::handshake::derive_accept_key;
        use rocket::http::uncased::eq;

        let headers = req.headers();
        let is_upgrade = headers.get("Connection")
            .any(|h| h.split(',').any(|v| eq(v.trim(), "upgrade")));

        let is_ws = headers.get("Upgrade")
            .any(|h| h.split(',').any(|v| eq(v.trim(), "websocket")));

        let is_13 = headers.get_one("Sec-WebSocket-Version").map_or(false, |v| v == "13");
        let key = headers.get_one("Sec-WebSocket-Key").map(|k| derive_accept_key(k.as_bytes()));
        match key {
            Some(key) if is_upgrade && is_ws && is_13 => Outcome::Success(WebSocket::new(key)),
            Some(_) | None => Outcome::Forward(Status::NotFound)
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Channel<'o> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        Response::build()
            .raw_header("Sec-Websocket-Version", "13")
            .raw_header("Sec-WebSocket-Accept", self.ws.key.clone())
            .upgrade("websocket", self)
            .ok()
    }
}

impl<'r, 'o: 'r, S> Responder<'r, 'o> for MessageStream<'o, S>
    where S: futures::Stream<Item = Result<Message>> + Send + 'o
{
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        Response::build()
            .raw_header("Sec-Websocket-Version", "13")
            .raw_header("Sec-WebSocket-Accept", self.ws.key.clone())
            .upgrade("websocket", self)
            .ok()
    }
}

#[rocket::async_trait]
impl IoHandler for Channel<'_> {
    async fn io(self: Pin<Box<Self>>, io: IoStream) -> io::Result<()> {
        let channel = Pin::into_inner(self);
        let result = (channel.handler)(DuplexStream::new(io, channel.ws.config).await).await;
        handle_result(result).map(|_| ())
    }
}

#[rocket::async_trait]
impl<'r, S> IoHandler for MessageStream<'r, S>
    where S: futures::Stream<Item = Result<Message>> + Send + 'r
{
    async fn io(self: Pin<Box<Self>>, io: IoStream) -> io::Result<()> {
        let (mut sink, source) = DuplexStream::new(io, self.ws.config).await.split();
        let handler = Pin::into_inner(self).handler;
        let mut stream = std::pin::pin!((handler)(source));
        while let Some(msg) = stream.next().await {
            let result = match msg {
                Ok(msg) => sink.send(msg).await,
                Err(e) => Err(e)
            };

            if !handle_result(result)? {
                return Ok(());
            }
        }

        Ok(())
    }
}

/// Returns `Ok(true)` if processing should continue, `Ok(false)` if processing
/// has terminated without error, and `Err(e)` if an error has occurred.
fn handle_result(result: Result<()>) -> io::Result<bool> {
    match result {
        Ok(_) => Ok(true),
        Err(Error::ConnectionClosed) => Ok(false),
        Err(Error::Io(e)) => Err(e),
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e))
    }
}
