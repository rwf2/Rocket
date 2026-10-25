#![cfg(feature = "tungstenite")]

#[macro_use] extern crate rocket;

use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;

use rocket::State;
use rocket::fairing::AdHoc;
use rocket::futures::{SinkExt, StreamExt};
use rocket::tokio::sync::{Notify, oneshot};
use rocket::tokio::time::timeout;
use rocket_ws as ws;
use ws::frame::{CloseCode, CloseFrame};

struct Dropped(Arc<Notify>);

impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

fn close_frame() -> CloseFrame<'static> {
    CloseFrame { code: CloseCode::Away, reason: "going away".into() }
}

#[get("/close/<peer>/<echo>")]
fn close(
    socket: ws::WebSocket,
    peer: bool,
    echo: bool,
    polled: &State<Arc<AtomicUsize>>,
    dropped: &State<Arc<Notify>>,
) -> ws::Stream!['static] {
    let polled = polled.inner().clone();
    let dropped = dropped.inner().clone();
    ws::Stream! { socket =>
        let mut socket = socket;
        let _dropped = Dropped(dropped);
        if peer {
            let message = socket.next().await.expect("peer close")?;
            assert!(message.is_close());
            if !echo {
                return;
            }

            yield message;
        } else {
            yield ws::Message::Close(Some(close_frame()));
        }

        polled.fetch_add(1, Ordering::SeqCst);
        yield "after close".into();
    }
}

#[async_test]
async fn close_frames_are_sent() {
    let polled = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(Notify::new());
    let (tx, rx) = oneshot::channel();
    let mut config = rocket::Config { port: 0, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    let rocket = rocket::custom(config)
        .manage(polled.clone()).manage(dropped.clone())
        .mount("/", routes![close])
        .attach(AdHoc::on_liftoff("WebSocket Test", |rocket| Box::pin(async move {
            tx.send((rocket.config().port, rocket.shutdown())).unwrap();
        })));

    let server = rocket::tokio::spawn(rocket.launch());
    let (port, shutdown) = rx.await.unwrap();
    for (peer, echo) in [(false, false), (true, true), (true, false)] {
        let url = format!("ws://127.0.0.1:{port}/close/{peer}/{echo}");
        let (mut client, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        if peer {
            client.send(ws::Message::Close(Some(close_frame()))).await.unwrap();
        }

        let message = timeout(Duration::from_secs(3), client.next())
            .await.expect("close frame").unwrap().unwrap();
        assert_eq!(message, ws::Message::Close(Some(close_frame())));
        timeout(Duration::from_secs(3), dropped.notified()).await.expect("stream drop");
        assert_eq!(polled.load(Ordering::SeqCst), 0);
    }

    shutdown.notify();
    server.await.unwrap().unwrap();
}
