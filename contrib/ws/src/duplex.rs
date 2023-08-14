use std::pin::Pin;
use std::task::{Context, Poll};

use rocket::data::IoStream;
use rocket::futures::{StreamExt, SinkExt, Sink};
use rocket::futures::stream::{Stream, FusedStream};

use crate::frame::{Message, CloseFrame};
use crate::result::{Result, Error};

/// A readable and writeable WebSocket [`Message`] `async` stream.
///
/// This struct implements [`Stream`] and [`Sink`], allowing for `async` reading
/// and writing of [`Message`]s. The [`StreamExt`] and [`SinkExt`] traits can be
/// imported to provide additional functionality for streams and sinks:
///
/// ```rust
/// # use rocket::get;
/// # use rocket_ws as ws;
/// use rocket::futures::{SinkExt, StreamExt};
///
/// #[get("/echo/manual")]
/// fn echo_manual<'r>(ws: ws::WebSocket) -> ws::Channel<'r> {
///     ws.channel(move |mut stream| Box::pin(async move {
///         while let Some(message) = stream.next().await {
///             let _ = stream.send(message?).await;
///         }
///
///         Ok(())
///     }))
/// }
/// ```
///
/// [`StreamExt`]: rocket::futures::StreamExt
/// [`SinkExt`]: rocket::futures::SinkExt

pub struct DuplexStream(tokio_tungstenite::WebSocketStream<IoStream>);

impl DuplexStream {
    pub(crate) async fn new(stream: IoStream, config: crate::Config) -> Self {
        use tokio_tungstenite::WebSocketStream;
        use crate::tungstenite::protocol::Role;

        let inner = WebSocketStream::from_raw_socket(stream, Role::Server, Some(config));
        DuplexStream(inner.await)
    }

    /// Close the stream now. This does not typically need to be called.
    pub async fn close(&mut self, msg: Option<CloseFrame<'_>>) -> Result<()> {
        self.0.close(msg).await
    }
}

impl Stream for DuplexStream {
    type Item = Result<Message>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().0.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl FusedStream for DuplexStream {
    fn is_terminated(&self) -> bool {
        self.0.is_terminated()
    }
}

impl Sink<Message> for DuplexStream {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().0.poll_ready_unpin(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.get_mut().0.start_send_unpin(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().0.poll_flush_unpin(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().0.poll_close_unpin(cx)
    }
}
