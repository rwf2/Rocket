#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate rocket_http;

use rocket::response::Redirect;
use rocket::Response;
use rocket_http::{Header, Status};

#[get("/do_not_overwrite")]
fn do_not_overwrite<'r>() -> Result<Response<'r>, ()> {
    let header = Header::new("Server", "Test");

    Response::build()
        .header(header)
        .ok()
}

#[get("/use_default")]
fn use_default<'r>() -> Result<Response<'r>, ()> {
    Response::build()
        .ok()
}

mod conditionally_set_server_header {
    use super::*;
    use rocket::local::Client;

    #[test]
    fn do_not_overwrite_server_header() {
        let rocket = rocket::ignite().mount("/", routes![do_not_overwrite, use_default]);
        let client = Client::new(rocket).unwrap();

        let response = client.get("/do_not_overwrite").dispatch();
        let server = response.headers().get_one("Server");
        assert_eq!(server, Some("Test"));

        let response = client.get("/use_default").dispatch();
        let server = response.headers().get_one("Server");
        assert_eq!(server, Some("Rocket"));
    }
}
