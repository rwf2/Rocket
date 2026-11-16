#[macro_use] extern crate rocket;

use std::time::{Duration, Instant};

use rocket::{Build, Rocket};
use rocket::data::{Data, ToByteUnit};
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::net::{TcpStream, tcp::OwnedWriteHalf};
use rocket::tokio::{sync::oneshot, task::JoinHandle, time::{sleep, timeout}};

const BODY: usize = 16 << 20;

#[post("/", data = "<data>")]
async fn upload(data: Data<'_>) -> (Status, &'static str) {
    match data.open(64.kibibytes()).into_bytes().await {
        Ok(bytes) if bytes.is_complete() => (Status::Ok, "ok"),
        _ => (Status::PayloadTooLarge, "too large"),
    }
}

fn rocket(linger: u32, grace: u32, mercy: u32) -> Rocket<Build> {
    let mut config = rocket::Config { port: 0, linger, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    config.shutdown.grace = grace;
    config.shutdown.mercy = mercy;
    rocket::custom(config).mount("/", routes![upload])
}

async fn launch(rocket: Rocket<Build>) -> (u16, rocket::Shutdown, impl std::future::Future) {
    let (tx, rx) = oneshot::channel();
    let rocket = rocket
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            tx.send((rocket.config().port, rocket.shutdown())).unwrap();
        })));

    let server = rocket::tokio::spawn(rocket.launch());
    let (port, shutdown) = rx.await.unwrap();
    (port, shutdown, async move { server.await.unwrap().unwrap(); })
}

async fn start_upload(port: u16) -> TcpStream {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let head = format!("POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: {BODY}\r\n\r\n");
    stream.write_all(head.as_bytes()).await.unwrap();
    stream
}

// Keeps sending body data until a write fails. Returns when that happened.
fn keep_sending(mut writer: OwnedWriteHalf) -> JoinHandle<Instant> {
    rocket::tokio::spawn(async move {
        let chunk = vec![b'a'; 16 << 10];
        while writer.write_all(&chunk).await.is_ok() {
            sleep(Duration::from_millis(5)).await;
        }

        Instant::now()
    })
}

// Like some browsers, the client sends the entire body before reading the
// response. The handler stops reading at the limit. The client must still be
// able to send the body and read the response without a TCP reset.
#[rocket::async_test]
async fn response_survives_unread_body() {
    let (port, shutdown, server) = launch(rocket(10, 2, 3)).await;
    let mut stream = start_upload(port).await;
    timeout(Duration::from_secs(10), stream.write_all(&vec![b'a'; BODY]))
        .await
        .expect("upload timed out")
        .expect("upload failed");

    let mut response = String::new();
    timeout(Duration::from_secs(10), stream.read_to_string(&mut response))
        .await
        .expect("response timed out")
        .expect("response failed");

    assert!(response.starts_with("HTTP/1.1 413"));
    assert!(response.ends_with("\r\n\r\ntoo large"));

    drop(stream);
    shutdown.notify();
    server.await;
}

// A client that keeps sending after the response can do so for at most
// `linger` seconds, after which the connection is closed.
#[rocket::async_test]
async fn linger_is_bounded() {
    let (port, shutdown, server) = launch(rocket(1, 2, 3)).await;
    let (mut reader, writer) = start_upload(port).await.into_split();
    let sender = keep_sending(writer);

    let mut response = String::new();
    timeout(Duration::from_secs(10), reader.read_to_string(&mut response))
        .await
        .expect("response timed out")
        .expect("response failed");

    let closed = Instant::now();
    let failed = timeout(Duration::from_secs(10), sender).await.unwrap().unwrap();
    let lingered = failed.saturating_duration_since(closed);
    assert!(response.ends_with("\r\n\r\ntoo large"));
    assert!(lingered > Duration::from_millis(500), "{lingered:?}");
    assert!(lingered < Duration::from_secs(5), "{lingered:?}");

    shutdown.notify();
    server.await;
}

// When lingering is disabled, the connection is closed immediately after the
// response.
#[rocket::async_test]
async fn linger_can_be_disabled() {
    let (port, shutdown, server) = launch(rocket(0, 2, 3)).await;
    let start = Instant::now();
    let (_reader, writer) = start_upload(port).await.into_split();
    let failed = timeout(Duration::from_secs(10), keep_sending(writer)).await.unwrap().unwrap();
    let elapsed = failed - start;
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");

    shutdown.notify();
    server.await;
}

// A client that keeps sending after the response is uncooperative. Like any
// uncooperative client, it can stall shutdown for at most `grace + mercy`
// seconds, not `linger` seconds.
#[rocket::async_test]
async fn linger_does_not_delay_shutdown() {
    let (port, shutdown, server) = launch(rocket(10, 1, 2)).await;
    let (_reader, writer) = start_upload(port).await.into_split();
    let sender = keep_sending(writer);

    sleep(Duration::from_millis(250)).await;
    let start = Instant::now();
    shutdown.notify();
    server.await;
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(3500), "{elapsed:?}");
    sender.abort();
}
