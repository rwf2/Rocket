#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::request::Form;
use rocket::http::Status;

#[derive(FromForm)]
struct FormData {
    form_data: String,
}

#[patch("/", data = "<form_data>")]
fn bug(form_data: Form<FormData>) -> &'static str {
    assert_eq!("Form data", &form_data.get().form_data);
    "OK"
}

use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::ContentType;

#[test]
fn method_eval() {
    let rocket = rocket::ignite().mount("/", routes![bug]);

    let mut req = MockRequest::new(Post, "/")
        .header(ContentType::Form)
        .body("_method=patch&form_data=Form+data");

    let mut response = req.dispatch_with(&rocket);
    let body_str = response.body().and_then(|b| b.into_string());
    assert_eq!(body_str, Some("OK".to_string()));
}

#[test]
fn get_passes_through() {
    let rocket = rocket::ignite().mount("/", routes![bug]);

    let mut req = MockRequest::new(Get, "/")
        .header(ContentType::Form)
        .body("_method=patch&form_data=Form+data");

    let mut response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), Status::NotFound);
}
