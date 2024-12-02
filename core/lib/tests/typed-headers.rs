#[macro_use]
extern crate rocket;

use std::time::{Duration, SystemTime};
use headers::IfModifiedSince;
use rocket::http::Expires;
use rocket_http::{Header, Status};

#[derive(Responder)]
struct MyResponse {
    body: String,
    expires: Expires,
}

#[get("/expires")]
fn index() -> MyResponse {
    let some_future_time =
        SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(60 * 60 * 24 * 365 * 100)).unwrap();

    MyResponse {
        body: "Hello, world!".into(),
        expires: Expires::from(some_future_time)
    }
}

#[get("/data")]
fn get_data_with_opt_header(since: Option<IfModifiedSince>) -> String {
    if let Some(time) = since {
        format!("GET after: {:}", time::OffsetDateTime::from(SystemTime::from(time)))
    } else {
        format!("Unconditional GET")
    }
}

#[get("/data_since")]
fn get_data_with_header(since: IfModifiedSince) -> String {
    format!("GET after: {:}", time::OffsetDateTime::from(SystemTime::from(since)))
}

#[test]
fn respond_with_typed_header() {
    let rocket = rocket::build().mount(
        "/",
        routes![index, get_data_with_opt_header, get_data_with_header]);
    let client = rocket::local::blocking::Client::debug(rocket).unwrap();

    let response = client.get("/expires").dispatch();
    assert_eq!(response.headers().get_one("Expires").unwrap(), "Sat, 07 Dec 2069 00:00:00 GMT");
}

#[test]
fn read_typed_header() {
    let rocket = rocket::build().mount(
        "/",
        routes![index, get_data_with_opt_header, get_data_with_header]);
    let client = rocket::local::blocking::Client::debug(rocket).unwrap();

    let response = client.get("/data").dispatch();
    assert_eq!(response.into_string().unwrap(), "Unconditional GET".to_string());

    let response = client.get("/data")
        .header(Header::new("if-modified-since", "Mon, 07 Dec 2020 00:00:00 GMT")).dispatch();
    assert_eq!(response.into_string().unwrap(),
        "GET after: 2020-12-07 0:00:00.0 +00:00:00".to_string());

    let response = client.get("/data_since")
        .header(Header::new("if-modified-since", "Tue, 08 Dec 2020 00:00:00 GMT")).dispatch();
    assert_eq!(response.into_string().unwrap(),
        "GET after: 2020-12-08 0:00:00.0 +00:00:00".to_string());

    let response = client.get("/data_since")
        .header(Header::new("if-modified-since", "WTF, 07 Dec 2020 00:00:00 GMT")).dispatch();
    assert_eq!(response.status(), Status::BadRequest);

    let response = client.get("/data_since")
        .header(Header::new("if-modified-since", "\x0c  , 07 Dec 2020 00:00:00 GMT")).dispatch();
    assert_eq!(response.status(), Status::BadRequest);
}
