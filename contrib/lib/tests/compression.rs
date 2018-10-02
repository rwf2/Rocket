#![cfg(all(feature = "brotli_compression", feature = "gzip_compression"))]
#![feature(plugin)]
#![plugin(rocket_codegen)]
#![feature(decl_macro)]
#![feature(proc_macro_non_items)]

extern crate brotli;
extern crate flate2;
extern crate rocket;
extern crate rocket_contrib;

use rocket::http::hyper::header::{ContentEncoding, Encoding};
use rocket::http::Status;
use rocket::http::{ContentType, Header};
use rocket::local::Client;
use rocket::response::Response;
use rocket::routes;

use std::io::Cursor;
use std::io::Read;

use flate2::read::{GzDecoder, GzEncoder};

const HELLO: &str = r"This is a message to hello with more than 100 bytes \
    in order to have to read more than one buffer when gzipping. こんにちは!";

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, font, image, already_encoded, chunked])
        .attach(rocket_contrib::Compression::fairing())
}

#[get("/")]
pub fn index() -> String {
    String::from(HELLO)
}
#[get("/font")]
pub fn font() -> Response<'static> {
    Response::build()
        .header(ContentType::WOFF)
        .sized_body(Cursor::new(String::from(HELLO)))
        .finalize()
}
#[get("/image")]
pub fn image() -> Response<'static> {
    Response::build()
        .header(ContentType::PNG)
        .sized_body(Cursor::new(String::from(HELLO)))
        .finalize()
}
#[get("/already_encoded")]
pub fn already_encoded() -> Response<'static> {
    let mut encoder = GzEncoder::new(
        Cursor::new(String::from(HELLO)),
        flate2::Compression::default(),
    );
    let mut encoded = Vec::new();
    encoder.read_to_end(&mut encoded).unwrap();
    Response::build()
        .header(ContentEncoding(vec![Encoding::Gzip]))
        .sized_body(Cursor::new(encoded))
        .finalize()
}
#[get("/chunked")]
pub fn chunked() -> Response<'static> {
    Response::build()
        .header(ContentEncoding(vec![Encoding::Chunked]))
        .sized_body(Cursor::new(String::from(HELLO)))
        .finalize()
}

// Tests

/// This function should compress the content in br
#[test]
fn test_index() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/")
        .header(Header::new("Accept-Encoding", "deflate, gzip, brotli"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br")
    );
    let mut body_plain = Cursor::new(Vec::<u8>::new());
    brotli::BrotliDecompress(
        &mut Cursor::new(response.body_bytes().unwrap()),
        &mut body_plain,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(body_plain.get_mut().to_vec()).unwrap(),
        String::from(HELLO)
    );
}

/// This function should not compress the content because it is already encoded
#[test]
fn test_already_encoded() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/already_encoded")
        .header(Header::new("Accept-Encoding", "deflate, gzip, brotli"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        !response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br")
    );
    assert!(
        response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "gzip")
    );
    let mut s = String::new();
    GzDecoder::new(&response.body_bytes().unwrap()[..])
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, String::from(HELLO));
}

/// This function should compress the content in br because the ContentEncoding
/// is chunked, not a compression encoding
#[test]
fn test_chunked() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/chunked")
        .header(Header::new("Accept-Encoding", "deflate, gzip, brotli"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br")
    );
    let mut body_plain = Cursor::new(Vec::<u8>::new());
    brotli::BrotliDecompress(
        &mut Cursor::new(response.body_bytes().unwrap()),
        &mut body_plain,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(body_plain.get_mut().to_vec()).unwrap(),
        String::from(HELLO)
    );
}

/// This function should compress the content in br (test font mode)
#[test]
fn test_br_font() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/font")
        .header(Header::new("Accept-Encoding", "deflate, gzip, brotli"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br")
    );
    let mut body_plain = Cursor::new(Vec::<u8>::new());
    brotli::BrotliDecompress(
        &mut Cursor::new(response.body_bytes().unwrap()),
        &mut body_plain,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(body_plain.get_mut().to_vec()).unwrap(),
        String::from(HELLO)
    );
}

/// This function should not compress because images are not compressed
#[test]
fn test_br_image() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/image")
        .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        !response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br")
    );
    assert_eq!(
        String::from_utf8(response.body_bytes().unwrap()).unwrap(),
        String::from(HELLO)
    );
}

/// This function should not compress because images are not compressed
#[test]
fn test_gzip_image() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/image")
        .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        !response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "gzip")
    );
    assert_eq!(
        String::from_utf8(response.body_bytes().unwrap()).unwrap(),
        String::from(HELLO)
    );
}

/// This function should compress the content in gzip becasue br is not accepted
#[test]
fn test_br_not_accepted() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/")
        .header(Header::new("Accept-Encoding", "deflate, gzip"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        !response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br")
    );
    assert!(
        response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "gzip")
    );
    let mut s = String::new();
    GzDecoder::new(&response.body_bytes().unwrap()[..])
        .read_to_string(&mut s)
        .unwrap();
    assert_eq!(s, String::from(HELLO));
}

/// This function should not compress because gzip and br are not accepted
#[test]
fn test_br_nor_gzip_not_accepted() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/")
        .header(Header::new("Accept-Encoding", "deflate"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        !response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br" || x == "gzip")
    );
    assert_eq!(
        String::from_utf8(response.body_bytes().unwrap()).unwrap(),
        String::from(HELLO)
    );
}

// Test with identity Accept-Encoding, it should not compress
#[test]
fn test_identity() {
    let client = Client::new(rocket()).expect("valid rocket instance");
    let mut response = client
        .get("/")
        .header(Header::new("Accept-Encoding", "identity"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(
        !response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "gzip" || x == "br")
    );
    assert_eq!(
        String::from_utf8(response.body_bytes().unwrap()).unwrap(),
        String::from(HELLO)
    );
}
