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

    use rocket::Route;
    use rocket::local::asynchronous::Client;
    use rocket::http::{Status, ContentType};

    fn routes() -> Vec<Route> {
        routes![index, empty, other]
    }

    #[rocket::async_test]
    async fn auto_head() {
        let client = Client::new(rocket::ignite().mount("/", routes())).await.unwrap();
        let response = client.head("/").dispatch().await;

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::Plain.to_string()]);
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body().unwrap().known_size(), Some(13));
        assert!(response.into_bytes().await.unwrap().is_empty());

        let response = client.head("/empty").dispatch().await;
        assert_eq!(response.status(), Status::NoContent);
        assert!(response.into_bytes().await.is_none());
    }

    #[rocket::async_test]
    async fn user_head() {
        let client = Client::new(rocket::ignite().mount("/", routes())).await.unwrap();
        let response = client.head("/other").dispatch().await;

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::JSON.to_string()]);
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body().unwrap().known_size(), Some(17));
        assert!(response.into_bytes().await.unwrap().is_empty());
    }
}
