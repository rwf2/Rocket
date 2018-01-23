#![feature(plugin)]
#![plugin(rocket_codegen)]
#![feature(decl_macro)]

extern crate brotli;
extern crate rocket;
extern crate rocket_contrib;

use rocket::local::Client;
use rocket::http::Status;
use rocket::response::Response;
use rocket::http::{ContentType, Header};

use std::io::Cursor;

const HELLO: &str = "Hello world!";

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, gzip, br])
        .attach(rocket_contrib::BrotliFairing::fairing())
}

#[get("/")]
pub fn index() -> String {
    String::from("Hello world!")
}
#[get("/gzip")]
pub fn gzip() -> Response<'static> {
    Response::build()
        .header(ContentType::Plain)
        .header(Header::new("Content-Encoding", "gzip"))
        .sized_body(Cursor::new(String::from(HELLO)))
        .finalize()
}
#[get("/br")]
pub fn br() -> Response<'static> {
    Response::build()
        .header(ContentType::Plain)
        .header(Header::new("Content-Encoding", "br"))
        .sized_body(Cursor::new(String::from(HELLO)))
        .finalize()
}

#[test]
fn test_index() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/")
        .header(Header::new("Accept-Encoding", "deflate"))
        .header(Header::new("Accept-Encoding", "gzip"))
        .header(Header::new("Accept-Encoding", "br"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == String::from("br"))
    );
    let mut body_plain = Cursor::new(Vec::<u8>::new());
    brotli::BrotliDecompress(
        &mut Cursor::new(response.body_bytes().unwrap()),
        &mut body_plain,
    ).unwrap();
    assert_eq!(
        String::from_utf8(body_plain.get_mut().to_vec()).unwrap(),
        String::from(HELLO)
    );
}

#[test]
fn test_gzip() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/gzip")
        .header(Header::new("Accept-Encoding", "deflate"))
        .header(Header::new("Accept-Encoding", "gzip"))
        .header(Header::new("Accept-Encoding", "br"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(!response
        .headers()
        .get("Content-Encoding")
        .any(|x| x == String::from("br")));
    assert_eq!(
        String::from_utf8(response.body_bytes().unwrap()).unwrap(),
        String::from(HELLO)
    );
}

#[test]
fn test_br() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/br")
        .header(Header::new("Accept-Encoding", "deflate"))
        .header(Header::new("Accept-Encoding", "gzip"))
        .header(Header::new("Accept-Encoding", "br"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == String::from("br"))
    );
    let mut body_plain = Cursor::new(Vec::<u8>::new());
    brotli::BrotliDecompress(
        &mut Cursor::new(response.body_bytes().unwrap()),
        &mut body_plain,
    ).unwrap();
    assert_eq!(
        String::from_utf8(body_plain.get_mut().to_vec()).unwrap(),
        String::from(HELLO)
    );
}

#[test]
fn test_br_not_accepted() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/br")
        .header(Header::new("Accept-Encoding", "deflate"))
        .header(Header::new("Accept-Encoding", "gzip"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(!response
        .headers()
        .get("Content-Encoding")
        .any(|x| x == String::from("br")));
    assert_eq!(
        String::from_utf8(response.body_bytes().unwrap()).unwrap(),
        String::from(HELLO)
    );
}
