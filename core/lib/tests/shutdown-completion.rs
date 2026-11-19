#[macro_use] extern crate rocket;

use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use rocket::{Build, Rocket};
use rocket::fairing::AdHoc;
use rocket::listener::tcp::TcpListener;
use rocket::response::stream::TextStream;
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::{net::TcpStream, sync::oneshot, time::sleep};

#[get("/")]
fn index() -> &'static str {
    "hi"
}

#[get("/infinite")]
fn infinite() -> TextStream![&'static str] {
    TextStream! {
        loop {
            yield "hi";
            sleep(Duration::from_millis(5)).await;
        }
    }
}

fn rocket(grace: u32, mercy: u32) -> Rocket<Build> {
    let mut config = rocket::Config::debug_default();
    config.shutdown.ctrlc = false;
    config.shutdown.grace = grace;
    config.shutdown.mercy = mercy;
    rocket::custom(config).mount("/", routes![index, infinite])
}

async fn launch(rocket: Rocket<Build>) -> (u16, rocket::Shutdown, impl std::future::Future) {
    let (tx, rx) = oneshot::channel();
    let rocket = rocket
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            let port = rocket.endpoints().find_map(|e| e.tcp()).unwrap().port();
            tx.send((port, rocket.shutdown())).unwrap();
        })));

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let server = rocket::tokio::spawn(rocket.try_launch_on(TcpListener::bind(addr)));
    let (port, shutdown) = rx.await.unwrap();
    (port, shutdown, async move { server.await.unwrap().unwrap(); })
}

async fn get(port: u16, path: &str) -> TcpStream {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    stream
}

// Times shutdown from `notify()` until `launch()` returns.
async fn time_shutdown(shutdown: rocket::Shutdown, server: impl std::future::Future) -> Duration {
    sleep(Duration::from_millis(250)).await;
    let start = Instant::now();
    shutdown.notify();
    server.await;
    start.elapsed()
}

// A cooperating client reading an infinite response stalls shutdown only until
// the grace period elapses.
#[rocket::async_test]
async fn infinite_response_ends_shutdown_after_grace() {
    let (port, shutdown, server) = launch(rocket(1, 2)).await;
    let mut stream = get(port, "/infinite").await;
    let client = rocket::tokio::spawn(async move {
        let mut buf = [0; 1024];
        while matches!(stream.read(&mut buf).await, Ok(n) if n > 0) {}
    });

    let elapsed = time_shutdown(shutdown, server).await;
    assert!(elapsed > Duration::from_millis(900), "{elapsed:?}");
    assert!(elapsed < Duration::from_millis(1500), "{elapsed:?}");
    client.abort();
}

// An idle keep-alive connection closed by a cooperating client doesn't stall
// shutdown for the grace period.
#[rocket::async_test]
async fn idle_connection_does_not_delay_shutdown() {
    let (port, shutdown, server) = launch(rocket(2, 3)).await;
    let mut stream = get(port, "/").await;
    let mut response = vec![];
    let mut buf = [0; 1024];
    while !response.ends_with(b"\r\n\r\nhi") {
        let n = stream.read(&mut buf).await.unwrap();
        assert!(n > 0, "connection closed before response");
        response.extend_from_slice(&buf[..n]);
    }

    // The client closes the connection when the server does.
    let client = rocket::tokio::spawn(async move {
        while matches!(stream.read(&mut buf).await, Ok(n) if n > 0) {}
    });

    let elapsed = time_shutdown(shutdown, server).await;
    assert!(elapsed < Duration::from_secs(1), "{elapsed:?}");
    client.abort();
}
