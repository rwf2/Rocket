use rocket;

use rocket::{get, routes};
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket::http::hyper::header::{AcceptLanguage, qitem, Authorization, Bearer};
use language_tags::langtag;

#[get("/lang-required")]
fn lang_required(lang: AcceptLanguage) -> String {
    format!("Accept-Language: {}", lang)
}

#[get("/lang-required", rank = 2)]
fn no_lang() -> Status {
    Status::BadRequest
}

#[test]
fn test_required_header() {
    let rocket = rocket::ignite().mount("/", routes![lang_required, no_lang]);
    let client = Client::tracked(rocket).unwrap();

    // Will hit no_lang() above
    let response = client.get("/lang-required").dispatch();
    assert_eq!(response.status(), Status::BadRequest);

    // Will hit the lang_required() route
    let request = client.get("/lang-required");
    let response = request.header(AcceptLanguage(vec![qitem(langtag!(da))])).dispatch();
    assert_eq!(response.into_string().unwrap(), "Accept-Language: da");
}

#[get("/lang-optional")]
fn lang_optional(lang: Option<AcceptLanguage>) -> String {
    if let Some(lang) = lang {
        format!("Accept-Language: {}", lang)
    } else {
        format!("English is the lingua franca of the internet")
    }
}

#[test]
fn test_optional_header() {
    let rocket = rocket::ignite().mount("/", routes![lang_optional]);
    let client = Client::tracked(rocket).unwrap();

    // When header is present
    let request = client.get("/lang-optional");
    let response = request.header(AcceptLanguage(vec![qitem(langtag!(da))])).dispatch();
    assert_eq!(response.into_string().unwrap(), "Accept-Language: da");

    // When header is elided
    let response = client.get("/lang-optional").dispatch();
    assert_eq!(response.into_string().unwrap(), "English is the lingua franca of the internet");
}

#[get("/spill-beans")]
fn spill_beans(auth: Authorization<Bearer>) -> String {
    format!("I'll tell you secrets: {:?}", auth.0.token)
}

#[test]
fn test_generic_header() {
    let rocket = rocket::ignite().mount("/", routes![spill_beans]);
    let client = Client::tracked(rocket).unwrap();

    let request = client.get("/spill-beans");
    let response = request.header(Authorization(Bearer {
                    token: "aaa".to_owned()
    })).dispatch();
    assert_eq!(response.into_string().unwrap(), "I'll tell you secrets: \"aaa\"");
}

