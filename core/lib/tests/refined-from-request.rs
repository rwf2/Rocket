#[macro_use]
extern crate rocket;

use std::num::ParseIntError;

use rocket::{outcome::IntoOutcome, request::{FromRequest, Outcome}, Request};
use rocket_http::{Header, Status};

pub struct SessionId {
    session_id: u64,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionId {
    type Error = ParseIntError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ParseIntError> {
        let session_id_string = request.headers().get("Session-Id").next()
            .or_forward(Status::BadRequest);
        session_id_string.and_then(|v| v.parse()
            .map(|id| SessionId { session_id: id })
            .or_error(Status::BadRequest))
    }
}

#[get("/mandatory")]
fn get_data_with_mandatory_header(header: SessionId) -> String {
    format!("GET for session {:}", header.session_id)
}

#[get("/optional")]
fn get_data_with_opt_header(opt_header: Option<SessionId>) -> String {
    if let Some(id) = opt_header {
        format!("GET for session {:}", id.session_id)
    } else {
        format!("GET for new session")
    }
}

#[test]
fn read_optional_header() {
    let rocket = rocket::build().mount(
        "/",
        routes![get_data_with_opt_header, get_data_with_mandatory_header]);
    let client = rocket::local::blocking::Client::debug(rocket).unwrap();

    // If we supply the header, the handler sees it
    let response = client.get("/optional")
        .header(Header::new("session-id", "1234567")).dispatch();
    assert_eq!(response.into_string().unwrap(), "GET for session 1234567".to_string());

    // If no header, means that the handler sees a None
    let response = client.get("/optional").dispatch();
    assert_eq!(response.into_string().unwrap(), "GET for new session".to_string());

    // If we supply a malformed header, the handler will not be called, but the request will fail
    let response = client.get("/optional")
        .header(Header::new("session-id", "Xw23")).dispatch();
    assert_eq!(response.status(), Status::BadRequest);
}

#[test]
fn read_mandatory_header() {
        let rocket = rocket::build().mount(
            "/",
            routes![get_data_with_opt_header, get_data_with_mandatory_header]);
        let client = rocket::local::blocking::Client::debug(rocket).unwrap();
    
    // If the header is missing, it's a bad request (extra info would be nice, though)
    let response = client.get("/mandatory").dispatch();
    assert_eq!(response.status(), Status::BadRequest);

    // If the header is malformed, it's a bad request too (extra info would be nice, though)
    let response = client.get("/mandatory")
        .header(Header::new("session-id", "Xw23")).dispatch();
    assert_eq!(response.status(), Status::BadRequest);

    // If the header is fine, just do the stuff
    let response = client.get("/mandatory")
        .header(Header::new("session-id", "64535")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), "GET for session 64535".to_string());
}
