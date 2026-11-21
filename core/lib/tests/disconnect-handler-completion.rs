#[macro_use] extern crate rocket;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[cfg(feature = "http2")]
use http_body_util::Empty;
#[cfg(feature = "http2")]
use hyper::body::Bytes;
#[cfg(feature = "http2")]
use hyper_util::rt::{TokioExecutor, TokioIo};

use rocket::{Config, Ignite, Rocket, Shutdown, State};
use rocket::fairing::AdHoc;
use rocket::listener::tcp::TcpListener;
use rocket::tokio::io::AsyncWriteExt;
use rocket::tokio::net::TcpStream;
use rocket::tokio::sync::{oneshot, Notify};
use rocket::tokio::task::JoinHandle;
use rocket::tokio::time::timeout;

#[derive(Default)]
struct Signals {
    entered: Notify,
    release: Notify,
    finished: Notify,
    dropped: Notify,
    responded: Notify,
    responses: AtomicUsize,
}

/// Notifies `dropped` when dropped. The handler forgets it once it finishes.
struct Guard<'a>(&'a Signals);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.dropped.notify_one();
    }
}

#[get("/")]
async fn pending(signals: &State<Arc<Signals>>) -> &'static str {
    let guard = Guard(signals);
    signals.entered.notify_one();
    signals.release.notified().await;
    std::mem::forget(guard);
    signals.finished.notify_one();
    "finished"
}

struct Server {
    port: u16,
    shutdown: Shutdown,
    signals: Arc<Signals>,
    task: JoinHandle<Result<Rocket<Ignite>, rocket::Error>>,
}

impl Server {
    async fn launch(grace: u32, mercy: u32) -> Server {
        let mut config = Config::debug_default();
        config.shutdown.ctrlc = false;
        config.shutdown.grace = grace;
        config.shutdown.mercy = mercy;

        let signals = Arc::new(Signals::default());
        let counted = signals.clone();
        let (tx, rx) = oneshot::channel();
        let rocket = rocket::custom(config)
            .manage(signals.clone())
            .mount("/", routes![pending])
            .attach(AdHoc::on_response("Count Responses", move |_, _| {
                let signals = counted.clone();
                Box::pin(async move {
                    signals.responses.fetch_add(1, Ordering::SeqCst);
                    signals.responded.notify_one();
                })
            }))
            .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
                let port = rocket.endpoints().find_map(|e| e.tcp()).unwrap().port();
                tx.send((port, rocket.shutdown())).unwrap();
            })));

        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let task = rocket::tokio::spawn(rocket.try_launch_on(TcpListener::bind(addr)));
        let (port, shutdown) = rx.await.unwrap();
        Server { port, shutdown, signals, task }
    }

    /// Sends an HTTP/1.1 request to `pending` and waits for the handler to start.
    async fn http1_request(&self) -> TcpStream {
        let mut tcp = TcpStream::connect(("127.0.0.1", self.port)).await.unwrap();
        tcp.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await.unwrap();
        self.signals.entered.notified().await;
        tcp
    }

    /// Checks that a handler whose client is gone runs to completion once
    /// released and that its response passes through response fairings.
    async fn check_handler_finishes(&self) {
        let signals = &self.signals;
        let dropped = timeout(Duration::from_millis(500), signals.dropped.notified()).await;
        assert!(dropped.is_err(), "handler was dropped before it finished");

        signals.release.notify_one();
        timeout(Duration::from_secs(2), signals.finished.notified()).await
            .expect("handler did not finish after it was released");
        timeout(Duration::from_secs(2), signals.responded.notified()).await
            .expect("response fairing did not run");
        assert_eq!(signals.responses.load(Ordering::SeqCst), 1);
    }

    async fn shutdown(self) {
        self.shutdown.notify();
        timeout(Duration::from_secs(5), self.task).await.unwrap().unwrap().unwrap();
    }
}

#[rocket::async_test]
async fn http1_disconnect_lets_handler_finish() {
    let server = Server::launch(2, 3).await;
    drop(server.http1_request().await);
    server.check_handler_finishes().await;
    server.shutdown().await;
}

#[rocket::async_test]
async fn http1_half_close_lets_handler_finish() {
    let server = Server::launch(2, 3).await;
    let mut tcp = server.http1_request().await;
    tcp.shutdown().await.unwrap();
    server.check_handler_finishes().await;
    drop(tcp);
    server.shutdown().await;
}

#[cfg(feature = "http2")]
#[rocket::async_test]
async fn http2_reset_lets_handler_finish() {
    let server = Server::launch(2, 3).await;
    let tcp = TcpStream::connect(("127.0.0.1", server.port)).await.unwrap();
    let (mut client, connection) = hyper::client::conn::http2::handshake(
        TokioExecutor::new(), TokioIo::new(tcp)
    ).await.unwrap();

    // Dropping the response future resets the stream.
    let connection = rocket::tokio::spawn(connection);
    let request = hyper::Request::builder().uri("http://localhost/")
        .body(Empty::<Bytes>::new()).unwrap();
    let response = client.send_request(request);
    server.signals.entered.notified().await;
    drop(response);

    server.check_handler_finishes().await;
    drop(client);
    connection.await.unwrap().unwrap();
    server.shutdown().await;
}

#[rocket::async_test]
async fn shutdown_cancels_handler_after_disconnect() {
    let server = Server::launch(1, 1).await;
    drop(server.http1_request().await);
    let dropped = timeout(Duration::from_millis(500), server.signals.dropped.notified()).await;
    assert!(dropped.is_err(), "handler was dropped before shutdown");

    // The handler is never released, so it's cancelled when grace ends.
    let start = Instant::now();
    server.shutdown.notify();
    timeout(Duration::from_secs(5), server.signals.dropped.notified()).await
        .expect("handler was not dropped at shutdown");
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(1500), "handler dropped after {elapsed:?}");
    timeout(Duration::from_secs(5), server.task).await.unwrap().unwrap().unwrap();
}
