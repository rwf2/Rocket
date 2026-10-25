#![cfg(feature = "tungstenite")]

#[macro_use] extern crate rocket;

use std::sync::Mutex;
use std::time::Duration;

use rocket::State;
use rocket::fairing::AdHoc;
use rocket::futures::{SinkExt, StreamExt};
use rocket::tokio::sync::oneshot;
use rocket::tokio::task::JoinHandle;
use rocket::tokio::time::timeout;
use rocket_ws as ws;
use ws::frame::{CloseCode, CloseFrame};

struct Output {
    resumed: bool,
    dropped: Option<oneshot::Sender<bool>>,
}

impl Drop for Output {
    fn drop(&mut self) {
        if let Some(dropped) = self.dropped.take() {
            let _ = dropped.send(self.resumed);
        }
    }
}

fn close_frame() -> CloseFrame {
    CloseFrame { code: CloseCode::Away, reason: "going away".into() }
}

#[get("/server-close")]
fn server_close(
    _socket: ws::WebSocket,
    output: &State<Mutex<Option<Output>>>,
) -> ws::Stream!['static] {
    let mut output = output.lock().unwrap().take().unwrap();
    ws::Stream! { _socket =>
        yield ws::Message::Close(Some(close_frame()));
        output.resumed = true;
        yield "after close".into();
        drop(output);
    }
}

#[get("/echo-close")]
fn echo_close(socket: ws::WebSocket) -> ws::Stream!['static] {
    ws::Stream! { socket =>
        let mut socket = socket;
        let message = socket.next().await.expect("peer close")?;
        assert!(message.is_close());
        yield message;
    }
}

#[get("/end-after-close")]
fn end_after_close(socket: ws::WebSocket) -> ws::Stream!['static] {
    ws::Stream! { socket =>
        for await message in socket {
            let message = message?;
            if message.is_close() {
                return;
            }

            yield message;
        }
    }
}

struct Server {
    shutdown: rocket::Shutdown,
    task: JoinHandle<()>,
}

impl Server {
    async fn stop(mut self) {
        self.shutdown.clone().notify();
        timeout(Duration::from_secs(3), &mut self.task)
            .await.expect("server shutdown").unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown.clone().notify();
    }
}

async fn serve(rocket: rocket::Rocket<rocket::Build>) -> (u16, Server) {
    let (tx, rx) = oneshot::channel();
    let mut config = rocket::Config { port: 0, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    let rocket = rocket.configure(config)
        .attach(AdHoc::on_liftoff("WebSocket Test", |rocket| Box::pin(async move {
            let _ = tx.send(rocket.config().port);
        })));

    let rocket = timeout(Duration::from_secs(3), rocket.ignite())
        .await.expect("server ignition").unwrap();
    let server = Server {
        shutdown: rocket.shutdown(),
        task: rocket::tokio::spawn(async move { rocket.launch().await.unwrap(); }),
    };
    let port = timeout(Duration::from_secs(3), rx).await.expect("server startup").unwrap();
    (port, server)
}

#[async_test]
async fn server_close_stops_output() {
    let (dropped, resumed) = oneshot::channel();
    let output = Output { resumed: false, dropped: Some(dropped) };
    let rocket = rocket::build().manage(Mutex::new(Some(output)))
        .mount("/", routes![server_close]);
    let (port, server) = serve(rocket).await;
    let url = format!("ws://127.0.0.1:{port}/server-close");
    let (mut client, _) = timeout(Duration::from_secs(3), tokio_tungstenite::connect_async(url))
        .await.expect("client connection").unwrap();

    let message = timeout(Duration::from_secs(3), client.next())
        .await.expect("server close").unwrap().unwrap();
    assert_eq!(message, ws::Message::Close(Some(close_frame())));
    assert!(!timeout(Duration::from_secs(3), resumed).await.expect("stream drop").unwrap());
    server.stop().await;
}

#[async_test]
async fn peer_close_is_echoed() {
    let (port, server) = serve(rocket::build().mount("/", routes![echo_close])).await;
    let url = format!("ws://127.0.0.1:{port}/echo-close");
    let (mut client, _) = timeout(Duration::from_secs(3), tokio_tungstenite::connect_async(url))
        .await.expect("client connection").unwrap();
    timeout(Duration::from_secs(3), client.send(ws::Message::Close(Some(close_frame()))))
        .await.expect("peer close").unwrap();

    let message = timeout(Duration::from_secs(3), client.next())
        .await.expect("echoed close").unwrap().unwrap();
    assert_eq!(message, ws::Message::Close(Some(close_frame())));
    server.stop().await;
}

#[async_test]
async fn stream_end_flushes_peer_close() {
    let (port, server) = serve(rocket::build().mount("/", routes![end_after_close])).await;
    let url = format!("ws://127.0.0.1:{port}/end-after-close");
    let (mut client, _) = timeout(Duration::from_secs(3), tokio_tungstenite::connect_async(url))
        .await.expect("client connection").unwrap();
    timeout(Duration::from_secs(3), client.send(ws::Message::Close(Some(close_frame()))))
        .await.expect("peer close").unwrap();

    let message = timeout(Duration::from_secs(3), client.next())
        .await.expect("queued close reply").unwrap().unwrap();
    assert_eq!(message, ws::Message::Close(Some(close_frame())));
    server.stop().await;
}
