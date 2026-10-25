#![cfg(feature = "tungstenite")]

#[macro_use] extern crate rocket;

use std::sync::Mutex;
use std::time::Duration;

use rocket::fairing::AdHoc;
use rocket::log::private as log;
use rocket::tokio::sync::oneshot;
use rocket::tokio::time::{sleep, timeout};
use rocket_ws as ws;

static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Capture;

impl log::Log for Capture {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            WARNINGS.lock().unwrap().push(record.args().to_string());
        }
    }

    fn flush(&self) {}
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
    // Rocket doesn't install its logger if another logger is installed.
    log::set_boxed_logger(Box::new(Capture)).unwrap();
    log::set_max_level(log::LevelFilter::Warn);

    let mut config = rocket::Config { port: 0, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    config.shutdown.grace = 1;
    config.shutdown.mercy = 1;

    let (tx, rx) = oneshot::channel();
    let rocket = rocket::custom(config)
        .mount("/", routes![echo])
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            tx.send((rocket.config().port, rocket.shutdown())).unwrap();
        })));

    let server = rocket::tokio::spawn(rocket.launch());
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
    assert!(!warnings.iter().any(|w| w.contains("I/O handler")), "{warnings:?}");
}
