#![cfg(feature = "http3-preview")]

#[macro_use] extern crate rocket;

use std::future::poll_fn;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::{Buf, Bytes};
use h3::error::{Code, StreamError};
use rocket::{Build, Rocket, Shutdown};
use rocket::fairing::AdHoc;
use rocket::figment::Figment;
use rocket::futures::{stream, StreamExt};
use rocket::response::{Responder, Response as RocketResponse};
use rocket::tls::TlsConfig;
use rocket::tokio::io::{AsyncRead, ReadBuf};
use rocket::tokio::sync::{oneshot, Notify};
use rocket::tokio::task::JoinHandle;
use rocket::tokio::time::timeout;
use rustls::crypto::ring;
use rustls::pki_types::{CertificateDer, pem::PemObject};

type RequestStream = h3::client::RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>;

fn fixture(name: &str) -> Vec<u8> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/tls");
    std::fs::read(format!("{dir}/private/{name}.pem")).unwrap()
}

/// A Rocket instance that serves HTTP/3 on an ephemeral port.
fn rocket() -> Rocket<Build> {
    let tls = TlsConfig::from_bytes(&fixture("rsa_sha256_cert"), &fixture("rsa_sha256_key"));
    let mut config = rocket::Config::debug_default();
    config.shutdown.ctrlc = false;
    rocket::custom(Figment::from(config).merge(("port", 0)).merge(("tls", tls)))
}

struct Server {
    addr: SocketAddr,
    shutdown: Shutdown,
    task: JoinHandle<()>,
}

impl Server {
    async fn launch(rocket: Rocket<Build>) -> Server {
        let (tx, rx) = oneshot::channel();
        let rocket = rocket.attach(AdHoc::on_liftoff("Send Address", |rocket| {
            Box::pin(async move {
                let addr = rocket.endpoints().find_map(|e| e.quic()).unwrap();
                tx.send((addr, rocket.shutdown())).unwrap();
            })
        }));

        let task = rocket::tokio::spawn(async move { rocket.launch().await.unwrap(); });
        let (addr, shutdown) = rx.await.unwrap();
        Server { addr, shutdown, task }
    }

    async fn shutdown(self) {
        self.shutdown.notify();
        self.task.await.unwrap();
    }
}

/// How a response stream ended.
#[derive(Debug, PartialEq)]
enum End {
    Finished,
    Reset(u64),
    Failed(String),
}

struct Client {
    endpoint: quinn::Endpoint,
    conn: quinn::Connection,
    requests: h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>,
    driver: JoinHandle<()>,
}

impl Client {
    async fn connect(addr: SocketAddr) -> Client {
        let mut roots = rustls::RootCertStore::empty();
        for cert in CertificateDer::pem_slice_iter(&fixture("ca_cert")) {
            roots.add(cert.unwrap()).unwrap();
        }

        let mut tls = rustls::ClientConfig::builder_with_provider(ring::default_provider().into())
            .with_protocol_versions(&[&rustls::version::TLS13]).unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();

        tls.alpn_protocols = vec![b"h3".to_vec()];
        let quic = quinn::crypto::rustls::QuicClientConfig::try_from(tls).unwrap();
        let local = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let mut endpoint = quinn::Endpoint::client(local).unwrap();
        endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(quic)));
        let conn = endpoint.connect(addr, "localhost").unwrap().await.unwrap();
        let (mut driver, requests) = h3::client::new(h3_quinn::Connection::new(conn.clone()))
            .await.unwrap();

        let driver = rocket::tokio::spawn(async move {
            poll_fn(|cx| driver.poll_close(cx)).await;
        });

        Client { endpoint, conn, requests, driver }
    }

    /// Sends a request with `body`, if any.
    async fn request(&mut self, method: &str, path: &str, body: Option<&[u8]>) -> RequestStream {
        let request = http::Request::builder()
            .method(method)
            .uri(format!("https://localhost{path}"))
            .body(()).unwrap();

        let mut stream = self.requests.send_request(request).await.unwrap();
        if let Some(body) = body {
            stream.send_data(Bytes::copy_from_slice(body)).await.unwrap();
        }

        stream.finish().await.unwrap();
        stream
    }

    /// Sends a request with `body`, if any, and waits for the response head.
    async fn send(&mut self, method: &str, path: &str, body: Option<&[u8]>) -> Response {
        let mut stream = self.request(method, path, body).await;
        let head = stream.recv_response().await.unwrap();
        Response { head, stream }
    }

    async fn get(&mut self, path: &str) -> Response {
        self.send("GET", path, None).await
    }

    /// Opens a request stream to write HTTP/3 frames to directly.
    async fn raw(&self) -> (quinn::SendStream, quinn::RecvStream) {
        self.conn.open_bi().await.unwrap()
    }

    /// Closes the connection with `H3_NO_ERROR` and waits until it is closed.
    async fn close(self) {
        self.conn.close(Code::H3_NO_ERROR.value().try_into().unwrap(), b"");
        self.endpoint.wait_idle().await;
        self.driver.await.unwrap();
    }
}

struct Response {
    head: http::Response<()>,
    stream: RequestStream,
}

impl Response {
    fn status(&self) -> u16 {
        self.head.status().as_u16()
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.head.headers().get(name).map(|v| v.to_str().unwrap())
    }

    /// Reads the next chunk of the body, or `Err` with how the stream ended.
    async fn chunk(&mut self) -> Result<Bytes, End> {
        match self.stream.recv_data().await {
            Ok(Some(mut buf)) => Ok(buf.copy_to_bytes(buf.remaining())),
            Ok(None) => Err(End::Finished),
            Err(StreamError::RemoteTerminate { code, .. }) => Err(End::Reset(code.value())),
            Err(e) => Err(End::Failed(e.to_string())),
        }
    }

    /// Reads the rest of the body and returns it with how the stream ended.
    async fn body(&mut self) -> (Vec<u8>, End) {
        let mut body = vec![];
        loop {
            match self.chunk().await {
                Ok(chunk) => body.extend_from_slice(&chunk),
                Err(end) => return (body, end),
            }
        }
    }

    async fn into_string(mut self) -> String {
        let (body, end) = self.body().await;
        assert_eq!(end, End::Finished);
        String::from_utf8(body).unwrap()
    }
}

#[get("/")]
fn index(uri: &rocket::http::uri::Origin<'_>) -> String {
    format!("index: {uri}")
}

#[post("/echo", data = "<body>")]
fn echo(body: &str) -> String {
    format!("echo: {body}")
}

#[rocket::async_test]
async fn round_trip() {
    let rocket = rocket()
        .mount("/", routes![index, echo])
        .attach(AdHoc::on_response("Version", |req, res| Box::pin(async move {
            res.set_raw_header("X-Version", format!("{:?}", req.version()));
        })));

    let server = Server::launch(rocket).await;
    let mut client = Client::connect(server.addr).await;

    let response = client.get("/?a=b").await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.head.version(), http::Version::HTTP_3);
    assert_eq!(response.header("content-type"), Some("text/plain; charset=utf-8"));
    assert_eq!(response.header("content-length"), Some("12"));
    assert_eq!(response.header("x-version"), Some("Some(Http3)"));
    assert_eq!(response.into_string().await, "index: /?a=b");

    let response = client.send("POST", "/echo", Some(b"hello")).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.header("x-version"), Some("Some(Http3)"));
    assert_eq!(response.into_string().await, "echo: hello");

    client.close().await;
    timeout(Duration::from_secs(5), server.shutdown()).await.unwrap();
}

struct RawResponse(RocketResponse<'static>);

impl<'r> Responder<'r, 'static> for RawResponse {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        Ok(self.0)
    }
}

#[get("/failed-stream")]
fn failed_stream(release: &rocket::State<Arc<Notify>>) -> RawResponse {
    let release = release.inner().clone();
    let stream = stream::once(async { Ok::<_, io::Error>(Bytes::from_static(b"hello")) })
        .chain(stream::once(async move {
            release.notified().await;
            Err(io::Error::other("response reader failed"))
        }));

    let reader = tokio_util::io::StreamReader::new(Box::pin(stream));
    RawResponse(RocketResponse::build().streamed_body(reader).finalize())
}

#[get("/bad-header")]
fn bad_header() -> RawResponse {
    RawResponse(RocketResponse::build().raw_header("X-Bad", "a\nb").finalize())
}

async fn check_stream_error(known_length: bool) {
    let release = Arc::new(Notify::new());
    let rocket = rocket().manage(release.clone())
        .mount("/", routes![failed_stream, bad_header])
        .attach(AdHoc::on_response("Optional Length", move |_, response| Box::pin(async move {
            if known_length {
                response.set_raw_header("Content-Length", "10");
            }
        })));

    let server = Server::launch(rocket).await;
    let mut client = Client::connect(server.addr).await;
    let mut response = client.get("/failed-stream").await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.header("content-length").is_some(), known_length);
    assert_eq!(response.chunk().await.unwrap(), "hello");
    release.notify_one();
    let (rest, end) = response.body().await;
    assert!(rest.is_empty());
    assert_eq!(end, End::Reset(Code::H3_INTERNAL_ERROR.value()),
        "failed reader must not produce a successful truncated response");

    // A response that cannot be sent at all is also reset.
    let mut stream = client.request("GET", "/bad-header", None).await;
    match stream.recv_response().await {
        Err(StreamError::RemoteTerminate { code, .. }) => assert_eq!(code, Code::H3_INTERNAL_ERROR),
        result => panic!("expected a reset, found {result:?}"),
    }

    client.close().await;
    server.shutdown().await;
}

#[rocket::async_test]
async fn stream_errors() {
    for known_length in [false, true] {
        timeout(Duration::from_secs(10), check_stream_error(known_length)).await.unwrap();
    }
}

#[derive(Default)]
struct Signals {
    stalled: Notify,
    dropped: Notify,
}

struct StalledReader {
    signals: Arc<Signals>,
    started: bool,
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

#[get("/stalled?<sized>")]
fn stalled(sized: bool, signals: &rocket::State<Arc<Signals>>) -> RawResponse {
    let reader = StalledReader { signals: signals.inner().clone(), started: false };
    let mut response = RocketResponse::build();
    if sized {
        response.raw_header("Content-Length", "10");
    }

    RawResponse(response.streamed_body(reader).finalize())
}

#[get("/ready")]
fn ready() -> &'static str {
    "ready"
}

async fn check_cancellation(reset: bool, sized: bool) {
    let signals = Arc::new(Signals::default());
    let rocket = rocket().manage(signals.clone()).mount("/", routes![stalled, ready]);
    let server = Server::launch(rocket).await;
    let mut client = Client::connect(server.addr).await;
    let mut response = client.get(&format!("/stalled?sized={sized}")).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.chunk().await.unwrap(), "hello");
    signals.stalled.notified().await;
    if reset {
        // Dropping a response stream stops it. `h3_quinn` panics on an explicit
        // `stop_sending()` while a read is pending, as one is here. The server
        // cannot observe the stop, so the reader lives until the connection
        // closes, but other requests on the connection are unaffected.
        drop(response);
        assert_eq!(client.get("/ready").await.into_string().await, "ready");
    }

    client.close().await;
    timeout(Duration::from_secs(2), signals.dropped.notified()).await
        .expect("stalled reader not dropped after HTTP/3 connection was closed");

    server.shutdown().await;
}

#[rocket::async_test]
async fn close_drops_stalled_reader() {
    for sized in [false, true] {
        timeout(Duration::from_secs(10), check_cancellation(false, sized)).await.unwrap();
    }
}

#[rocket::async_test]
async fn reset_then_close_drops_stalled_reader() {
    for sized in [false, true] {
        timeout(Duration::from_secs(10), check_cancellation(true, sized)).await.unwrap();
    }
}

/// Appends `value` to `buf` as a QUIC variable-length integer.
fn varint(buf: &mut Vec<u8>, value: usize) {
    match value {
        0..64 => buf.push(value as u8),
        64..16384 => buf.extend_from_slice(&(value as u16 | 0x4000).to_be_bytes()),
        _ => buf.extend_from_slice(&(value as u32 | 0x8000_0000).to_be_bytes()),
    }
}

/// Appends `value` to `buf` as a QPACK integer with an `n`-bit prefix.
fn prefixed(buf: &mut Vec<u8>, flags: u8, n: u32, value: usize) {
    let max = (1 << n) - 1;
    if value < max {
        return buf.push(flags | value as u8);
    }

    buf.push(flags | max as u8);
    let mut rest = value - max;
    while rest >= 128 {
        buf.push(rest as u8 | 128);
        rest >>= 7;
    }

    buf.push(rest as u8);
}

/// A HEADERS frame carrying `fields` as QPACK literals.
fn headers_frame(fields: &[(&str, &str)]) -> Vec<u8> {
    let mut block = vec![0, 0];
    for (name, value) in fields {
        prefixed(&mut block, 0b0010_0000, 3, name.len());
        block.extend_from_slice(name.as_bytes());
        prefixed(&mut block, 0, 7, value.len());
        block.extend_from_slice(value.as_bytes());
    }

    let mut frame = vec![0x01];
    varint(&mut frame, block.len());
    frame.extend(block);
    frame
}

/// Reads the DATA (0x0) and HEADERS (0x1) frames of a response, as `(type,
/// payload)`, until the stream ends. Other frames, like GREASE, are skipped.
async fn raw_frames(recv: &mut quinn::RecvStream) -> Result<Vec<(usize, Vec<u8>)>, String> {
    fn varint(bytes: &mut &[u8]) -> usize {
        let (int, rest) = bytes.split_at(1 << (bytes[0] >> 6));
        *bytes = rest;
        int[1..].iter().fold(int[0] as usize & 0x3f, |acc, &b| acc << 8 | b as usize)
    }

    let data = recv.read_to_end(1 << 20).await.map_err(|e| e.to_string())?;
    let (mut bytes, mut frames) = (&data[..], vec![]);
    while !bytes.is_empty() {
        let kind = varint(&mut bytes);
        let len = varint(&mut bytes);
        let (payload, rest) = bytes.split_at(len);
        if kind <= 1 {
            frames.push((kind, payload.to_vec()));
        }

        bytes = rest;
    }

    Ok(frames)
}

fn counted(rocket: Rocket<Build>) -> (Rocket<Build>, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let counter = count.clone();
    let rocket = rocket.attach(AdHoc::on_request("Count", move |_, _| {
        counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {})
    }));

    (rocket, count)
}

#[rocket::async_test]
async fn oversized_headers_are_refused() {
    let (rocket, count) = counted(rocket().mount("/", routes![ready]));
    let server = Server::launch(rocket).await;
    let client = Client::connect(server.addr).await;
    let (mut send, mut recv) = client.raw().await;
    let large = "a".repeat(16 * 1024);
    let request = [
        (":method", "GET"), (":scheme", "https"), (":authority", "localhost"),
        (":path", "/ready"), ("x-large", &large),
    ];

    send.write_all(&headers_frame(&request)).await.unwrap();
    send.finish().unwrap();

    // A QPACK block with only `:status: 431`, which h3 sends as a static name
    // reference and a Huffman-coded value.
    let status_431 = vec![0, 0, 0x5f, 0x09, 0x83, 0x69, 0x90, 0xff];
    let frames = timeout(Duration::from_secs(5), raw_frames(&mut recv)).await.unwrap();
    assert_eq!(frames, Ok(vec![(0x01, status_431)]));
    assert_eq!(count.load(Ordering::SeqCst), 0);

    // Headers within the limit are accepted on the same connection.
    let (mut send, mut recv) = client.raw().await;
    let small = "a".repeat(15 * 1024);
    let request = [
        (":method", "GET"), (":scheme", "https"), (":authority", "localhost"),
        (":path", "/ready"), ("x-large", &small),
    ];

    send.write_all(&headers_frame(&request)).await.unwrap();
    send.finish().unwrap();
    let frames = timeout(Duration::from_secs(5), raw_frames(&mut recv)).await.unwrap().unwrap();
    assert_eq!(frames.last(), Some(&(0x00, b"ready".to_vec())));
    assert_eq!(count.load(Ordering::SeqCst), 1);

    client.close().await;
    server.shutdown().await;
}

// Waits for the server's 15 second header read timeout.
#[ignore = "takes 15 seconds"]
#[rocket::async_test]
async fn unfinished_headers_time_out() {
    let (rocket, count) = counted(rocket().mount("/", routes![ready]));
    let server = Server::launch(rocket).await;
    let client = Client::connect(server.addr).await;
    let (mut send, mut recv) = client.raw().await;
    let request = [(":method", "GET"), (":scheme", "https"), (":path", "/ready")];
    let frame = headers_frame(&request);
    send.write_all(&frame[..frame.len() / 2]).await.unwrap();

    let start = std::time::Instant::now();
    let frames = timeout(Duration::from_secs(20), raw_frames(&mut recv)).await.unwrap();
    assert!(start.elapsed() > Duration::from_secs(14), "cut off after {:?}", start.elapsed());
    assert_eq!(frames, Ok(vec![]));
    assert_eq!(count.load(Ordering::SeqCst), 0);

    client.close().await;
    server.shutdown().await;
}
