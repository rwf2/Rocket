#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::http::uri::Segments;

#[get("/test/<path..>")]
fn test(path: Segments) -> String {
    path.collect::<Vec<_>>().join("/")
}

#[get("/two/<path..>")]
fn two(path: Segments) -> String {
    path.collect::<Vec<_>>().join("/")
}

#[get("/one/two/<path..>")]
fn one_two(path: Segments) -> String {
    path.collect::<Vec<_>>().join("/")
}

#[get("/<path..>", rank = 2)]
fn none(path: Segments) -> String {
    path.collect::<Vec<_>>().join("/")
}

use rocket::testing::MockRequest;
use rocket::http::Method::*;

#[test]
fn segments_works() {
    let rocket = rocket::ignite().mount("/", routes![test, two, one_two, none]);

    // We construct a path that matches each of the routes above. We ensure the
    // prefix is stripped, confirming that dynamic segments are working.
    for prefix in &["", "/test", "/two", "/one/two"] {
        let path = "this/is/the/path/we/want";
        let mut req = MockRequest::new(Get, format!("{}/{}", prefix, path));

        let mut response = req.dispatch_with(&rocket);
        let body_str = response.body().and_then(|b| b.into_string());
        assert_eq!(body_str, Some(path.into()));
    }
}
