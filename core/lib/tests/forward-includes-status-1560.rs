#[macro_use] extern crate rocket;

use rocket::http::Status;
use rocket::request::{self, Request, FromRequest};

struct Authenticated;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        if request.headers().contains("Authenticated") {
            request::Outcome::Success(Authenticated)
        } else {
            request::Outcome::Forward(Status::Unauthorized)
        }
    }
}

struct TeapotForward;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TeapotForward {
    type Error = std::convert::Infallible;

    async fn from_request(_: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Forward(Status::ImATeapot)
    }
}

#[get("/auth")]
fn auth(_name: Authenticated) -> &'static str {
    "Protected"
}

#[get("/auth", rank = 2)]
fn public() -> &'static str {
    "Public"
}

#[get("/auth", rank = 3)]
fn teapot(_teapot: TeapotForward) -> &'static str {
    "Protected"
}

#[get("/need-auth")]
fn auth_needed(_auth: Authenticated) -> &'static str {
    "Have Auth"
}

#[catch(401)]
fn catcher() -> &'static str {
    "Custom Catcher"
}

mod tests {
    use super::*;
    use rocket::routes;
    use rocket::local::blocking::Client;
    use rocket::http::{Header, Status};

    #[test]
    fn authorized_forwards() {
        let client = Client::debug_with(routes![auth, public, auth_needed]).unwrap();

        let response = client.get("/auth")
            .header(Header::new("Authenticated", "true"))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "Protected");

        let response = client.get("/auth").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "Public");

        let response = client.get("/need-auth")
            .header(Header::new("Authenticated", "true"))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "Have Auth");

        let response = client.get("/need-auth").dispatch();
        assert_eq!(response.status(), Status::Unauthorized);
        assert!(response.into_string().unwrap().contains("Rocket"));
    }

    #[test]
    fn unauthorized_custom_catcher() {
        let rocket = rocket::build()
            .mount("/", routes![auth_needed])
            .register("/", catchers![catcher]);

        let client = Client::debug(rocket).unwrap();
        let response = client.get("/need-auth").dispatch();
        assert_eq!(response.status(), Status::Unauthorized);
        assert_eq!(response.into_string().unwrap(), "Custom Catcher");
    }

    #[test]
    fn use_last_forward() {
        let client = Client::debug_with(routes![auth, teapot]).unwrap();
        let response = client.get("/auth").dispatch();
        assert_eq!(response.status(), Status::ImATeapot);
    }
}
