use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::{Method, Status};

#[test]
fn index() {
    let rocket = rocket::ignite().mount("/", routes![super::index]);
    let mut req = MockRequest::new(Method::Get, "/");
    let mut response = req.dispatch_with(&rocket);

    assert_eq!(response.status(), Status::Ok);

    let body_str = response.body().and_then(|body| body.into_string());
    assert_eq!(body_str, Some(include_str!("usage.txt").to_string()));
}

#[test]
fn upload() {
    let rocket = rocket::ignite().mount("/", routes![super::upload]);
    let mut req = MockRequest::new(Method::Post, "/")
        .body(include_str!("file.txt").to_string());
    let mut response = req.dispatch_with(&rocket);

    assert_eq!(response.status(), Status::Ok);

    match response.body().and_then(|body| body.into_string()) {
        Some(url) => {
            match url.trim().split("/").last() {
                Some(paste_id) => {
                    let filename = format!("upload/{}", paste_id);
                    assert!(Path::new(&filename).exists());
                },
                _ => unreachable!(),
            }
        },
        _ => unreachable!(),
    }
}

#[test]
fn retrieve_file_found() {
    let id = super::PasteID::new(super::ID_LENGTH);
    let filename = format!("upload/{}", id);

    let mut file = File::create(&filename).expect("failed to create file");
    file.write_all(b"Hello from Rocket!").expect("failed to write file");

    let rocket = rocket::ignite().mount("/", routes![super::retrieve]);
    let mut req = MockRequest::new(Method::Get, format!("/{}", id));
    let mut response = req.dispatch_with(&rocket);

    assert_eq!(response.status(), Status::Ok);

    let body_str = response.body().and_then(|body| body.into_string());
    assert_eq!(body_str, Some("Hello from Rocket!".to_string()));
}

#[test]
fn retrieve_file_not_found() {
    let rocket = rocket::ignite().mount("/", routes![super::retrieve]);
    let mut req = MockRequest::new(Method::Get, format!("/{}", "file-not-found"));
    let response = req.dispatch_with(&rocket);

    assert_eq!(response.status(), Status::NotFound);
}
