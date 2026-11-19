#[macro_use] extern crate rocket;

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use rocket::Request;
use rocket::http::uri::Origin;
use rocket::fairing::AdHoc;
use rocket::listener::tcp::TcpListener;
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::{net::TcpStream, sync::oneshot, time::timeout};

#[get("/admin")]
fn admin(uri: &Origin<'_>) -> String {
    format!("admin: {uri}")
}

#[get("/<name>")]
fn name(name: &str, uri: &Origin<'_>) -> String {
    format!("name: {name} {uri}")
}

#[catch(400)]
fn bad_request(req: &Request<'_>) -> String {
    format!("400: {}", req.uri())
}

#[catch(404)]
fn not_found(req: &Request<'_>) -> String {
    format!("404: {}", req.uri())
}

// Returns the status code and the (dechunked) body of the response.
async fn send(port: u16, method: &str, target: &[u8]) -> (u16, String) {
    let mut request = format!("{method} ").into_bytes();
    request.extend_from_slice(target);
    request.extend_from_slice(b" HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");

    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream.write_all(&request).await.unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let (head, mut body) = response.split_once("\r\n\r\n").unwrap();
    let status = head[9..12].parse().unwrap();
    if !head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        return (status, body.to_string());
    }

    let mut dechunked = String::new();
    while let Some((size, rest)) = body.split_once("\r\n") {
        let size = usize::from_str_radix(size, 16).unwrap();
        dechunked.push_str(&rest[..size]);
        body = rest[size..].trim_start_matches("\r\n");
    }

    (status, dechunked)
}

async fn check_request_targets() {
    let (tx, rx) = oneshot::channel();
    let mut config = rocket::Config::debug_default();
    config.shutdown.ctrlc = false;
    let rocket = rocket::custom(config)
        .mount("/", routes![admin, name])
        .register("/", catchers![bad_request, not_found])
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            let port = rocket.endpoints().find_map(|e| e.tcp()).unwrap().port();
            tx.send((port, rocket.shutdown())).unwrap();
        })));

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let server = rocket::tokio::spawn(rocket.try_launch_on(TcpListener::bind(addr)));
    let (port, shutdown) = rx.await.unwrap();

    let cases: &[(&str, &[u8], u16, &str)] = &[
        ("GET", b"/admin", 200, "admin: /admin"),
        ("GET", b"http://example.com/admin", 200, "admin: /admin"),
        ("GET", b"/a%C3%A4", 200, "name: a\u{e4} /a%C3%A4"),
        ("OPTIONS", b"*", 404, "404: *"),

        // Targets without a path are dispatched to the 400 catcher.
        ("CONNECT", b"example.com:443", 400, "400: /"),
        ("GET", b"admin", 400, "400: /"),

        // hyper rejects these targets itself; no catcher runs.
        ("GET", b"/a\xFF", 400, ""),
        ("GET", b"/a<b>", 400, ""),
        ("GET", b"/a?q=\xFF", 400, ""),

        // hyper accepts these targets though `Origin::parse()` rejects them.
        ("GET", b"/admin\xC2\xA0", 200, "name: admin\u{a0} /admin\u{a0}"),
        ("GET", b"/h\xC3\xA4cker", 200, "name: h\u{e4}cker /h\u{e4}cker"),
        ("GET", b"/a|b", 200, "name: a|b /a|b"),
        ("GET", b"/a?q=\xC3\xA4", 200, "name: a /a?q=\u{e4}"),
    ];

    for &(method, target, status, body) in cases {
        let response = send(port, method, target).await;
        assert_eq!(response, (status, body.to_string()), "{method} {:?}", target.escape_ascii());
    }

    shutdown.notify();
    server.await.unwrap().unwrap();
}

#[rocket::async_test]
async fn request_targets_are_routed_or_rejected() {
    timeout(Duration::from_secs(10), check_request_targets()).await.unwrap();
}
