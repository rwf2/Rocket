#![feature(decl_macro, proc_macro_hygiene)]

#[macro_use]
extern crate rocket;
extern crate rocket_contrib;

#[cfg(all(feature = "brotli_compression", feature = "gzip_compression"))]
mod compressed_tests {
    extern crate brotli;
    extern crate flate2;

    use rocket::http::hyper::header::{ContentEncoding, Encoding};
    use rocket::http::Status;
    use rocket::http::{ContentType, Header};
    use rocket::local::Client;
    use rocket::response::{Content, Response};
    use rocket::routes;
    use rocket_contrib::compression::Compressed;

    use std::io::Cursor;
    use std::io::Read;

    use self::flate2::read::{GzDecoder, GzEncoder};

    const HELLO: &str = r"This is a message to hello with more than 100 bytes \
        in order to have to read more than one buffer when gzipping. こんにちは!";

    #[get("/")]
    pub fn index() -> Compressed<String> {
        Compressed::new(String::from(HELLO))
    }

    #[get("/font")]
    pub fn font() -> Compressed<Content<&'static str>> {
        Compressed::new(Content(ContentType::WOFF, HELLO))
    }

    #[get("/image")]
    pub fn image() -> Compressed<Content<&'static str>> {
        Compressed::new(Content(ContentType::PNG, HELLO))
    }

    #[get("/already_encoded")]
    pub fn already_encoded() -> Compressed<Response<'static>> {
        let mut encoder = GzEncoder::new(
            Cursor::new(String::from(HELLO)),
            flate2::Compression::default(),
        );
        let mut encoded = Vec::new();
        encoder.read_to_end(&mut encoded).unwrap();
        Compressed::new(
            Response::build()
                .header(ContentEncoding(vec![Encoding::Gzip]))
                .sized_body(Cursor::new(encoded))
                .finalize(),
        )
    }

    #[get("/identity")]
    pub fn identity() -> Compressed<Response<'static>> {
        Compressed::new(
            Response::build()
                .header(ContentEncoding(vec![Encoding::Identity]))
                .sized_body(Cursor::new(String::from(HELLO)))
                .finalize(),
        )
    }

    fn rocket() -> rocket::Rocket {
        rocket::ignite().mount("/", routes![index, font, image, already_encoded, identity])
    }

    #[test]
    fn test_prioritizes_brotli() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client
            .get("/")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br"));
        let mut body_plain = Cursor::new(Vec::<u8>::new());
        brotli::BrotliDecompress(
            &mut Cursor::new(response.body_bytes().unwrap()),
            &mut body_plain,
        )
        .expect("decompress response");
        assert_eq!(
            String::from_utf8(body_plain.get_mut().to_vec()).unwrap(),
            String::from(HELLO)
        );
    }

    #[test]
    fn test_br_font() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client
            .get("/font")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br"));
        let mut body_plain = Cursor::new(Vec::<u8>::new());
        brotli::BrotliDecompress(
            &mut Cursor::new(response.body_bytes().unwrap()),
            &mut body_plain,
        )
        .expect("decompress response");
        assert_eq!(
            String::from_utf8(body_plain.get_mut().to_vec()).unwrap(),
            String::from(HELLO)
        );
    }

    #[test]
    fn test_fallback_gzip() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client
            .get("/")
            .header(Header::new("Accept-Encoding", "deflate, gzip"))
            .dispatch();
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
        GzDecoder::new(&response.body_bytes().unwrap()[..])
            .read_to_string(&mut s)
            .expect("decompress response");
        assert_eq!(s, String::from(HELLO));
    }

    #[test]
    fn test_does_not_recompress() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client
            .get("/already_encoded")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch();
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
        GzDecoder::new(&response.body_bytes().unwrap()[..])
            .read_to_string(&mut s)
            .expect("decompress response");
        assert_eq!(s, String::from(HELLO));
    }

    #[test]
    fn test_does_not_compress_explicit_identity() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client
            .get("/identity")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(!response
            .headers()
            .get("Content-Encoding")
            .any(|x| x != "identity"));
        assert_eq!(
            String::from_utf8(response.body_bytes().unwrap()).unwrap(),
            String::from(HELLO)
        );
    }

    #[test]
    fn test_ignore_exceptions() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client
            .get("/image")
            .header(Header::new("Accept-Encoding", "deflate, gzip, br"))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response
            .headers()
            .get("Content-Encoding")
            .any(|x| x == "br"));
        let mut body_plain = Cursor::new(Vec::<u8>::new());
        brotli::BrotliDecompress(
            &mut Cursor::new(response.body_bytes().unwrap()),
            &mut body_plain,
        )
        .expect("decompress response");
        assert_eq!(
            String::from_utf8(body_plain.get_mut().to_vec()).unwrap(),
            String::from(HELLO)
        );
    }

    #[test]
    fn test_ignores_unimplemented_encodings() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client
            .get("/")
            .header(Header::new("Accept-Encoding", "deflate"))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(!response
            .headers()
            .get("Content-Encoding")
            .any(|x| x != "identity"));
        assert_eq!(
            String::from_utf8(response.body_bytes().unwrap()).unwrap(),
            String::from(HELLO)
        );
    }

    #[test]
    fn test_respects_identity_only() {
        let client = Client::new(rocket()).expect("valid rocket instance");
        let mut response = client
            .get("/")
            .header(Header::new("Accept-Encoding", "identity"))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(!response
            .headers()
            .get("Content-Encoding")
            .any(|x| x != "identity"));
        assert_eq!(
            String::from_utf8(response.body_bytes().unwrap()).unwrap(),
            String::from(HELLO)
        );
    }
}
