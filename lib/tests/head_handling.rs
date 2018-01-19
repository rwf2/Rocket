#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::response::{status, content};

#[get("/empty")]
fn empty() -> status::NoContent {
    status::NoContent
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[head("/other")]
fn other() -> content::Json<&'static str> {
    content::Json("{\"h\": 1}")
}

mod tests {
    use super::*;

    use rocket::Route;
    use rocket::local::Client;
    use rocket::http::{Status, ContentType};
    use rocket::response::Body;

    fn routes() -> Vec<Route> {
        routes![index, empty, other]
    }

    #[test]
    fn auto_head() {
        let client = Client::new(rocket::ignite().mount("/", routes())).unwrap();
        let mut response = client.head("/").dispatch();
        assert_eq!(response.status(), Status::Ok);

        if let Some(body) = response.body() {
            match body {
                Body::Sized(_, n) => assert_eq!(n, "Hello, world!".len() as u64),
                _ => panic!("Expected a sized body!")
            }

            assert_eq!(body.into_string(), Some("".to_string()));
        } else {
            panic!("Expected a non-empty body!")
        }

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::Plain.to_string()]);

        //Content-Length should be same as for GET (issue 514)
        let content_length = response.headers().get_one("Content-Length");
        assert_eq!(content_length, Some("Hello, world!".len().to_string().as_str()));

        //Test empty response
        let response = client.head("empty").dispatch();
        assert_eq!(response.status(), Status::NoContent);
        let content_length = response.headers().get_one("Content-Length");
        assert_eq!(content_length, Some("0"));

    }

    #[test]
    fn user_head() {
        let client = Client::new(rocket::ignite().mount("/", routes())).unwrap();
        let response = client.head("/other").dispatch();

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(content_type, vec![ContentType::JSON.to_string()]);

        let content_length = response.headers().get_one("Content-Length");
        assert_eq!(content_length, Some("{\"h\": 1}".len().to_string().as_str()));

    }
}
