#[macro_use] extern crate rocket;

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use rocket::Request;
use rocket::config::Ident;
use rocket::fairing::AdHoc;
use rocket::http::Cookie;
use rocket::listener::tcp::TcpListener;
use rocket::tokio::io::{AsyncReadExt, AsyncWriteExt};
use rocket::tokio::{net::TcpStream, sync::oneshot, time::timeout};

struct RequestFairingRan(bool);

#[catch(400)]
fn bad_request(req: &Request<'_>) -> &'static str {
    req.cookies().add(Cookie::new("caught", "400"));
    "bad request"
}

#[catch(404)]
fn not_found(req: &Request<'_>) -> &'static str {
    req.cookies().add(Cookie::new("caught", "404"));
    "not found"
}

#[derive(Debug, PartialEq)]
struct Sent {
    status: u16,
    server: Option<String>,
    set_cookie: Option<String>,
    request_fairing: Option<String>,
    response_fairing: Option<String>,
    content_type_options: Option<String>,
    content_length: Option<String>,
    transfer_encoding: Option<String>,
    body: String,
}

async fn send(port: u16, method: &str, target: &str) -> Sent {
    let request = format!("{method} {target} HTTP/1.1\r\n\
        Host: localhost\r\nConnection: close\r\n\r\n");

    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    let header = |name: &str| head.lines()
        .filter_map(|line| line.split_once(": "))
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.to_string());

    Sent {
        status: head[9..12].parse().unwrap(),
        server: header("Server"),
        set_cookie: header("Set-Cookie"),
        request_fairing: header("X-Request-Fairing"),
        response_fairing: header("X-Response-Fairing"),
        content_type_options: header("X-Content-Type-Options"),
        content_length: header("Content-Length"),
        transfer_encoding: header("Transfer-Encoding"),
        body: body.to_string(),
    }
}

async fn check_bad_request_responses() {
    let (tx, rx) = oneshot::channel();
    let mut config = rocket::Config::debug_default();
    config.ident = Ident::try_new("Rocket Test").unwrap();
    config.shutdown.ctrlc = false;
    let rocket = rocket::custom(config)
        .register("/", catchers![bad_request, not_found])
        .attach(AdHoc::on_request("Mark Request", |req, _| Box::pin(async move {
            req.local_cache(|| RequestFairingRan(true));
        })))
        .attach(AdHoc::on_response("Mark Response", |req, res| Box::pin(async move {
            let ran = req.local_cache(|| RequestFairingRan(false)).0;
            res.set_raw_header("X-Request-Fairing", ran.to_string());
            res.set_raw_header("X-Response-Fairing", "true");
        })))
        .attach(AdHoc::on_liftoff("Send Port and Shutdown", |rocket| Box::pin(async move {
            let port = rocket.endpoints().find_map(|e| e.tcp()).unwrap().port();
            tx.send((port, rocket.shutdown())).unwrap();
        })));

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let server = rocket::tokio::spawn(rocket.try_launch_on(TcpListener::bind(addr)));
    let (port, shutdown) = rx.await.unwrap();

    // Requests without a path are answered by the 400 catcher without being
    // routed. The response is otherwise finished like the routed 404.
    let cases = [
        ("CONNECT", "example.com:443", 400, "bad request", 11),
        ("GET", "admin", 400, "bad request", 11),
        ("HEAD", "admin", 400, "", 11),
        ("GET", "/missing", 404, "not found", 9),
        ("HEAD", "/missing", 404, "", 9),
    ];

    for (method, target, status, body, length) in cases {
        let expected = Sent {
            status,
            server: Some("Rocket Test".into()),
            set_cookie: Some(format!("caught={status}; SameSite=Strict; Path=/")),
            request_fairing: Some("true".into()),
            response_fairing: Some("true".into()),
            content_type_options: Some("nosniff".into()),
            content_length: Some(length.to_string()),
            transfer_encoding: None,
            body: body.into(),
        };

        assert_eq!(send(port, method, target).await, expected, "{method} {target}");
    }

    shutdown.notify();
    server.await.unwrap().unwrap();
}

#[rocket::async_test]
async fn bad_requests_are_answered_like_other_errors() {
    timeout(Duration::from_secs(10), check_bad_request_responses()).await.unwrap();
}
