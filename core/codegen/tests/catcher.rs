// Rocket sometimes generates mangled identifiers that activate the
// non_snake_case lint. We deny the lint in this test to ensure that
// code generation uses #[allow(non_snake_case)] in the appropriate places.
#![deny(non_snake_case)]

#[macro_use] extern crate rocket;

use rocket::{Request, Rocket, Build};
use rocket::local::blocking::Client;
use rocket::http::Status;

#[catch(404)] fn not_found_0() -> &'static str { "404-0" }
#[catch(404)] fn not_found_1(_: &Request) -> &'static str { "404-1" }
#[catch(404)] fn not_found_2(_: Status, _: &Request) -> &'static str { "404-2" }
#[catch(default)] fn all(_: Status, r: &Request) -> String { r.uri().to_string() }

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
#[catch(400)] fn forward_400(status: Status, _: &Request) -> String { status.code.to_string() }
#[catch(404)] fn forward_404(status: Status, _: &Request) -> String { status.code.to_string() }
#[catch(444)] fn forward_444(status: Status, _: &Request) -> String { status.code.to_string() }
#[catch(500)] fn forward_500(status: Status, _: &Request) -> String { status.code.to_string() }

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
