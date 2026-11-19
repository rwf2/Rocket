use crate::prelude::*;

use std::net::{Ipv4Addr, SocketAddr};

use rocket::tokio::net::TcpListener;
use rocket::{get, post, routes, Rocket};
use rocket::data::Data;
use rocket::http::Status;
use rocket::listener::Endpoint;
use rocket::tls::TlsListener;

use reqwest::tls::TlsInfo;

#[get("/")]
fn hello_world(endpoint: &Endpoint) -> String {
    format!("Hello, {endpoint}!")
}

fn test_tls_works() -> Result<()> {
    let mut server = spawn! {
        Rocket::tls_default().mount("/", routes![hello_world])
    }?;

    let client = Client::default();
    let response = client.get(&server, "/")?.send()?;
    let tls = response.extensions().get::<TlsInfo>().unwrap();
    assert!(!tls.peer_certificate().unwrap().is_empty());
    assert!(response.text()?.starts_with("Hello, https://127.0.0.1"));

    server.terminate()?;
    let stdout = server.read_stdout()?;
    assert!(stdout.contains("Rocket has launched on https"));
    assert!(stdout.contains("Graceful shutdown completed"));
    assert!(stdout.contains("GET /"));

    let server = Server::spawn((), |(token, _)| {
        let rocket = rocket::build()
            .reconfigure_with_toml(TLS_CONFIG)
            .mount("/", routes![hello_world]);

        token.with_launch(rocket, |rocket| {
            let config = rocket.figment().extract_inner("tls");
            rocket.try_launch_on(async move {
                let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);
                let listener = TcpListener::bind(addr).await?;
                TlsListener::from(listener, config?).await
            })
        })
    }).unwrap();

    let client = Client::default();
    let response = client.get(&server, "/")?.send()?;
    let tls = response.extensions().get::<TlsInfo>().unwrap();
    assert!(!tls.peer_certificate().unwrap().is_empty());
    assert!(response.text()?.starts_with("Hello, https://127.0.0.1"));

    Ok(())
}

#[post("/", data = "<_data>")]
fn too_large(_data: Data<'_>) -> Status {
    Status::PayloadTooLarge
}

// The client is still sending the body when the response arrives. Closing the
// connection with unread data would reset it and discard the response.
fn test_tls_response_survives_unread_body() -> Result<()> {
    let server = spawn! {
        Rocket::tls_default().mount("/", routes![too_large])
    }?;

    let client: Client = Client::build().http1_only().try_into()?;
    let response = client.post(&server, "/")?.body(vec![b'a'; 16 << 20]).send()?;
    assert_eq!(response.status(), 413);
    Ok(())
}

register!(test_tls_works);
register!(test_tls_response_survives_unread_body);
