use std::fs::File;
use std::io::Read;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::Status;

use super::rocket;

fn test_query_file<T> (path: &str, status: Status, content: T)
    where T: Into<Option<String>>
{
    let rocket = rocket();
    let mut req = MockRequest::new(Get, &path);

    let mut response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), status);

    let body_str = response.body().and_then(|body| body.into_string());
    if let Some(expected_str) = content.into() {
        assert!(body_str.map_or(false, |s| s == expected_str));
    }
}

#[test]
fn test_index_html() {
    let file_path = "static/index.html";

    let mut fp = File::open(&file_path)
        .expect(&format!("Can not open {}", file_path));

    let mut expected_content = String::new();
    fp.read_to_string(&mut expected_content)
        .expect(&format!("Reading {} failed.", file_path));

    test_query_file("/", Status::Ok, expected_content);
}

#[test]
fn test_hidden_file() {
    let file_path = "static/hidden/hi.txt";

    let mut fp = File::open(&file_path)
        .expect(&format!("Can not open {}", file_path));

    let mut expected_content = String::new();
    fp.read_to_string(&mut expected_content)
        .expect(&format!("Reading {} failed.", file_path));

    test_query_file("/hidden/hi.txt", Status::Ok, expected_content);
}

#[test]
fn test_invalid_path() {
    test_query_file("/thou_shalt_not_exist", Status::NotFound, None);
    test_query_file("/thou/shalt/not/exist", Status::NotFound, None);
}
