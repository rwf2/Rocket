#[macro_use] extern crate rocket;

use std::io::{self, Cursor, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use rocket::fairing::AdHoc;
use rocket::futures::future::poll_fn;
use rocket::http::Status;
use rocket::response::{Response, Responder};
use rocket::tokio::io::{AsyncRead, AsyncSeek, ReadBuf};
use rocket::tokio::{net::TcpStream, sync::oneshot, time::timeout};

struct RawResponse(Response<'static>);

impl<'r> Responder<'r, 'static> for RawResponse {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        Ok(self.0)
    }
}

struct FailedSeek;

impl AsyncRead for FailedSeek {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut ReadBuf<'_>
    ) -> Poll<io::Result<()>> {
        panic!("a stripped body must not be read")
    }
}

impl AsyncSeek for FailedSeek {
    fn start_seek(self: Pin<&mut Self>, _: SeekFrom) -> io::Result<()> {
        Err(io::Error::other("cannot seek"))
    }

    fn poll_complete(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Poll::Ready(Err(io::Error::other("cannot seek")))
    }
}

#[get("/<status>/<kind>?<length>&<chunked>")]
fn response(status: u16, kind: &str, length: Option<usize>, chunked: bool) -> RawResponse {
    let mut response = Response::new();
    response.set_status(Status::new(status));
    match kind {
        "none" => {},
        "empty" => { response.set_sized_body(0, Cursor::new("")); },
        "sized" => { response.set_sized_body(5, Cursor::new("hello")); },
        "seek" => { response.set_sized_body(None, Cursor::new("hello")); },
        "stream" => { response.set_streamed_body(Cursor::new("hello")); },
        "failed-seek" => { response.set_sized_body(None, FailedSeek); },
        _ => panic!("unknown body"),
    }

    if let Some(length) = length {
        response.set_raw_header("Content-Length", length.to_string());
    }

    if chunked {
        response.set_raw_header("Transfer-Encoding", "chunked");
    }

    RawResponse(response)
}

#[rocket::async_test]
async fn local_representation_length() {
    let client = rocket::local::asynchronous::Client::debug_with(routes![response]).await.unwrap();
    for (path, length) in [("/304/none?length=123", Some("123")), ("/304/stream", None)] {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.headers().get_one("Content-Length"), length);
        assert_eq!(response.body().preset_size(), None);
        assert!(response.into_bytes().await.unwrap_or_default().is_empty());
    }
}

async fn request(
    client: &mut hyper::client::conn::SendRequest<hyper::Body>,
    method: &str,
    path: &str,
) -> hyper::Response<hyper::Body> {
    let request = hyper::Request::builder()
        .method(method).uri(format!("http://localhost{path}"))
        .body(hyper::Body::empty()).unwrap();

    poll_fn(|cx| client.poll_ready(cx)).await.unwrap();
    client.send_request(request).await.unwrap()
}

async fn check_framing(http2: bool) {
    let (tx, rx) = oneshot::channel();
    let mut config = rocket::Config { port: 0, ..rocket::Config::debug_default() };
    config.shutdown.ctrlc = false;
    let rocket = rocket::custom(config).mount("/", routes![response])
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            tx.send((rocket.config().port, rocket.shutdown())).unwrap();
        })));

    let server = rocket::tokio::spawn(rocket.launch());
    let (port, shutdown) = rx.await.unwrap();
    let tcp = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let (mut client, connection) = hyper::client::conn::Builder::new()
        .http2_only(http2).handshake(tcp).await.unwrap();
    let connection = rocket::tokio::spawn(connection);

    // HEAD and 304 omit the body but preserve its length when it is known.
    for (method, status) in [("HEAD", 200), ("GET", 304)] {
        for (body, length) in [
            ("sized", Some("5")),
            ("seek", Some("5")),
            ("empty", Some("0")),
            ("none", None),
            ("stream", None),
            ("failed-seek", None),
            ("none?length=123", Some("123")),
            ("stream?length=123", Some("123")),
            ("sized?length=123&chunked=true", None),
            ("stream?length=123&chunked=true", None),
        ] {
            let path = format!("/{status}/{body}");
            let response = request(&mut client, method, &path).await;
            assert_eq!(response.status(), status, "{method} {path}");
            let actual = response.headers().get("Content-Length")
                .map(|value| value.to_str().unwrap());
            assert_eq!(actual, length, "{method} {path}");
            assert!(hyper::body::to_bytes(response).await.unwrap().is_empty());

            let response = request(&mut client, "GET", "/200/sized").await;
            assert_eq!(hyper::body::to_bytes(response).await.unwrap(), "hello");
        }
    }

    // A 204 has neither a body nor framing headers, including for HEAD.
    for method in ["GET", "HEAD"] {
        let response = request(&mut client, method, "/204/sized?length=123&chunked=true").await;
        assert_eq!(response.status(), 204);
        assert!(!response.headers().contains_key("Content-Length"));
        assert!(!response.headers().contains_key("Transfer-Encoding"));
        assert!(hyper::body::to_bytes(response).await.unwrap().is_empty());

        let response = request(&mut client, "GET", "/200/sized").await;
        assert_eq!(hyper::body::to_bytes(response).await.unwrap(), "hello");
    }

    // A known body size replaces an explicit length instead of adding another.
    for (body, length, bytes) in [
        ("none", "0", ""),
        ("empty", "0", ""),
        ("sized", "5", "hello"),
    ] {
        let path = format!("/200/{body}?length=123");
        let response = request(&mut client, "GET", &path).await;
        assert_eq!(response.headers().get("Content-Length").unwrap(), length);
        assert_eq!(response.headers().get_all("Content-Length").iter().count(), 1);
        assert_eq!(hyper::body::to_bytes(response).await.unwrap(), bytes);
    }

    // Transfer-Encoding and Content-Length must not be sent together.
    let response = request(&mut client, "GET", "/200/stream?length=123&chunked=true").await;
    assert!(!response.headers().contains_key("Content-Length"));
    assert_eq!(hyper::body::to_bytes(response).await.unwrap(), "hello");

    drop(client);
    shutdown.notify();
    server.await.unwrap().unwrap();
    connection.await.unwrap().unwrap();
}

#[rocket::async_test]
async fn http1_framing() {
    timeout(Duration::from_secs(10), check_framing(false)).await.unwrap();
}

#[cfg(feature = "http2")]
#[rocket::async_test]
async fn http2_framing() {
    timeout(Duration::from_secs(10), check_framing(true)).await.unwrap();
}
