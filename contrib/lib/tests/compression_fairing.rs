#![feature(proc_macro_hygiene)]

#[macro_use]
#[cfg(all(feature = "brotli_compression", feature = "gzip_compression"))]
extern crate rocket;

#[cfg(all(feature = "brotli_compression", feature = "gzip_compression"))]
mod compression_fairing_tests {
    use rocket::config::{Config, Environment};
    use rocket::futures::io::Cursor;
    use rocket::futures::prelude::*;
    use rocket::http::Status;
    use rocket::http::{ContentType, Header};
    use rocket::local::Client;
    use rocket::response::{Content, Response};
    use rocket_contrib::compression::Compression;

    use async_compression::futures::bufread::{BrotliDecoder, GzipDecoder, GzipEncoder};

    const HELLO: &str = r"This is a message to hello with more than 100 bytes \
        in order to have to read more than one buffer when gzipping. こんにちは!";

    #[get("/")]
    pub fn index() -> String {
        String::from(HELLO)
    }

    #[get("/font")]
    pub fn font() -> Content<&'static str> {
        Content(ContentType::WOFF, HELLO)
    }

    #[get("/image")]
    pub fn image() -> Content<&'static str> {
        Content(ContentType::PNG, HELLO)
    }

    #[get("/tar")]
    pub fn tar() -> Content<&'static str> {
        Content(ContentType::TAR, HELLO)
    }

    #[get("/already_encoded")]
    pub async fn already_encoded() -> Response<'static> {
        let mut encoder = GzipEncoder::new(Cursor::new(String::from(HELLO)));
        let mut encoded = Vec::new();
        encoder.read_to_end(&mut encoded).await.unwrap();
        Response::build()
            .raw_header("Content-Encoding", "gzip")
            .sized_body(std::io::Cursor::new(encoded))
            .await
            .finalize()
    }

    #[get("/identity")]
    pub async fn identity() -> Response<'static> {
        Response::build()
            .raw_header("Content-Encoding", "identity")
            .sized_body(std::io::Cursor::new(String::from(HELLO)))
            .await
            .finalize()
    }

    fn rocket() -> rocket::Rocket {
        rocket::ignite()
            .mount(
                "/",
                routes![index, font, image, tar, already_encoded, identity],
            )
            .attach(Compression::fairing())
    }

    fn rocket_tar_exception() -> rocket::Rocket {
        let mut table = std::collections::BTreeMap::new();
        table.insert("exclude".to_string(), vec!["application/x-tar"]);
        let config = Config::build(Environment::Development)
            .extra("compress", table)
            .expect("valid configuration");

        rocket::custom(config)
            .mount("/", routes![image, tar])
            .attach(Compression::fairing())
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
        GzipDecoder::new(Cursor::new(response.body_bytes().await.unwrap()))
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
    async fn test_does_not_compress_image() {
        let client = Client::new(rocket()).await.expect("valid rocket instance");
        let mut response = client
            .get("/image")
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

    #[rocket::async_test]
    async fn test_does_not_compress_custom_exception() {
        let client = Client::new(rocket_tar_exception())
            .await
            .expect("valid rocket instance");
        let mut response = client
            .get("/tar")
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
    async fn test_compress_custom_removed_exception() {
        let client = Client::new(rocket_tar_exception())
            .await
            .expect("valid rocket instance");
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
}
