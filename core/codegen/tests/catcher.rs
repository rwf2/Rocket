// Rocket sometimes generates mangled identifiers that activate the
// non_snake_case lint. We deny the lint in this test to ensure that
// code generation uses #[allow(non_snake_case)] in the appropriate places.
#![deny(non_snake_case)]

#[macro_use] extern crate rocket;

use std::io;
use std::num::ParseIntError;
use std::str::ParseBoolError;

use rocket::request::FromParamError;
use rocket::{Request, Rocket, Build};
use rocket::local::blocking::Client;
use rocket::http::Status;
use rocket_http::uri::Origin;

#[catch(404)] fn not_found_0() -> &'static str { "404-0" }
#[catch(404)] fn not_found_1() -> &'static str { "404-1" }
#[catch(404)] fn not_found_2() -> &'static str { "404-2" }
#[catch(default)] fn all(r: &Request<'_>) -> String { r.uri().to_string() }

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
#[catch(400)] fn forward_400(status: Status) -> String { status.code.to_string() }
#[catch(404)] fn forward_404(status: Status) -> String { status.code.to_string() }
#[catch(444)] fn forward_444(status: Status) -> String { status.code.to_string() }
#[catch(500)] fn forward_500(status: Status) -> String { status.code.to_string() }

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


#[catch(400)] fn test_status(status: Status) -> String { format!("{}", status.code) }
#[catch(404)] fn test_request(r: &Request<'_>) -> String { format!("{}", r.uri()) }
#[catch(444)] fn test_uri(uri: &Origin<'_>) -> String { format!("{}", uri) }

#[test]
fn test_basic_params() {
    fn rocket() -> Rocket<Build> {
        rocket::build()
            .mount("/", routes![forward])
            .register("/", catchers![
                test_status,
                test_request,
                test_uri,
            ])
    }

    let client = Client::debug(rocket()).unwrap();
    let response = client.get(uri!(forward(400))).dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(response.into_string().unwrap(), "400");

    let response = client.get(uri!(forward(404))).dispatch();
    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.into_string().unwrap(), "/404");

    let response = client.get(uri!(forward(444))).dispatch();
    assert_eq!(response.status(), Status::new(444));
    assert_eq!(response.into_string().unwrap(), "/444");
}

#[get("/c/<code>")] fn read_int(code: u16) -> String { format!("{code}") }
#[get("/b/<code>")] fn read_bool(code: bool) -> String { format!("{code}") }
#[get("/b/force")] fn force_bool_error() -> Result<&'static str, ParseBoolError> {
    "smt".parse::<bool>().map(|_| todo!())
}

#[catch(default, error = "<e>")]
fn test_io_error(e: &io::Error) -> String { format!("{e:?}") }
#[catch(default, error = "<_e>")]
fn test_parse_int_error(_e: &ParseIntError) -> String { format!("ParseIntError") }
#[catch(default, error = "<_e>")]
fn test_parse_bool_error(_e: &ParseBoolError) -> String { format!("ParseBoolError") }
#[catch(default, error = "<e>")]
fn test_param_parse_bool_error(e: &FromParamError<'_, ParseBoolError>) -> String {
    format!("ParseBoolError: {}", e.raw)
}


#[test]
fn test_error_types() {
    fn rocket() -> Rocket<Build> {
        rocket::build()
            .mount("/", routes![read_int, read_bool, force_bool_error])
            .register("/", catchers![
                test_io_error,
                test_parse_int_error,
                test_parse_bool_error,
                test_param_parse_bool_error,
            ])
    }

    let client = Client::debug(rocket()).unwrap();
    let response = client.get(uri!(read_int(400))).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), "400");

    let response = client.get(uri!("/c/40000000")).dispatch();
    assert_eq!(response.status(), Status::UnprocessableEntity);
    assert_eq!(response.into_string().unwrap(), "ParseIntError");

    let response = client.get(uri!(read_bool(true))).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), "true");

    let response = client.get(uri!("/b/smt")).dispatch();
    assert_eq!(response.status(), Status::UnprocessableEntity);
    assert_eq!(response.into_string().unwrap(), "ParseBoolError: smt");
    let response = client.get(uri!("/b/force")).dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(response.into_string().unwrap(), "ParseBoolError");
}
