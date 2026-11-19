// An upgraded connection belongs to its I/O handler: shutting down writes must
// not discard data the peer sends afterwards, even when lingering is enabled.
#[macro_use] extern crate rocket;

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Duration;

use rocket::{Request, Response, State};
use rocket::data::{IoHandler, IoStream};
use rocket::fairing::AdHoc;
use rocket::listener::tcp::TcpListener;
use rocket::response::{self, Responder};
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::net::TcpStream;
use rocket::tokio::sync::oneshot;
use rocket::tokio::time::timeout;

type Tx = Mutex<Option<oneshot::Sender<Vec<u8>>>>;

struct HalfClose(oneshot::Sender<Vec<u8>>);

#[rocket::async_trait]
impl IoHandler for HalfClose {
    async fn io(self: Box<Self>, mut io: IoStream) -> io::Result<()> {
        io.write_all(b"server-done").await?;
        // Half-close: signal EOF to the peer, then read what it sends back.
        io.shutdown().await?;
        let mut rest = Vec::new();
        io.read_to_end(&mut rest).await?;
        let _ = self.0.send(rest);
        Ok(())
    }
}

impl<'r> Responder<'r, 'r> for HalfClose {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build().upgrade("raw", self).ok()
    }
}

#[get("/")]
fn upgrade(tx: &State<Tx>) -> HalfClose {
    HalfClose(tx.lock().unwrap().take().unwrap())
}

#[rocket::async_test]
async fn data_sent_after_half_close_is_read() {
    let (tx, rx) = oneshot::channel::<Vec<u8>>();
    let (ptx, prx) = oneshot::channel();
    let mut config = rocket::Config::debug_default();
    config.shutdown.ctrlc = false;
    let rocket = rocket::custom(config)
        .manage(Mutex::new(Some(tx)))
        .mount("/", routes![upgrade])
        .attach(AdHoc::on_liftoff("port", |r| Box::pin(async move {
            let port = r.endpoints().find_map(|e| e.tcp()).unwrap().port();
            ptx.send((port, r.shutdown())).unwrap();
        })));

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let server = rocket::tokio::spawn(rocket.try_launch_on(TcpListener::bind(addr)));
    let (port, shutdown) = prx.await.unwrap();

    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\nHost: x\r\nConnection: upgrade\r\nUpgrade: raw\r\n\r\n")
        .await.unwrap();

    // Read until the server half-closes, then reply and close.
    let mut head = Vec::new();
    timeout(Duration::from_secs(5), stream.read_to_end(&mut head)).await.unwrap().unwrap();
    assert!(head.ends_with(b"server-done"));
    stream.write_all(b"client-reply").await.unwrap();
    stream.shutdown().await.unwrap();
    drop(stream);

    let got = timeout(Duration::from_secs(15), rx).await.expect("handler finished").unwrap();
    shutdown.notify();
    let _ = server.await;
    assert_eq!(got, b"client-reply");
}
