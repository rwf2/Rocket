#![feature(plugin)]
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
fn other() -> content::JSON<()> {
    content::JSON(())
}

mod tests {
    use super::*;

    use rocket::Route;
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;
    use rocket::http::{Status, ContentType};
    use rocket::response::Body;

    fn routes() -> Vec<Route> {
        routes![index, empty, other]
    }

    #[test]
    fn auto_head() {
        let rocket = rocket::ignite().mount("/", routes());

        let mut req = MockRequest::new(Head, "/");
        let mut response = req.dispatch_with(&rocket);

        assert_eq!(response.status(), Status::Ok);
        if let Some(body) = response.body() {
            match body {
                Body::Sized(_, n) => assert_eq!(n, "Hello, world!".len() as u64),
                _ => panic!("Expected a sized body!")
            }

            assert_eq!(body.into_string(), Some("".to_string()));
        } else {
            panic!("Expected an empty body!")
        }


        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::Plain.to_string()]);

        let mut req = MockRequest::new(Head, "/empty");
        let response = req.dispatch_with(&rocket);
        assert_eq!(response.status(), Status::NoContent);
    }

    #[test]
    fn user_head() {
        let rocket = rocket::ignite().mount("/", routes());
        let mut req = MockRequest::new(Head, "/other");
        let response = req.dispatch_with(&rocket);

        assert_eq!(response.status(), Status::Ok);

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::JSON.to_string()]);
    }
}
