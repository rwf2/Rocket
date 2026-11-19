#[macro_use] extern crate rocket;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Duration;

use rocket::fairing::AdHoc;
use rocket::listener::tcp::TcpListener;
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::{net::TcpStream, sync::oneshot, time::sleep};
use rocket::tracing::{Event, Level, Metadata, Subscriber};
use rocket::tracing::span::{Attributes, Id, Record};

static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

// Records the names of warning and error spans. Rocket logs a failed
// connection in a span named "request error" or "server error".
struct Capture;

impl Subscriber for Capture {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= Level::WARN
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        WARNINGS.lock().unwrap().push(span.metadata().name().to_string());
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, _: &Event<'_>) {}

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

#[get("/")]
fn index() -> &'static str {
    "hi"
}

// A client that keeps an idle connection open after the server shuts down
// writes leaves the connection lingering when the grace period ends. Ending
// the connection then is part of a normal shutdown, so no error is logged.
#[rocket::async_test]
async fn lingering_connection_ends_at_grace_without_error() {
    // Rocket doesn't install its subscriber if another one is installed.
    rocket::tracing::subscriber::set_global_default(Capture).unwrap();

    let mut config = rocket::Config { linger: 10, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    config.shutdown.grace = 1;
    config.shutdown.mercy = 1;

    let (tx, rx) = oneshot::channel();
    let rocket = rocket::custom(config)
        .mount("/", routes![index])
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            let port = rocket.endpoints().find_map(|e| e.tcp()).unwrap().port();
            tx.send((port, rocket.shutdown())).unwrap();
        })));

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let server = rocket::tokio::spawn(rocket.try_launch_on(TcpListener::bind(addr)));
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
    let failed = |w: &String| w == "request error" || w == "server error";
    assert!(!warnings.iter().any(failed), "{warnings:?}");
}
