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
        let body = response.body().unwrap().into_bytes().unwrap();
        // Represents a message of `[1, "Hello, world!"]`
        assert_eq!(&body, &[146, 1, 173, 72, 101, 108, 108, 111, 44, 32, 119, 111,
                            114, 108, 100, 33]);
    });
}

#[test]
fn msgpack_post() {
    let rocket = rocket();
    let req = MockRequest::new(Post, "/message")
        .header(ContentType::MsgPack)
        // Represents a message of `[2, "Goodbye, world!"]`
        .body(&[146, 2, 175, 71, 111, 111, 100, 98, 121, 101, 44, 32, 119, 111, 114, 108, 100, 33]);
    run_test!(&rocket, req, |mut response: Response| {
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body().unwrap().into_string().unwrap(), "Goodbye, world!");
    });
}
