#![cfg(feature = "tungstenite")]

use rocket::http::{Header, Status};
use rocket::local::asynchronous::{Client, LocalRequest};
use rocket::request::Outcome;
use rocket_ws::WebSocket;

fn request<'c>(client: &'c Client, key: Option<&'static str>) -> LocalRequest<'c> {
    let mut request = client.get("/").header(Header::new("Sec-WebSocket-Version", "13"));
    if let Some(key) = key {
        request.add_header(Header::new("Sec-WebSocket-Key", key));
    }

    request
}

#[rocket::async_test]
async fn valid_key_and_upgrade_headers() {
    let client = Client::debug(rocket::build()).await.unwrap();
    for (case, connection, upgrade) in [
        ("lowercase", &["upgrade"][..], &["websocket"][..]),
        ("mixed-case lists", &["keep-alive, Upgrade"][..], &["h2c, WebSocket"][..]),
        ("repeated headers", &["keep-alive", "UpGrAdE"][..], &["h2c", "WebSocket"][..]),
    ] {
        // RFC 6455's sample nonce and corresponding accept key.
        let mut request = request(&client, Some("dGhlIHNhbXBsZSBub25jZQ=="));
        for value in connection {
            request.add_header(Header::new("Connection", *value));
        }

        for value in upgrade {
            request.add_header(Header::new("Upgrade", *value));
        }

        let Outcome::Success(socket) = request.inner().guard::<WebSocket>().await else {
            panic!("expected a valid WebSocket request: {case}");
        };

        assert_eq!(socket.accept_key(), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=", "{case}");
    }
}

#[rocket::async_test]
async fn invalid_keys_forward_bad_request() {
    let client = Client::debug(rocket::build()).await.unwrap();
    for (case, key) in [
        ("missing", None),
        ("empty", Some("")),
        ("invalid alphabet", Some("!!!!!!!!!!!!!!!!!!!!!!!!")),
        ("URL-safe alphabet", Some("_____________________w==")),
        ("invalid padding", Some("dGhlIHNhbXBsZSBub25jZQ=A")),
        ("nonzero padding bits", Some("dGhlIHNhbXBsZSBub25jZR==")),
        ("15 zero bytes", Some("AAAAAAAAAAAAAAAAAAAA")),
        ("17 zero bytes", Some("AAAAAAAAAAAAAAAAAAAAAAA=")),
    ] {
        let request = request(&client, key)
            .header(Header::new("Connection", "upgrade"))
            .header(Header::new("Upgrade", "websocket"));

        let outcome = request.inner().guard::<WebSocket>().await;
        assert!(matches!(outcome, Outcome::Forward(Status::BadRequest)), "{case}");
    }
}
