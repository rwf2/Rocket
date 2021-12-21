#[macro_use] extern crate rocket;

use rocket::http::Status;
use rocket::request::{self, Request, FromRequest};

pub struct Authenticated;

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

#[get("/one")]
pub async fn get_protected_one(_user: Authenticated) -> &'static str {
    "Protected"
}

#[get("/one", rank = 2)]
pub async fn get_public_one() -> &'static str {
    "Public" 
}

#[get("/two")]
pub async fn get_protected_two(_user: Authenticated) -> &'static str {
    "Protected"
}

mod tests {
    use super::*;
    use rocket::routes;
    use rocket::local::blocking::Client;
    use rocket::http::{Header, Status};

    #[test]
    fn one_protected_returned_for_authenticated() {
        let rocket = rocket::build().mount("/",
            routes![get_protected_one, get_public_one, get_protected_two]);

        let client = Client::debug(rocket).unwrap();
        let req = client.get("/one").header(Header::new("Authenticated", "true"));
        let response = req.dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string(), Some("Protected".into()));
    }

    #[test]
    fn one_public_returned_for_unauthenticated() {
        let rocket = rocket::build().mount("/",
            routes![get_protected_one, get_public_one, get_protected_two]);

        let client = Client::debug(rocket).unwrap();
        let req = client.get("/one");
        let response = req.dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string(), Some("Public".into()));
    }

    #[test]
    fn two_unauthorized_returned_for_unauthenticated() {
        let rocket = rocket::build().mount("/",
            routes![get_protected_one, get_public_one, get_protected_two]);

        let client = Client::debug(rocket).unwrap();
        let req = client.get("/two");
        let response = req.dispatch();

        assert_eq!(response.status(), Status::Unauthorized);
    }
}
