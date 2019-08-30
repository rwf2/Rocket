#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::Response;
use rocket::http::Header;

#[get("/do_not_overwrite")]
fn do_not_overwrite() -> Response<'static> {
    Response::build()
        .header(Header::new("Server", "Test"))
        .finalize()
}

#[get("/use_default")]
fn use_default() { }

mod conditionally_set_server_header {
    use super::*;
    use rocket::local::Client;

    #[rocket::async_test]
    async fn do_not_overwrite_server_header() {
        let rocket = rocket::ignite().mount("/", routes![do_not_overwrite, use_default]);
        let client = Client::new(rocket).unwrap();

        let response = client.get("/do_not_overwrite").dispatch().await;
        let server = response.headers().get_one("Server");
        assert_eq!(server, Some("Test"));

        let response = client.get("/use_default").dispatch().await;
        let server = response.headers().get_one("Server");
        assert_eq!(server, Some("Rocket"));
    }
}
