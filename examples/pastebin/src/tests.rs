use super::{rocket, index};
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::{Status, ContentType};
use rocket::Rocket;

fn extract_id(from: &str) -> Option<String> {
    from.rfind('/').map(|i| &from[(i + 1)..]).map(|s| s.trim_right().to_string())
}

#[test]
fn check_index() {
    let rocket = rocket();

    // Ensure the index returns what we expect.
    let mut req = MockRequest::new(Get, "/");
    let mut response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.body_string(), Some(index().into()))
}

fn upload_paste(rocket: &Rocket, body: &str) -> String {
    let mut req = MockRequest::new(Post, "/").body(body);
    let mut response = req.dispatch_with(rocket);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    extract_id(&response.body_string().unwrap()).unwrap()
}


fn download_paste(rocket: &Rocket, id: &str) -> String {
    let mut req = MockRequest::new(Get, format!("/{}", id));
    let mut response = req.dispatch_with(rocket);
    assert_eq!(response.status(), Status::Ok);
    response.body_string().unwrap()
}

#[test]
fn pasting() {
    let rocket = rocket();

    // Do a trivial upload, just to make sure it works.
    let body_1 = "Hello, world!";
    let id_1 = upload_paste(&rocket, body_1);
    assert_eq!(download_paste(&rocket, &id_1), body_1);

    // Make sure we can keep getting that paste.
    assert_eq!(download_paste(&rocket, &id_1), body_1);
    assert_eq!(download_paste(&rocket, &id_1), body_1);
    assert_eq!(download_paste(&rocket, &id_1), body_1);

    // Upload some unicode.
    let body_2 = "こんにちは";
    let id_2 = upload_paste(&rocket, body_2);
    assert_eq!(download_paste(&rocket, &id_2), body_2);

    // Make sure we can get both pastes.
    assert_eq!(download_paste(&rocket, &id_1), body_1);
    assert_eq!(download_paste(&rocket, &id_2), body_2);
    assert_eq!(download_paste(&rocket, &id_1), body_1);
    assert_eq!(download_paste(&rocket, &id_2), body_2);

    // Now a longer upload.
    let body_3 = "Lorem ipsum dolor sit amet, consectetur adipisicing elit, sed
        do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim
        ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut
        aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit
        in voluptate velit esse cillum dolore eu fugiat nulla pariatur.
        Excepteur sint occaecat cupidatat non proident, sunt in culpa qui
        officia deserunt mollit anim id est laborum.";
    let id_3 = upload_paste(&rocket, body_3);
    assert_eq!(download_paste(&rocket, &id_3), body_3);
    assert_eq!(download_paste(&rocket, &id_1), body_1);
    assert_eq!(download_paste(&rocket, &id_2), body_2);
}
