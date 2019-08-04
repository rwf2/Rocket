use super::rocket;
use rocket::local::Client;
use rocket::http::Status;

fn test(uri: &str, expected: &str) {
    let client = Client::new(rocket()).unwrap();
    let mut res = client.get(uri).dispatch();
    assert_eq!(res.body_string_wait(), Some(expected.into()));
}

fn test_404(uri: &str) {
    let client = Client::new(rocket()).unwrap();
    let res = client.get(uri).dispatch();
    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn test_people() {
    test("/people/7f205202-7ba1-4c39-b2fc-3e630722bf9f", "We found: Lacy");
    test("/people/4da34121-bc7d-4fc1-aee6-bf8de0795333", "We found: Bob");
    test("/people/ad962969-4e3d-4de7-ac4a-2d86d6d10839", "We found: George");
    test("/people/e18b3a5c-488f-4159-a240-2101e0da19fd",
         "Person not found for UUID: e18b3a5c-488f-4159-a240-2101e0da19fd");
    test_404("/people/invalid_uuid");
}
