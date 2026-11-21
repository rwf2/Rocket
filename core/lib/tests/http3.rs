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

/// A Rocket instance that serves HTTP/3. `Server::launch()` sets its port.
fn rocket() -> Rocket<Build> {
    let tls = TlsConfig::from_bytes(&fixture("rsa_sha256_cert"), &fixture("rsa_sha256_key"));
    let mut config = rocket::Config::debug_default();
    config.shutdown.ctrlc = false;
    rocket::custom(Figment::from(config).merge(("tls", tls)))
}

/// Held while sockets are bound so that a port found by `free_port()` is still
/// free when Rocket binds to it.
static BINDING: rocket::tokio::sync::Mutex<()> = rocket::tokio::sync::Mutex::const_new(());

/// Returns a port that's free for TCP and UDP: Rocket serves HTTP/3 on the UDP
/// port with the same number as its TCP port.
fn free_port() -> u16 {
    use std::net::{TcpListener, UdpSocket};

    loop {
        let udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = udp.local_addr().unwrap().port();
        if TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok() {
            return port;
        }
    }
}

struct Server {
    addr: SocketAddr,
    shutdown: Shutdown,
    task: JoinHandle<()>,
}

impl Server {
    async fn launch(rocket: Rocket<Build>) -> Server {
        let _binding = BINDING.lock().await;
        let figment = rocket.figment().clone().merge(("port", free_port()));
        let (tx, rx) = oneshot::channel();
        let rocket = rocket.reconfigure(figment);
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

/// Opens a QUIC connection to `addr` that negotiates HTTP/3.
async fn quic_connect(addr: SocketAddr) -> (quinn::Endpoint, quinn::Connection) {
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
    let mut endpoint = {
        let _binding = BINDING.lock().await;
        quinn::Endpoint::client(local).unwrap()
    };

    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(quic)));
    let conn = endpoint.connect(addr, "localhost").unwrap().await.unwrap();
    (endpoint, conn)
}

struct Client {
    endpoint: quinn::Endpoint,
    conn: quinn::Connection,
    requests: h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>,
    driver: JoinHandle<()>,
}

impl Client {
    async fn connect(addr: SocketAddr) -> Client {
        let (endpoint, conn) = quic_connect(addr).await;
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

        // The server may respond, and stop reading, before the request ends.
        if let Err(e) = stream.finish().await {
            assert!(matches!(e, StreamError::RemoteTerminate { .. }), "{e}");
        }

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

/// Reads a QUIC variable-length integer from `recv`.
async fn read_varint(recv: &mut quinn::RecvStream) -> u64 {
    let mut buf = [0; 8];
    recv.read_exact(&mut buf[..1]).await.unwrap();
    let len = 1 << (buf[0] >> 6);
    recv.read_exact(&mut buf[1..len]).await.unwrap();
    buf[1..len].iter().fold(buf[0] as u64 & 0x3f, |acc, &b| acc << 8 | b as u64)
}

/// An HTTP/3 connection without an h3 client, to see what the server sends on
/// its control stream.
struct RawClient {
    endpoint: quinn::Endpoint,
    conn: quinn::Connection,
    control: quinn::SendStream,
}

impl RawClient {
    async fn connect(addr: SocketAddr) -> RawClient {
        let (endpoint, conn) = quic_connect(addr).await;
        // Open the control stream with an empty SETTINGS frame. Closing it is
        // a connection error, so it lives as long as the client.
        let mut control = conn.open_uni().await.unwrap();
        control.write_all(&[0x00, 0x04, 0x00]).await.unwrap();
        RawClient { endpoint, conn, control }
    }

    /// Sends a GET request for `path`.
    async fn get(&self, path: &str) -> quinn::RecvStream {
        let (mut send, recv) = self.conn.open_bi().await.unwrap();
        let request = [
            (":method", "GET"), (":scheme", "https"), (":authority", "localhost"),
            (":path", path),
        ];

        send.write_all(&headers_frame(&request)).await.unwrap();
        send.finish().unwrap();
        recv
    }

    /// Sends a GOAWAY frame with push ID 0 on the control stream.
    async fn send_goaway(&mut self) {
        self.control.write_all(&[0x07, 0x01, 0x00]).await.unwrap();
    }

    /// Returns the stream ID in the first GOAWAY frame the server sends.
    /// Consumes the server's control stream, so it must only be called once.
    async fn goaway(&self) -> u64 {
        let mut control = loop {
            let mut recv = self.conn.accept_uni().await.unwrap();
            if read_varint(&mut recv).await == 0x00 {
                break recv;
            }
        };

        loop {
            let (kind, len) = (read_varint(&mut control).await, read_varint(&mut control).await);
            if kind == 0x07 {
                return read_varint(&mut control).await;
            }

            let mut payload = vec![0; len as usize];
            control.read_exact(&mut payload).await.unwrap();
        }
    }

    async fn close(self) {
        self.conn.close(Code::H3_NO_ERROR.value().try_into().unwrap(), b"");
        self.endpoint.wait_idle().await;
    }
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

#[get("/connection-headers")]
fn connection_headers() -> RawResponse {
    RawResponse(RocketResponse::build()
        .raw_header("Transfer-Encoding", "chunked")
        .raw_header("Connection", "close, x-hop")
        .raw_header("X-Hop", "1")
        .raw_header_adjoin("Connection", "x-hop-2")
        .raw_header("X-Hop-2", "1")
        .raw_header("Keep-Alive", "timeout=5")
        .raw_header("Proxy-Connection", "keep-alive")
        .raw_header("Upgrade", "websocket")
        .raw_header("TE", "trailers")
        .raw_header("X-Kept", "1")
        .sized_body(5, std::io::Cursor::new("hello"))
        .finalize())
}

#[rocket::async_test]
async fn connection_headers_are_not_sent() {
    // The response itself is unchanged: only HTTP/3 leaves the headers out.
    let app = rocket().mount("/", routes![connection_headers]);
    let local = rocket::local::asynchronous::Client::untracked(app).await.unwrap();
    let response = local.get("/connection-headers").dispatch().await;
    assert_eq!(response.headers().get_one("Transfer-Encoding"), Some("chunked"));
    assert_eq!(response.headers().get_one("X-Hop"), Some("1"));
    assert_eq!(response.headers().get("Connection").count(), 2);

    let server = Server::launch(rocket().mount("/", routes![connection_headers])).await;
    let mut client = Client::connect(server.addr).await;
    let response = client.get("/connection-headers").await;
    assert_eq!(response.status(), 200);
    for name in [
        "transfer-encoding", "connection", "x-hop", "x-hop-2", "keep-alive",
        "proxy-connection", "upgrade", "te",
    ] {
        assert_eq!(response.header(name), None, "{name}");
    }

    assert_eq!(response.header("x-kept"), Some("1"));
    assert_eq!(response.into_string().await, "hello");

    client.close().await;
    server.shutdown().await;
}

#[derive(Default)]
struct Slow {
    entered: Notify,
    release: Notify,
    finished: Notify,
}

#[get("/slow")]
async fn slow(slow: &rocket::State<Arc<Slow>>) -> &'static str {
    slow.entered.notify_one();
    slow.release.notified().await;
    slow.finished.notify_one();
    "slow"
}

#[rocket::async_test]
async fn close_lets_handler_finish() {
    let state = Arc::new(Slow::default());
    let server = Server::launch(rocket().manage(state.clone()).mount("/", routes![slow])).await;
    let mut client = Client::connect(server.addr).await;
    let _stream = client.request("GET", "/slow", None).await;
    state.entered.notified().await;

    // Give the server time to see the close before releasing the handler.
    client.close().await;
    rocket::tokio::time::sleep(Duration::from_millis(200)).await;
    state.release.notify_one();
    timeout(Duration::from_secs(2), state.finished.notified()).await
        .expect("handler did not finish after HTTP/3 connection was closed");

    server.shutdown().await;
}

#[rocket::async_test]
async fn shutdown_waits_for_requests_in_flight() {
    let state = Arc::new(Slow::default());
    let server = Server::launch(rocket().manage(state.clone()).mount("/", routes![slow])).await;
    let mut client = Client::connect(server.addr).await;
    let mut stream = client.request("GET", "/slow", None).await;
    state.entered.notified().await;

    let start = std::time::Instant::now();
    server.shutdown.notify();
    rocket::tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!server.task.is_finished());

    state.release.notify_one();
    let head = stream.recv_response().await.unwrap();
    assert_eq!(Response { head, stream }.into_string().await, "slow");
    client.close().await;
    server.task.await.unwrap();
    assert!(start.elapsed() < Duration::from_secs(1), "shutdown took {:?}", start.elapsed());
}

#[rocket::async_test]
async fn shutdown_fairings_run_with_requests_in_flight() {
    let state = Arc::new(Slow::default());
    let fairing_ran = Arc::new(Notify::new());
    let rocket = rocket().manage(state.clone()).mount("/", routes![slow])
        .attach(AdHoc::on_shutdown("Notify", {
            let fairing_ran = fairing_ran.clone();
            move |_| Box::pin(async move { fairing_ran.notify_one(); })
        }));

    let server = Server::launch(rocket).await;
    let mut client = Client::connect(server.addr).await;
    let mut stream = client.request("GET", "/slow", None).await;
    state.entered.notified().await;

    server.shutdown.notify();
    timeout(Duration::from_millis(500), fairing_ran.notified()).await
        .expect("shutdown fairing did not run while a request was in flight");

    state.release.notify_one();
    let head = stream.recv_response().await.unwrap();
    assert_eq!(Response { head, stream }.into_string().await, "slow");
    client.close().await;
    timeout(Duration::from_secs(5), server.task).await.unwrap().unwrap();
}

#[rocket::async_test]
async fn goaway_allows_requests_in_flight() {
    let state = Arc::new(Slow::default());
    let server = Server::launch(rocket().manage(state.clone()).mount("/", routes![slow])).await;
    let client = RawClient::connect(server.addr).await;
    let mut recv = client.get("/slow").await;
    state.entered.notified().await;

    // The request on stream 0 was accepted, so the first refused is stream 4.
    server.shutdown.notify();
    assert_eq!(timeout(Duration::from_secs(5), client.goaway()).await.unwrap(), 4);
    state.release.notify_one();
    let frames = timeout(Duration::from_secs(5), raw_frames(&mut recv)).await.unwrap().unwrap();
    assert_eq!(frames.last(), Some(&(0x00, b"slow".to_vec())));
    client.close().await;
    timeout(Duration::from_secs(5), server.task).await.unwrap().unwrap();
}

#[rocket::async_test]
async fn goaway_follows_last_request() {
    // Completed requests ensure the connection is being served at shutdown.
    let server = Server::launch(rocket().mount("/", routes![ready])).await;
    let client = RawClient::connect(server.addr).await;
    for _ in 0..2 {
        let frames = raw_frames(&mut client.get("/ready").await).await.unwrap();
        assert_eq!(frames.last(), Some(&(0x00, b"ready".to_vec())));
    }

    server.shutdown.notify();
    assert_eq!(timeout(Duration::from_secs(5), client.goaway()).await.unwrap(), 8);
    client.close().await;
    timeout(Duration::from_secs(5), server.task).await.unwrap().unwrap();
}

#[get("/large")]
fn large() -> Vec<u8> {
    vec![b'a'; 200_000]
}

#[rocket::async_test]
async fn client_goaway_lets_responses_finish() {
    let server = Server::launch(rocket().mount("/", routes![large])).await;
    let mut client = RawClient::connect(server.addr).await;
    let mut recv = client.get("/large").await;
    client.send_goaway().await;

    let frames = timeout(Duration::from_secs(5), raw_frames(&mut recv)).await.unwrap().unwrap();
    let body: usize = frames.iter().filter(|(kind, _)| *kind == 0x00).map(|(_, d)| d.len()).sum();
    assert_eq!(body, 200_000);
    client.close().await;
    server.shutdown().await;
}

#[rocket::async_test]
async fn shutdown_cancels_responses_after_grace() {
    let signals = Arc::new(Signals::default());
    let rocket = rocket();
    let figment = rocket.figment().clone().merge(("shutdown.grace", 1));
    let rocket = rocket.reconfigure(figment).manage(signals.clone()).mount("/", routes![stalled]);
    let server = Server::launch(rocket).await;
    let mut client = Client::connect(server.addr).await;
    let mut response = client.get("/stalled?sized=false").await;
    assert_eq!(response.chunk().await.unwrap(), "hello");
    signals.stalled.notified().await;

    // When the grace period ends, the stream is reset and the connection is
    // closed. Either way, the response must not appear complete.
    server.shutdown.notify();
    let (rest, end) = timeout(Duration::from_secs(3), response.body()).await.unwrap();
    assert!(rest.is_empty());
    assert_ne!(end, End::Finished);
    timeout(Duration::from_secs(1), signals.dropped.notified()).await.unwrap();
    client.close().await;
    server.task.await.unwrap();
}

#[rocket::async_test]
async fn alt_svc_is_per_instance() {
    use rocket::local::asynchronous::Client as LocalClient;

    // A local client has no QUIC endpoint, so it doesn't advertise HTTP/3.
    let client = LocalClient::untracked(rocket().mount("/", routes![ready])).await.unwrap();
    assert_eq!(client.get("/ready").dispatch().await.headers().get_one("Alt-Svc"), None);

    for _ in 0..2 {
        let server = Server::launch(rocket().mount("/", routes![ready])).await;
        let mut client = Client::connect(server.addr).await;
        let alt_svc = format!("h3=\":{}\"", server.addr.port());
        assert_eq!(client.get("/ready").await.header("Alt-Svc"), Some(&*alt_svc));
        client.close().await;
        server.shutdown().await;
    }
}

#[catch(400)]
fn bad_request() -> &'static str {
    "bad request"
}

#[rocket::async_test]
async fn bad_requests_are_answered() {
    let server = Server::launch(rocket().register("/", catchers![bad_request])).await;
    let client = Client::connect(server.addr).await;

    // A request without a path is answered by the 400 catcher. h3's client
    // sends CONNECT with a path, so the request is written directly.
    let (mut send, mut recv) = client.raw().await;
    let request = [(":method", "CONNECT"), (":authority", "example.com:443")];
    send.write_all(&headers_frame(&request)).await.unwrap();
    send.finish().unwrap();

    // The QPACK block starts with `:status` 400, static table entry 67.
    let frames = timeout(Duration::from_secs(5), raw_frames(&mut recv)).await.unwrap().unwrap();
    assert!(frames[0].0 == 0x01 && frames[0].1.starts_with(&[0, 0, 0xff, 0x04]));
    assert_eq!(frames[1..], [(0x00, b"bad request".to_vec())]);

    client.close().await;
    server.shutdown().await;
}

#[post("/", data = "<data>")]
async fn read_body(mut data: rocket::Data<'_>, entered: &rocket::State<Arc<Notify>>) -> String {
    use rocket::data::ToByteUnit;
    use rocket::tokio::io::AsyncReadExt;

    // quinn discards unread data once a stream is reset, so the body is read
    // before the client resets it. A failed peek must then not make the
    // following read end without an error.
    data.peek(5).await;
    entered.notify_one();
    data.peek(512).await;
    let mut body = vec![];
    let result = data.open(1.mebibytes()).read_to_end(&mut body).await;
    format!("{}:{}", String::from_utf8_lossy(&body), result.is_ok())
}

#[rocket::async_test]
async fn request_body_errors_are_not_eof() {
    let entered = Arc::new(Notify::new());
    let rocket = rocket().manage(entered.clone()).mount("/", routes![read_body]);
    let server = Server::launch(rocket).await;
    let mut client = Client::connect(server.addr).await;
    for reset in [false, true] {
        let request = http::Request::builder()
            .method("POST")
            .uri("https://localhost/")
            .body(()).unwrap();

        let mut stream = client.requests.send_request(request).await.unwrap();
        stream.send_data(Bytes::from_static(b"hello")).await.unwrap();
        entered.notified().await;
        match reset {
            true => stream.stop_stream(Code::H3_REQUEST_CANCELLED),
            false => stream.finish().await.unwrap(),
        }

        let head = stream.recv_response().await.unwrap();
        let body = Response { head, stream }.into_string().await;
        assert_eq!(body, format!("hello:{}", !reset));
    }

    client.close().await;
    server.shutdown().await;
}

#[derive(Default)]
struct Pending {
    reading: Notify,
    returned: Notify,
}

#[post("/pending", data = "<data>")]
async fn pending_body(mut data: rocket::Data<'_>, pending: &rocket::State<Arc<Pending>>) {
    data.peek(5).await;
    pending.reading.notify_one();
    data.peek(512).await;
    pending.returned.notify_one();
}

#[rocket::async_test]
async fn shutdown_cancels_pending_body_reads() {
    let pending = Arc::new(Pending::default());
    let rocket = rocket();
    let figment = rocket.figment().clone().merge(("shutdown.grace", 1));
    let rocket = rocket.reconfigure(figment).manage(pending.clone())
        .mount("/", routes![pending_body]);

    let server = Server::launch(rocket).await;
    let mut client = Client::connect(server.addr).await;
    let request = http::Request::builder()
        .method("POST")
        .uri("https://localhost/pending")
        .body(()).unwrap();

    let mut stream = client.requests.send_request(request).await.unwrap();
    stream.send_data(Bytes::from_static(b"hello")).await.unwrap();
    pending.reading.notified().await;

    // The read is cancelled when the grace period ends, and the handler
    // returns. The response may not be sent as the grace period has ended.
    server.shutdown.notify();
    timeout(Duration::from_secs(5), pending.returned.notified()).await
        .expect("handler did not return after its read was cancelled");

    client.close().await;
    timeout(Duration::from_secs(5), server.task).await.unwrap().unwrap();
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
