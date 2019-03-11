#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::response::Redirect;
use rocket::http::uri::Uri;

const NAME: &str = "John[]|\\%@^";

#[get("/hello/<name>")]
fn hello(name: String) -> String {
    format!("Hello, {}!", name)
}

#[get("/raw")]
fn raw_redirect() -> Redirect {
    Redirect::to(format!("/hello/{}", Uri::percent_encode(NAME)))
}

#[get("/uri")]
fn uri_redirect() -> Redirect {
    Redirect::to(uri!(hello: NAME))
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![hello, uri_redirect, raw_redirect])
}


mod tests {
    use super::*;
    use rocket::local::Client;
    use rocket::http::{Status, uri::Uri};

    #[test]
    fn uri_percent_encoding_redirect() {
        let expected_location = vec!["/hello/John%5B%5D%7C%5C%25@%5E"];
        let client = Client::new(rocket()).unwrap();

        let response = client.get("/raw").dispatch();
        let location: Vec<_> = response.headers().get("location").collect();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(&location, &expected_location);

        let response = client.get("/uri").dispatch();
        let location: Vec<_> = response.headers().get("location").collect();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(&location, &expected_location);
    }

    #[test]
    fn uri_percent_encoding_get() {
        let client = Client::new(rocket()).unwrap();
        let name = Uri::percent_encode(NAME);
        let mut response = client.get(format!("/hello/{}", name)).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string().unwrap(), format!("Hello, {}!", NAME));
    }
}
