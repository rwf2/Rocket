extern crate rmp_serde;

use rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::{Status, ContentType};
use rocket::Response;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Message {
    #[allow(dead_code)]
    id: Option<usize>,
    contents: String
}

macro_rules! run_test {
    ($rocket: expr, $req:expr, $test_fn:expr) => ({
        let mut req = $req;
        $test_fn(req.dispatch_with($rocket));
    })
}

fn build_hello_body() -> Vec<u8> {
    rmp_serde::to_vec(&[("contents", "Hello, world!")]
        .iter()
        .cloned()
        .collect::<HashMap<&'static str, &'static str>>()).unwrap()
}

fn build_goodbye_body() -> Vec<u8> {
    rmp_serde::to_vec(&[("contents", "Bye bye, world!")]
        .iter()
        .cloned()
        .collect::<HashMap<&'static str, &'static str>>()).unwrap()
}

#[test]
fn bad_get_put() {
    let rocket = rocket();

    // Try to get a message with an ID that doesn't exist.
    let req = MockRequest::new(Get, "/message/99").header(ContentType::MsgPack);
    run_test!(&rocket, req, |mut response: Response| {
        assert_eq!(response.status(), Status::NotFound);

        let body = rmp_serde::from_slice::<HashMap<String, String>>(
            &response.body().unwrap().into_bytes().unwrap()
        ).unwrap();
        assert!(body.values().any(|v| v == "error"));
        assert!(body.values().any(|v| v == "Resource was not found."));
    });

    // Try to get a message with an invalid ID.
    let req = MockRequest::new(Get, "/message/hi").header(ContentType::MsgPack);
    run_test!(&rocket, req, |mut response: Response| {
        assert_eq!(response.status(), Status::NotFound);
        let body = rmp_serde::from_slice::<HashMap<String, String>>(
            &response.body().unwrap().into_bytes().unwrap()
        ).unwrap();
        assert!(body.values().any(|v| v == "error"));
    });

    // Try to put a message without a proper body.
    let req = MockRequest::new(Put, "/message/80").header(ContentType::MsgPack);
    run_test!(&rocket, req, |response: Response| {
        assert_eq!(response.status(), Status::BadRequest);
    });

    // Try to put a message for an ID that doesn't exist.
    let req = MockRequest::new(Put, "/message/80")
        .header(ContentType::MsgPack)
        .body(build_goodbye_body());

    run_test!(&rocket, req, |response: Response| {
        assert_eq!(response.status(), Status::NotFound);
    });
}

#[test]
fn post_get_put_get() {
    let rocket = rocket();
    // Check that a message with ID 1 doesn't exist.
    let req = MockRequest::new(Get, "/message/1").header(ContentType::MsgPack);
    run_test!(&rocket, req, |response: Response| {
        assert_eq!(response.status(), Status::NotFound);
    });

    // Add a new message with ID 1.
    let req = MockRequest::new(Post, "/message/1")
        .header(ContentType::MsgPack)
        .body(build_hello_body());

    run_test!(&rocket, req, |response: Response| {
        assert_eq!(response.status(), Status::Ok);
    });

    // Check that the message exists with the correct contents.
    let req = MockRequest::new(Get, "/message/1").header(ContentType::MsgPack);
    run_test!(&rocket, req, |mut response: Response| {
        assert_eq!(response.status(), Status::Ok);
        let body = rmp_serde::from_slice::<Message>(
            &response.body().unwrap().into_bytes().unwrap()
        ).unwrap();
        assert!(body.contents == "Hello, world!");
    });

    // Change the message contents.
    let req = MockRequest::new(Put, "/message/1")
        .header(ContentType::MsgPack)
        .body(build_goodbye_body());

    run_test!(&rocket, req, |response: Response| {
        assert_eq!(response.status(), Status::Ok);
    });

    // Check that the message exists with the updated contents.
    let req = MockRequest::new(Get, "/message/1").header(ContentType::MsgPack);
    run_test!(&rocket, req, |mut response: Response| {
        assert_eq!(response.status(), Status::Ok);
        let body = rmp_serde::from_slice::<Message>(
            &response.body().unwrap().into_bytes().unwrap()
        ).unwrap();
        assert!(body.contents != "Hello, world!");
        assert!(body.contents == "Bye bye, world!");
    });
}
