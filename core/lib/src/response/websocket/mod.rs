//! Contains the websocket implemetation of rocket, based on websocket_codec and rockets own upgrade mecanism.

use bytes::BytesMut;
use futures::Future;
use rocket_http::Status;
use sha1::{Sha1, Digest};
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf, AsyncReadExt};
use tokio::sync::mpsc;
use tokio_util::codec::{Decoder, Encoder};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::borrow::Cow;

use crate::http::hyper::upgrade::Upgraded;
use crate::request::Request;
use crate::response::{self, Response, Responder};
use crate::upgrade::Upgrade;

pub use websocket_codec::Message as WebsocketMessage;

/*
    TODO's:
    - IntoMessage trait
    - websocket extension handling
    - subprotocol handling
*/

/// Represents a close status for a websocket connection.
#[derive(Debug, Clone, Eq)]
pub struct WebsocketStatus<'a> {
    code: u16,
    reason: Cow<'a, str>,
}

impl<'a> PartialEq for WebsocketStatus<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
    }
}

macro_rules! websocket_status_impl {
    ($($name:ident => $code:expr),*) => {
        $(
            /// Websocket pre-defined Status code
            #[allow(non_upper_case_globals)]
            pub const $name: WebsocketStatus<'static> = WebsocketStatus {
                code: $code,
                reason: Cow::Borrowed(stringify!($name))
            };
        )*
    }
}

impl<'a> WebsocketStatus<'a> {
    websocket_status_impl! {
        Ok => 1000,
        GoingAway => 1001,
        ProtocolError => 1002,
        UnknownMessageType => 1003,
        Reserved => 1004,
        NoStatusCode => 1005,
        AbnormalClose => 1006,
        InvalidDataType => 1007,
        PolicyViolation => 1008,
        MessageTooLarge => 1009,
        ExtensionRequired => 1010,
        InternalServerError => 1011,
        TlsFailure => 1015
    }

    /// Creates a new status with a code and a reason.
    pub fn new(code: u16, reason: Cow<'a, str>) -> Self {
        match code {
            0000..=0999 => panic!("Status codes in the range 0-999 are not used"),
            1000..=2999 => panic!(
                "Status codes in the range 1000-2999 are reserved for the WebSocket protocol"
            ),
            3000..=3999 => (),
            4000..=4999 => (),
            _ => panic!("Cannot create a status code outside the allowed range"),
        }
        Self { code, reason }
    }

    /// Returns the code contained in this status.
    pub fn code(&self) -> u16 {
        self.code
    }

    /// Returns the reason contained in this status.
    pub fn reason(&'a self) -> &'a str {
        self.reason.as_ref()
    }
}

impl<'a> Into<WebsocketMessage> for WebsocketStatus<'a> {
    fn into(self) -> WebsocketMessage {
        WebsocketMessage::close(Some((self.code, self.reason.into())))
    }
}

/// Channel to send/recieve messages via a websocket connection.
pub struct WebsocketChannel {
    inner: mpsc::Receiver<WebsocketMessage>,
    sender: mpsc::Sender<WebsocketMessage>,
}

const BUFFER_ALLOCATION_MIN: usize = 1024 * 4;

impl WebsocketChannel {
    fn new(upgrade: Upgraded) -> (Self, impl Future<Output = ()>, impl Future<Output = ()>) {
        let (sender_tx, sender_rx) = mpsc::channel(50);
        let (message_tx, message_rx) = mpsc::channel(1);
        let (a, b) = Self::message_handler(upgrade, message_tx, sender_rx, sender_tx.clone());
        (Self { inner: message_rx, sender: sender_tx }, a, b)
    }

    /// Sends a message over the websocket connection.
    pub async fn send(&self, msg: impl Into<WebsocketMessage>) {
        let _e = self.sender.send(msg.into()).await;
    }

    /// Recieves a message over the websocket connection.
    /// 
    /// This only contains text, binary and close messages; ping & pong is handled by the channel itself.
    pub async fn next(&mut self) -> Option<WebsocketMessage> {
        self.inner.recv().await
    }

    /// Sends a close message & closes the connection.
    pub async fn close(&self, status: WebsocketStatus<'_>) {
        let _e = self.sender.send(status.into()).await;
    }

    async fn send_close(sender_tx: &mpsc::Sender<WebsocketMessage>, status: WebsocketStatus<'_>) {
        let _e = sender_tx.send(status.into()).await;
    }

    async fn reader(
        recv_close: Arc<AtomicBool>,
        mut read: ReadHalf<Upgraded>,
        ping_tx: mpsc::Sender<WebsocketMessage>,
        message_tx: mpsc::Sender<WebsocketMessage>,
        sender_tx: mpsc::Sender<WebsocketMessage>,
    ) {
        let mut codec = websocket_codec::MessageCodec::server();
        let mut read_buf = BytesMut::with_capacity(BUFFER_ALLOCATION_MIN);

        loop {
            let msg = match codec.decode(&mut read_buf) {
                Ok(Some(msg)) => msg,
                Ok(None) => {
                    read_buf.reserve(BUFFER_ALLOCATION_MIN);
                    match read.read_buf(&mut read_buf).await {
                        Ok(0) => break,
                        Ok(_n) => (),
                        Err(e) => {
                            error_!("IO error occured during WebSocket messages: {:?}", e);
                            break;
                        },
                    }
                    continue;
                },
                Err(e) => {
                    error_!("WebSocket client broke protocol: {:?}", e);
                    Self::send_close(&sender_tx, WebsocketStatus::ProtocolError).await;
                    break;
                }
            };

            // Handle ping-pong messages
            if msg.opcode() == websocket_codec::Opcode::Ping {
                let _e = ping_tx.send(msg).await;
                continue;
            } else if msg.opcode() == websocket_codec::Opcode::Pong {
                continue;
            }

            let _e = message_tx.send(msg).await;
        }

        // Set the close flag one last time, just to be sure...
        recv_close.store(true, atomic::Ordering::Release);
    }

    async fn writer(
        recv_close: Arc<AtomicBool>,
        mut write: WriteHalf<Upgraded>,
        mut ping_rx: mpsc::Receiver<WebsocketMessage>,
        mut sender_rx: mpsc::Receiver<WebsocketMessage>,
    ) {
        let mut codec = websocket_codec::MessageCodec::server();
        let mut write_buf = BytesMut::with_capacity(BUFFER_ALLOCATION_MIN);
        let mut close_send = false;
        while let Some(message) = Self::await_or_ping(&mut sender_rx, &mut ping_rx).await {
            if message.opcode() == websocket_codec::Opcode::Close {
                close_send = true;
            }

            if let Err(err) = codec.encode(message, &mut write_buf) {
                error_!("Codec error while trying to encode websocket message: {err:?}");
                continue;
            }

            if let Err(err) = write.write_all_buf(&mut write_buf).await {
                error_!("Io error while trying to send websocket message: {err:?}");
                continue;
            }

            if close_send || recv_close.load(atomic::Ordering::Acquire) {
                break;
            }
        }

        if !close_send {
            warn_!("WebSocket Writer task did not send close");
        }
        let _e = write.shutdown().await;
    }

    fn message_handler(
        upgrade: Upgraded,
        message_tx: mpsc::Sender<WebsocketMessage>,
        sender_rx: mpsc::Receiver<WebsocketMessage>,
        sender_tx: mpsc::Sender<WebsocketMessage>,
    ) -> (impl Future<Output = ()>, impl Future<Output = ()>) {
        let (read, write) = tokio::io::split(upgrade);
        let recv_close = Arc::new(AtomicBool::new(false));
        let (ping_tx, ping_rx) = mpsc::channel(1);
        let reader = Self::reader(recv_close.clone(), read, ping_tx, message_tx, sender_tx);
        let writer = Self::writer(recv_close, write, ping_rx, sender_rx);
        (reader, writer)
    }

    async fn await_or_ping(
        sender_rx: &mut mpsc::Receiver<WebsocketMessage>,
        ping_rx: &mut mpsc::Receiver<WebsocketMessage>,
    ) -> Option<WebsocketMessage> {
        loop {
            tokio::select! {
                o = sender_rx.recv() => break o,
                Some(p) = ping_rx.recv() => break WebsocketMessage::pong(p.into_data()).into(),
            }
        }
    }
}

/// A `Responder` thats used to upgrade a http connection to an websocket connection.
#[derive(Clone)]
pub struct Websocket<F> {
    task: F,
}

#[crate::async_trait]
impl<F> Upgrade<'static> for Websocket<F>
where
    F: Fn(WebsocketChannel) -> Box<dyn Future<Output = ()> + Send> + Sync + Send
{
    async fn start(&self, upgraded: crate::http::hyper::upgrade::Upgraded) {
        // create an channel
        let (ch, a, b) = WebsocketChannel::new(upgraded);

        // run the event loop...
        let event_loop = (self.task)(ch);

        tokio::join!(a, b, Box::into_pin(event_loop));
    }
}

impl<F> Websocket<F>
where
    F: Fn(WebsocketChannel) -> Box<dyn Future<Output = ()> + Send> + Sync
{
    /// Creates a new websocket with the given handler function.
    pub fn create(ws_task: F) -> Self {
        Websocket { task: ws_task }
    }
}

impl<'r, F> Responder<'r, 'r> for Websocket<F>
where
    F: Fn(WebsocketChannel) -> Box<dyn Future<Output = ()> + Send> + Sync + Send + 'r + 'static
{
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        let ws_version = req.headers().get_one("Sec-Websocket-Version");
        match ws_version {
            Some(ws_version) => {
                if ws_version != "13" {
                    return Response::build().status(Status::BadRequest).ok();
                }
            }
            None => {
                return Response::build().status(Status::BadRequest).ok();
            }
        }

        let ws_key = req.headers().get_one("Sec-WebSocket-Key");
        let ws_accept;
        match ws_key {
            Some(ws_key) => {
                // TODO: tidy this up
                let mut s = Sha1::new();
                s.update(ws_key.as_bytes());
                s.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
                let res = s.finalize().to_vec();
                ws_accept = base64::encode(res);
            }
            None => {
                return Response::build().status(Status::BadRequest).ok();
            }
        }

        Response::build()
            .status(Status::SwitchingProtocols)
            .raw_header("Connection", "upgrade")
            .raw_header("Upgrade", "websocket")
            .raw_header("Sec-Websocket-Version", "13")
            .raw_header("Sec-Websocket-Accept", ws_accept)
            .upgrade(Some(Box::new(self)))
            .ok()
    }
}

pub use crate::__websocket as CreateWebsocket;

crate::export! {
    macro_rules! Websocket {
        () => (
            Websocket<
                impl Fn(WebsocketChannel) -> Box<dyn Future<Output = ()> + Send>
            >
        );
    }
}
