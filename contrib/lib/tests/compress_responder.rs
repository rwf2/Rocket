#![feature(proc_macro_hygiene)]

#[macro_use]
#[cfg(all(feature = "brotli_compression", feature = "gzip_compression"))]
extern crate rocket;

#[cfg(all(feature = "brotli_compression", feature = "gzip_compression"))]
mod compress_responder_tests {
    use std::io::Cursor;

    use rocket::http::Status;
    use rocket::http::{ContentType, Header};
    use rocket::local::Client;
    use rocket::response::{Content, Response};
    use rocket_contrib::compression::Compress;

    use async_compression::tokio_02::bufread::{BrotliDecoder, GzipDecoder, GzipEncoder};
    use tokio::prelude::*;

    const HELLO: &str = r"This is a message to hello with more than 100 bytes \
        in order to have to read more than one buffer when gzipping. こんにちは!";

    #[get("/")]
    pub fn index() -> Compress<String> {
        Compress(String::from(HELLO))
    }

    #[get("/font")]
    pub fn font() -> Compress<Content<&'static str>> {
        Compress(Content(ContentType::WOFF, HELLO))
    }

    #[get("/image")]
    pub fn image() -> Compress<Content<&'static str>> {
        Compress(Content(ContentType::PNG, HELLO))
    }

    #[get("/already_encoded")]
    pub async fn already_encoded() -> Compress<Response<'static>> {
        let mut encoder = GzipEncoder::new(Cursor::new(String::from(HELLO)));
        let mut encoded = Vec::new();
        encoder.read_to_end(&mut encoded).await.unwrap();
        Compress(
            Response::build()
                .raw_header("Content-Encoding", "gzip")
                .sized_body(std::io::Cursor::new(encoded))
                .await
                .finalize(),
        )
    }

    #[get("/identity")]
    pub async fn identity() -> Compress<Response<'static>> {
        Compress(
            Response::build()
                .raw_header("Content-Encoding", "identity")
                .sized_body(std::io::Cursor::new(String::from(HELLO)))
                .await
                .finalize(),
        )
    }

    fn rocket() -> rocket::Rocket {
        rocket::ignite().mount("/", routes![index, font, image, already_encoded, identity])
    }

    #[rocket::async_test]
    async fn test_prioritizes_brotli() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br"));
        let mut body_plain = Vec::new();
        BrotliDecoder::new(Cursor::new(response.body_bytes().await.unwrap()))
            .read_to_end(&mut body_plain)
            .await
            .expect("decompress response");
        assert_eq!(String::from_utf8(body_plain).unwrap(), String::from(HELLO));
    }

    #[rocket::async_test]
    async fn test_br_font() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/font")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br"));
        let mut body_plain = Vec::new();
        BrotliDecoder::new(Cursor::new(response.body_bytes().await.unwrap()))
            .read_to_end(&mut body_plain)
            .await
            .expect("decompress response");
        assert_eq!(String::from_utf8(body_plain).unwrap(), String::from(HELLO));
    }

    #[rocket::async_test]
    async fn test_fallback_gzip() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/")
            .header(Header::new("Accept-Encoding", "deflate, gzip"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(!response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br"));
        assert!(response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "gzip"));
        let mut s = String::new();
        GzipDecoder::new(Cursor::new(response.body_bytes().await.unwrap()))
            .read_to_string(&mut s)
            .await
            .expect("decompress response");
        assert_eq!(s, String::from(HELLO));
    }

    #[rocket::async_test]
    async fn test_does_not_recompress() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/already_encoded")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(!response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br"));
        assert!(response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "gzip"));
        let mut s = String::new();
        GzipDecoder::new(&response.body_bytes().await.unwrap()[..])
            .read_to_string(&mut s)
            .await
            .expect("decompress response");
        assert_eq!(s, String::from(HELLO));
    }

    #[rocket::async_test]
    async fn test_does_not_compress_explicit_identity() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/identity")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(!response
            .headers()
            .get("Content-Encoding")
            .any(|x| x != "identity"));
        assert_eq!(
            String::from_utf8(response.body_bytes().await.unwrap()).unwrap(),
            String::from(HELLO)
        );
    }

    #[rocket::async_test]
    async fn test_ignore_exceptions() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/image")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br"));
        let mut body_plain = Vec::new();
        BrotliDecoder::new(Cursor::new(response.body_bytes().await.unwrap()))
            .read_to_end(&mut body_plain)
            .await
            .expect("decompress response");
        assert_eq!(String::from_utf8(body_plain).unwrap(), String::from(HELLO));
    }

    #[rocket::async_test]
    async fn test_ignores_unimplemented_encodings() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/")
            .header(Header::new("Accept-Encoding", "deflate"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(!response
            .headers()
            .get("Content-Encoding")
            .any(|x| x != "identity"));
        assert_eq!(
            String::from_utf8(response.body_bytes().await.unwrap()).unwrap(),
            String::from(HELLO)
        );
    }

    #[rocket::async_test]
    async fn test_respects_identity_only() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/")
            .header(Header::new("Accept-Encoding", "identity"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(!response
            .headers()
            .get("Content-Encoding")
            .any(|x| x != "identity"));
        assert_eq!(
            String::from_utf8(response.body_bytes().await.unwrap()).unwrap(),
            String::from(HELLO)
        );
    }
}
