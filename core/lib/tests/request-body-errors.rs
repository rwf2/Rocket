#[macro_use] extern crate rocket;

use std::time::Duration;

use rocket::data::{Data, ToByteUnit};
use rocket::fairing::AdHoc;
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::{net::TcpStream, sync::oneshot, time::timeout};

#[post("/", data = "<data>")]
async fn read(data: Data<'_>) -> String {
    let mut body = vec![];
    let result = data.open(1.mebibytes()).read_to_end(&mut body).await;
    format!("{}:{}", String::from_utf8_lossy(&body), result.is_ok())
}

async fn send(port: u16, framing: &str, body: &str, close_write: bool) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("POST / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\
        {framing}\r\n\r\n{body}");

    stream.write_all(request.as_bytes()).await.unwrap();
    if close_write {
        stream.shutdown().await.unwrap();
    }

    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response.split_once("\r\n\r\n").unwrap().1.to_string()
}

async fn check_body_errors() {
    let (tx, rx) = oneshot::channel();
    let mut config = rocket::Config { port: 0, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    let rocket = rocket::custom(config)
        .mount("/", routes![read])
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            tx.send((rocket.config().port, rocket.shutdown())).unwrap();
        })));

    let server = rocket::tokio::spawn(rocket.launch());
    let (port, shutdown) = rx.await.unwrap();

    // The handler reads the body after Rocket has peeked into it.
    let valid = send(port, "Content-Length: 11", "hello world", false).await;
    assert_eq!(valid, "hello world:true");

    // The client sends 5 of 100 declared bytes, then closes its write side.
    let truncated = send(port, "Content-Length: 100", "hello", true).await;
    assert_eq!(truncated, "hello:false");

    // A valid 5-byte chunk followed by an invalid chunk size.
    let chunked = "Transfer-Encoding: chunked";
    let malformed = send(port, chunked, "5\r\nhello\r\ninvalid\r\n", false).await;
    assert_eq!(malformed, "hello:false");

    shutdown.notify();
    server.await.unwrap().unwrap();
}

#[rocket::async_test]
async fn request_body_errors_are_not_eof() {
    timeout(Duration::from_secs(10), check_body_errors()).await.unwrap();
}
