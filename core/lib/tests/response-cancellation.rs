#[macro_use] extern crate rocket;

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

#[cfg(feature = "http2")]
use http_body_util::{BodyExt, Empty};
#[cfg(feature = "http2")]
use hyper::body::Bytes;
#[cfg(feature = "http2")]
use hyper_util::rt::{TokioExecutor, TokioIo};

use rocket::fairing::AdHoc;
use rocket::response::{Responder, Response};
use rocket::tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};
use rocket::tokio::net::TcpStream;
use rocket::tokio::sync::{oneshot, Notify};
use rocket::tokio::time::timeout;

#[derive(Default)]
struct Signals {
    stalled: Notify,
    dropped: Notify,
}

struct StalledReader {
    signals: Arc<Signals>,
    started: bool,
    sized: bool,
}

impl AsyncRead for StalledReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>
    ) -> Poll<io::Result<()>> {
        if !self.started {
            self.started = true;
            buf.put_slice(b"hello");
            return Poll::Ready(Ok(()));
        }

        self.signals.stalled.notify_one();
        Poll::Pending
    }
}

impl Drop for StalledReader {
    fn drop(&mut self) {
        self.signals.dropped.notify_one();
    }
}

impl<'r> Responder<'r, 'static> for StalledReader {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let mut response = Response::build();
        if self.sized {
            response.raw_header("Content-Length", "10");
        }

        response.streamed_body(self).ok()
    }
}

#[get("/?<sized>")]
fn stalled(sized: bool, signals: &rocket::State<Arc<Signals>>) -> StalledReader {
    StalledReader { signals: signals.inner().clone(), started: false, sized }
}

#[get("/ready")]
fn ready() -> &'static str {
    "ready"
}

async fn check_cancellation(http2: bool, sized: bool) {
    let signals = Arc::new(Signals::default());
    let (tx, rx) = oneshot::channel();
    let mut config = rocket::Config { port: 0, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    let rocket = rocket::custom(config).manage(signals.clone())
        .mount("/", routes![stalled, ready])
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            tx.send((rocket.config().port, rocket.shutdown())).unwrap();
        })));

    let server = rocket::tokio::spawn(rocket.launch());
    let (port, shutdown) = rx.await.unwrap();
    let mut tcp = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    if http2 {
        #[cfg(feature = "http2")]
        {
            let (mut client, connection) = hyper::client::conn::http2::handshake(
                TokioExecutor::new(), TokioIo::new(tcp)
            ).await.unwrap();
            let connection = rocket::tokio::spawn(connection);
            let uri = format!("http://localhost/?sized={sized}");
            let request = hyper::Request::builder().uri(uri)
                .body(Empty::<Bytes>::new()).unwrap();
            let mut response = client.send_request(request).await.unwrap();
            assert_eq!(response.version(), hyper::Version::HTTP_2);
            assert_eq!(response.body_mut().frame().await.unwrap().unwrap().data_ref().unwrap(),
                "hello");
            signals.stalled.notified().await;
            drop(response);
            timeout(Duration::from_secs(2), signals.dropped.notified()).await
                .expect("stalled reader retained after HTTP/2 response stream was dropped");

            let request = hyper::Request::builder().uri("http://localhost/ready")
                .body(Empty::<Bytes>::new()).unwrap();
            let response = client.send_request(request).await.unwrap();
            assert_eq!(response.collect().await.unwrap().to_bytes(), "ready");
            drop(client);
            connection.await.unwrap().unwrap();
        }
        #[cfg(not(feature = "http2"))]
        unreachable!();
    } else {
        let request = format!("GET /?sized={sized} HTTP/1.1\r\nHost: localhost\r\n\r\n");
        tcp.write_all(request.as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        while !bytes.windows(5).any(|window| window == b"hello") {
            let mut buf = [0; 1024];
            let count = tcp.read(&mut buf).await.unwrap();
            assert_ne!(count, 0);
            bytes.extend_from_slice(&buf[..count]);
        }

        assert!(bytes.starts_with(b"HTTP/1.1 200"));
        signals.stalled.notified().await;
        drop(tcp);
        timeout(Duration::from_secs(2), signals.dropped.notified()).await
            .expect("stalled reader retained after HTTP/1.1 TCP close");
    }

    shutdown.notify();
    server.await.unwrap().unwrap();
}

#[rocket::async_test]
async fn http1_disconnect_drops_stalled_reader() {
    for sized in [false, true] {
        timeout(Duration::from_secs(10), check_cancellation(false, sized)).await.unwrap();
    }
}

#[cfg(feature = "http2")]
#[rocket::async_test]
async fn http2_reset_drops_stalled_reader() {
    for sized in [false, true] {
        timeout(Duration::from_secs(10), check_cancellation(true, sized)).await.unwrap();
    }
}
