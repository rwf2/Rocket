use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;

fn test(uri: &str, expected: &str) {
    let rocket = rocket::ignite().mount("/", routes![super::show]);

    let mut req = MockRequest::new(Get, uri);
    let mut res = req.dispatch_with(&rocket);

    assert_eq!(res.body().and_then(|b| b.into_string()), Some(expected.into()));
}

fn test_not_found(uri: &str) {
    let rocket = rocket::ignite().mount("/", routes![super::show]);

    let mut req = MockRequest::new(Get, uri);
    let mut res = req.dispatch_with(&rocket);

    assert_eq!(res.body().and_then(|b| b.into_string()), Some("Person not found".into()));
}


#[test]
fn test_show() {
    test("/7f205202-7ba1-4c39-b2fc-3e630722bf9f", "We found: Lacy");
    test("/4da34121-bc7d-4fc1-aee6-bf8de0795333", "We found: Bob");
    test("/ad962969-4e3d-4de7-ac4a-2d86d6d10839", "We found: George");
    test_not_found("/e18b3a5c-488f-4159-a240-2101e0da19fd");
}
