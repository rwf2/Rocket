#[macro_use]
extern crate rocket;

use std::time::{Duration, SystemTime};
use rocket::http::Expires;

#[derive(Responder)]
struct MyResponse {
    body: String,
    expires: Expires,
}

#[get("/")]
fn index() -> MyResponse {
    let some_future_time =
        SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(60 * 60 * 24 * 365 * 100)).unwrap();

    MyResponse {
        body: "Hello, world!".into(),
        expires: Expires::from(some_future_time)
    }
}

#[test]
fn typed_header() {
    let rocket = rocket::build().mount("/", routes![index]);
    let client = rocket::local::blocking::Client::debug(rocket).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.headers().get_one("Expires").unwrap(), "Sat, 07 Dec 2069 00:00:00 GMT");
}
