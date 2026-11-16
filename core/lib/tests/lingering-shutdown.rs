#[macro_use] extern crate rocket;

use std::sync::Mutex;
use std::time::Duration;

use rocket::fairing::AdHoc;
use rocket::log::private as log;
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::{net::TcpStream, sync::oneshot, time::sleep};

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

#[get("/")]
fn index() -> &'static str {
    "hi"
}

// A client that keeps an idle connection open after the server shuts down
// writes leaves the connection lingering when the grace period ends. Ending
// the connection then is part of a normal shutdown, not a failure.
#[rocket::async_test]
async fn lingering_connection_ends_at_grace_without_error() {
    // Rocket doesn't install its logger if another logger is installed.
    log::set_boxed_logger(Box::new(Capture)).unwrap();
    log::set_max_level(log::LevelFilter::Warn);

    let mut config = rocket::Config { port: 0, linger: 10, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    config.shutdown.grace = 1;
    config.shutdown.mercy = 1;

    let (tx, rx) = oneshot::channel();
    let rocket = rocket::custom(config)
        .mount("/", routes![index])
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            tx.send((rocket.config().port, rocket.shutdown())).unwrap();
        })));

    let server = rocket::tokio::spawn(rocket.launch());
    let (port, shutdown) = rx.await.unwrap();

    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await.unwrap();
    let mut response = vec![];
    let mut buf = [0; 1024];
    while !response.ends_with(b"\r\n\r\nhi") {
        let n = stream.read(&mut buf).await.unwrap();
        assert!(n > 0, "connection closed before response");
        response.extend_from_slice(&buf[..n]);
    }

    // Keep the connection open, neither reading nor closing, until shutdown.
    shutdown.notify();
    server.await.unwrap().unwrap();
    sleep(Duration::from_millis(100)).await;
    drop(stream);

    let warnings = WARNINGS.lock().unwrap();
    assert!(!warnings.iter().any(|w| w.contains("Connection failed")), "{warnings:?}");
}
