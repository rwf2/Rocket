#![feature(proc_macro_hygiene, async_await)]

#[macro_use] extern crate rocket;

use rocket::{http::Status, response::content};

#[get("/empty")]
fn empty() -> Status {
    Status::NoContent
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[head("/other")]
fn other() -> content::Json<&'static str> {
    content::Json("{ 'hi': 'hello' }")
}

mod head_handling_tests {
    use super::*;

    use futures::io::AsyncReadExt;

    use rocket::Route;
    use rocket::local::Client;
    use rocket::http::{Status, ContentType};
    use rocket::response::Body;

    fn routes() -> Vec<Route> {
        routes![index, empty, other]
    }

    fn assert_empty_sized_body<T: futures::AsyncRead + Unpin>(body: Body<T>, expected_size: u64) {
        match body {
            Body::Sized(mut body, size) => {
                let mut buffer = vec![];
                tokio::runtime::Runtime::new().expect("create runtime").block_on(async {
                    body.read_to_end(&mut buffer).await.unwrap();
                });
                assert_eq!(size, expected_size);
                assert_eq!(buffer.len(), 0);
            }
            _ => panic!("Expected a sized body.")
        }
    }

    #[test]
    fn auto_head() {
        let client = Client::new(rocket::ignite().mount("/", routes())).unwrap();
        let mut response = client.head("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_empty_sized_body(response.body().unwrap(), 13);

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::Plain.to_string()]);

        let mut response = client.head("/empty").dispatch();
        assert_eq!(response.status(), Status::NoContent);
        assert!(response.body_bytes_wait().is_none());
    }

    #[test]
    fn user_head() {
        let client = Client::new(rocket::ignite().mount("/", routes())).unwrap();
        let mut response = client.head("/other").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_empty_sized_body(response.body().unwrap(), 17);

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::JSON.to_string()]);
    }
}
