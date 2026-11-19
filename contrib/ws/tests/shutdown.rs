#![cfg(feature = "tungstenite")]

#[macro_use] extern crate rocket;

use std::fmt;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Duration;

use rocket::fairing::AdHoc;
use rocket::listener::tcp::TcpListener;
use rocket::tokio::sync::oneshot;
use rocket::tokio::time::{sleep, timeout};
use rocket::tracing::{Event, Level, Metadata, Subscriber};
use rocket::tracing::field::{Field, Visit};
use rocket::tracing::span::{Attributes, Id, Record};
use rocket_ws as ws;

static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

// Records the messages of warning and error events.
struct Capture;

struct Message<'a>(&'a mut String);

impl Visit for Message<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            *self.0 = format!("{value:?}");
        }
    }
}

impl Subscriber for Capture {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= Level::WARN
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut message = String::new();
        event.record(&mut Message(&mut message));
        WARNINGS.lock().unwrap().push(message);
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

#[get("/echo")]
fn echo(socket: ws::WebSocket) -> ws::Stream!['static] {
    ws::Stream! { socket =>
        for await message in socket {
            match message {
                Ok(message) => yield message,
                Err(_) => break,
            }
        }
    }
}

// Graceful shutdown cancels an open connection when the grace period ends. A
// stream that then ends without error ends normally, even though its final
// flush can't reach the connection.
#[async_test]
async fn stream_ended_by_shutdown_is_not_reported() {
    // Rocket doesn't install its subscriber if another one is installed.
    rocket::tracing::subscriber::set_global_default(Capture).unwrap();

    let mut config = rocket::Config::debug_default();
    config.shutdown.ctrlc = false;
    config.shutdown.grace = 1;
    config.shutdown.mercy = 1;

    let (tx, rx) = oneshot::channel();
    let rocket = rocket::custom(config)
        .mount("/", routes![echo])
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            let port = rocket.endpoints().find_map(|e| e.tcp()).unwrap().port();
            tx.send((port, rocket.shutdown())).unwrap();
        })));

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let server = rocket::tokio::spawn(rocket.try_launch_on(TcpListener::bind(addr)));
    let (port, shutdown) = rx.await.unwrap();

    let url = format!("ws://127.0.0.1:{port}/echo");
    let (client, _) = timeout(Duration::from_secs(3), tokio_tungstenite::connect_async(url))
        .await.expect("client connection").unwrap();

    // Keep the connection open, neither reading nor closing, until shutdown.
    shutdown.notify();
    server.await.unwrap().unwrap();
    sleep(Duration::from_millis(100)).await;
    drop(client);

    let warnings = WARNINGS.lock().unwrap();
    assert!(!warnings.iter().any(|w| w.contains("i/o handler")), "{warnings:?}");
}
