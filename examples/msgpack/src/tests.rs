extern crate rmp_serde;

use rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::{Status, ContentType};
use rocket::Response;

#[derive(Serialize, Deserialize)]
struct Message {
    id: usize,
    contents: String
}

macro_rules! run_test {
    ($rocket: expr, $req:expr, $test_fn:expr) => ({
        let mut req = $req;
        $test_fn(req.dispatch_with($rocket));
    })
}

#[test]
fn msgpack_get() {
    let rocket = rocket();
    let req = MockRequest::new(Get, "/message/1").header(ContentType::MsgPack);
    run_test!(&rocket, req, |mut response: Response| {
        assert_eq!(response.status(), Status::Ok);
        let body = rmp_serde::from_slice::<Message>(
            &response.body().unwrap().into_bytes().unwrap()
        ).unwrap();
        assert_eq!(body.id, 1);
        assert_eq!(body.contents, "Hello, world!");
    });
}

#[test]
fn msgpack_post() {
    let rocket = rocket();
    let req = MockRequest::new(Post, "/message")
        .header(ContentType::MsgPack)
        .body(rmp_serde::to_vec(&Message {
            id: 2,
            contents: "Goodbye, world!".to_string(),
        }).unwrap());
    run_test!(&rocket, req, |response: Response| {
        assert_eq!(response.status(), Status::Ok);
    });
}
