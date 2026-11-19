#[macro_use] extern crate rocket;

use std::path::{Path, PathBuf};
use rocket::http::{Status, ext::Normalize};
use rocket::request::{self, Request, FromRequest};
use rocket::Route;

#[get("/<path..>")]
fn files(route: &Route, path: PathBuf) -> String {
    Path::new(&route.uri.base()).join(path).normalized_str().to_string()
}

struct Forward;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Forward {
    type Error = std::convert::Infallible;

    async fn from_request(_: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Forward(Status::NotFound)
    }
}

#[get("/")]
fn first(_forward: Forward) { }

#[get("/", rank = 2)]
fn second(route: &Route) -> String {
    route.name.as_deref().unwrap().to_string()
}

mod route_guard_tests {
    use super::*;
    use rocket::local::blocking::Client;

    fn assert_path(client: &Client, path: &str) {
        let res = client.get(path).dispatch();
        assert_eq!(res.into_string(), Some(path.into()));
    }

    #[test]
    fn check_mount_path() {
        let rocket = rocket::build()
            .mount("/first", routes![files])
            .mount("/second", routes![files]);

        let client = Client::debug(rocket).unwrap();
        assert_path(&client, "/first/some/path");
        assert_path(&client, "/second/some/path");
        assert_path(&client, "/first/second/b/c");
        assert_path(&client, "/second/a/b/c");
    }

    #[test]
    fn check_route_after_forward() {
        let client = Client::debug_with(routes![first, second]).unwrap();
        let response = client.get("/").dispatch();
        assert_eq!(response.into_string().unwrap(), "second");
    }
}
