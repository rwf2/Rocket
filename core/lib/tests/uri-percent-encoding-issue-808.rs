#[macro_use] extern crate rocket;

use rocket::{Rocket, Build};
use rocket::response::Redirect;

const NAME: &str = "John[]|\\%@^";

#[get("/hello/<name>")]
fn hello(name: String) -> String {
    format!("Hello, {}!", name)
}

#[get("/raw")]
fn raw_redirect() -> Redirect {
    Redirect::to(uri!(hello(NAME)))
}

#[get("/uri")]
fn uri_redirect() -> Redirect {
    Redirect::to(uri!(hello(NAME)))
}

fn rocket() -> Rocket<Build> {
    rocket::build().mount("/", routes![hello, uri_redirect, raw_redirect])
}

mod tests {
    use super::*;
    use rocket::local::blocking::Client;
    use rocket::http::Status;

    #[test]
    fn uri_percent_encoding_redirect() {
        let expected_location = vec!["/hello/John[]%7C%5C%25@%5E"];
        let client = Client::debug(rocket()).unwrap();

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
        let client = Client::debug(rocket()).unwrap();
        let response = client.get(uri!(hello(NAME))).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), format!("Hello, {}!", NAME));
    }
}
