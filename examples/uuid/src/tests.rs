use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::Status;

fn test(uri: &str, expected: &str) {
    let rocket = rocket::ignite().mount("/", super::routes());

    let mut req = MockRequest::new(Get, uri);
    let mut res = req.dispatch_with(&rocket);

    assert_eq!(res.body().and_then(|b| b.into_string()), Some(expected.into()));
}

fn test_404(uri: &str) {
    let rocket = rocket::ignite().mount("/", super::routes());

    let mut req = MockRequest::new(Get, uri);
    let res = req.dispatch_with(&rocket);

    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_people() {
    test("/people/7f205202-7ba1-4c39-b2fc-3e630722bf9f", "We found: Lacy");
    test("/people/4da34121-bc7d-4fc1-aee6-bf8de0795333", "We found: Bob");
    test("/people/ad962969-4e3d-4de7-ac4a-2d86d6d10839", "We found: George");

    test("/people/e18b3a5c-488f-4159-a240-2101e0da19fd", "Person not found for UUID: e18b3a5c-488f-4159-a240-2101e0da19fd");

    test_404("/people/invalid_uuid");
}

#[test]
fn test_people_opt() {
    test("/people_opt/7f205202-7ba1-4c39-b2fc-3e630722bf9f", "We found: Lacy");
    test("/people_opt/4da34121-bc7d-4fc1-aee6-bf8de0795333", "We found: Bob");
    test("/people_opt/ad962969-4e3d-4de7-ac4a-2d86d6d10839", "We found: George");

    test("/people_opt/e18b3a5c-488f-4159-a240-2101e0da19fd", "Person not found for UUID: e18b3a5c-488f-4159-a240-2101e0da19fd");

    test("/people_opt/invalid_uuid", "Invalid length; expecting 32 or 36 chars, found 12");
}
