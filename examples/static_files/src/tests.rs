use std::fs::File;
use std::io::Read;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::Status;
use rocket::response::Body;

use super::rocket;

fn body_to_bytes(body: Body<&mut Read>) -> Option<Vec<u8>> {
    match body {
        Body::Sized(b, _)  => {
            let mut data: Vec<u8> = vec![];
            b.read(&mut data).expect("Read from response failed");
            return Some(data);
        },
        Body::Chunked(b, _) => {
            let mut data: Vec<u8> = vec![];
            b.read(&mut data).expect("Read from response failed");
            return Some(data);
        }
    }
}

fn test_query_file<T> (path: &str, status: Status, content: T)
    where T: Into<Option<Vec<u8>>>
{
    let rocket = rocket();
    let mut req = MockRequest::new(Get, &path);

    let mut response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), status);

    let body_data = response.body().and_then(|body| body_to_bytes(body));
    if let Some(expected_data) = content.into() {
        assert!(body_data.map_or(false, |s| s == expected_data));
    }
}

fn read_file_content(path: &str) -> Vec<u8> {
    let mut fp = File::open(&path).expect(&format!("Can not open {}", path));

    let mut file_content:Vec<u8> = vec![];
    fp.read(&mut file_content).expect(&format!("Reading {} failed.", path));

    file_content
}

#[test]
fn test_index_html() {
    let expected_content = read_file_content("static/index.html");
    test_query_file("/", Status::Ok, expected_content);
}

#[test]
fn test_hidden_file() {
    let expected_content = read_file_content("static/hidden/hi.txt");
    test_query_file("/hidden/hi.txt", Status::Ok, expected_content);
}

#[test]
fn test_icon_file() {
    let expected_content = read_file_content("static/rocket-icon.jpg");
    test_query_file("/rocket-icon.jpg", Status::Ok, expected_content);
}

#[test]
fn test_invalid_path() {
    test_query_file("/thou_shalt_not_exist", Status::NotFound, None);
    test_query_file("/thou/shalt/not/exist", Status::NotFound, None);
}
