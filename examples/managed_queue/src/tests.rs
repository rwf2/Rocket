use super::rocket;

use rocket::testing::MockRequest;
use rocket::http::Status;
use rocket::http::Method::*;

#[test]
fn test_push_pop() {
    let rocket = rocket();

    let mut req = MockRequest::new(Put, "/push?description=test1");
    let response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), Status::Ok);

    let mut req = MockRequest::new(Get, "/pop");
    let mut response = req.dispatch_with(&rocket);
    assert_eq!(response.body_string(), Some("test1".to_string()));
}
