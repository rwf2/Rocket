// Rocket sometimes generates mangled identifiers that activate the
// non_snake_case lint. We deny the lint in this test to ensure that
// code generation uses #[allow(non_snake_case)] in the appropriate places.
#![deny(non_snake_case)]

#[macro_use] extern crate rocket;

use rocket::{Rocket, Build};
use rocket::local::blocking::Client;
use rocket::http::{Status, uri::Origin};

#[catch(404)]
fn not_found_0() -> &'static str { "404-0" }
#[catch(404)]
fn not_found_1() -> &'static str { "404-1" }
#[catch(404, status = "<_s>")]
fn not_found_2(_s: Status) -> &'static str { "404-2" }
#[catch(default, status = "<_s>")]
fn all(_s: Status, uri: &Origin<'_>) -> String { uri.to_string() }

#[test]
fn test_simple_catchers() {
    fn rocket() -> Rocket<Build> {
        rocket::build()
            .register("/0", catchers![not_found_0])
            .register("/1", catchers![not_found_1])
            .register("/2", catchers![not_found_2])
            .register("/", catchers![all])
    }

    let client = Client::debug(rocket()).unwrap();
    for i in 0..6 {
        let response = client.get(format!("/{}", i)).dispatch();
        assert_eq!(response.status(), Status::NotFound);

        match i {
            0..=2 => assert_eq!(response.into_string().unwrap(), format!("404-{}", i)),
            _ => assert_eq!(response.into_string().unwrap(), format!("/{}", i)),
        }
    }
}

#[get("/<code>")] fn forward(code: u16) -> Status { Status::new(code) }
#[catch(400, status = "<status>")]
fn forward_400(status: Status) -> String { status.code.to_string() }
#[catch(404, status = "<status>")]
fn forward_404(status: Status) -> String { status.code.to_string() }
#[catch(444, status = "<status>")]
fn forward_444(status: Status) -> String { status.code.to_string() }
#[catch(500, status = "<status>")]
fn forward_500(status: Status) -> String { status.code.to_string() }

#[test]
fn test_status_param() {
    fn rocket() -> Rocket<Build> {
        rocket::build()
            .mount("/", routes![forward])
            .register("/", catchers![forward_400, forward_404, forward_444, forward_500])
    }

    let client = Client::debug(rocket()).unwrap();
    for code in &[400, 404, 444, 400, 800, 3480] {
        let response = client.get(uri!(forward(*code))).dispatch();
        let code = std::cmp::min(*code, 500);
        assert_eq!(response.status(), Status::new(code));
        assert_eq!(response.into_string().unwrap(), code.to_string());
    }
}

#[catch(404)]
fn bad_req_untyped() -> &'static str { "404" }
#[catch(404, error = "<_e>")]
fn bad_req_string(_e: &String) -> &'static str { "404 String" }
#[catch(404, error = "<_e>")]
fn bad_req_tuple(_e: &()) -> &'static str { "404 ()" }

#[test]
fn test_typed_catchers() {
    fn rocket() -> Rocket<Build> {
        rocket::build()
            .register("/", catchers![bad_req_untyped, bad_req_string, bad_req_tuple])
    }

    // Assert the catchers do not collide. They are only differentiated by their error type.
    let client = Client::debug(rocket()).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.status(), Status::NotFound);
}
