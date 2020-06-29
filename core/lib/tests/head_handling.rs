#![feature(proc_macro_hygiene)]

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

    use tokio::io::AsyncReadExt;

    use rocket::Route;
    use rocket::local::Client;
    use rocket::http::{Status, ContentType};
    use rocket::response::ResponseBody;

    fn routes() -> Vec<Route> {
        routes![index, empty, other]
    }

    async fn assert_empty_sized_body(body: &mut ResponseBody<'_>, expected_size: usize) {
        let size = body.size().await.expect("sized body");
        assert_eq!(size, expected_size);

        let mut buffer = vec![];
        body.as_reader().read_to_end(&mut buffer).await.unwrap();
        assert_eq!(buffer.len(), 0);
    }

    #[rocket::async_test]
    async fn auto_head() {
        let client = Client::new(rocket::ignite().mount("/", routes())).await.unwrap();
        let mut response = client.head("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_empty_sized_body(response.body().unwrap(), 13).await;

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::Plain.to_string()]);

        let mut response = client.head("/empty").dispatch().await;
        assert_eq!(response.status(), Status::NoContent);
        assert!(response.body_bytes().await.is_none());
    }

    #[rocket::async_test]
    async fn user_head() {
        let client = Client::new(rocket::ignite().mount("/", routes())).await.unwrap();
        let mut response = client.head("/other").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_empty_sized_body(response.body().unwrap(), 17).await;

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::JSON.to_string()]);
    }
}
