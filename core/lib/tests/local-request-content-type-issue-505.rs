#[macro_use] extern crate rocket;

use rocket::{Request, Data};
use rocket::request::{self, FromRequest};
use rocket::outcome::IntoOutcome;
use rocket::http::Status;

struct HasContentType;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HasContentType {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, ()> {
        req.content_type().map(|_| HasContentType).or_forward(Status::NotFound)
    }
}

use rocket::data::{self, FromData};

#[rocket::async_trait]
impl<'r> FromData<'r> for HasContentType {
    type Error = ();

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        req.content_type().map(|_| HasContentType).or_forward((data, Status::NotFound))
    }
}

#[post("/")]
fn rg_ct(ct: Option<HasContentType>) -> &'static str {
    ct.map_or("Absent", |_| "Present")
}

#[post("/data", data = "<_ct>", rank = 1)]
fn data_has_ct(_ct: HasContentType) -> &'static str {
    "Data Present"
}

#[post("/data", rank = 2)]
fn data_no_ct() -> &'static str {
    "Data Absent"
}

mod local_request_content_type_tests {
    use super::*;

    use rocket::{Rocket, Build};
    use rocket::local::blocking::Client;
    use rocket::http::ContentType;

    fn rocket() -> Rocket<Build> {
        rocket::build().mount("/", routes![rg_ct, data_has_ct, data_no_ct])
    }

    #[test]
    fn has_no_ct() {
        let client = Client::debug(rocket()).unwrap();

        let req = client.post("/");
        assert_eq!(req.clone().dispatch().into_string(), Some("Absent".to_string()));
        assert_eq!(req.clone().dispatch().into_string(), Some("Absent".to_string()));
        assert_eq!(req.dispatch().into_string(), Some("Absent".to_string()));

        let req = client.post("/data");
        assert_eq!(req.clone().dispatch().into_string(), Some("Data Absent".to_string()));
        assert_eq!(req.clone().dispatch().into_string(), Some("Data Absent".to_string()));
        assert_eq!(req.dispatch().into_string(), Some("Data Absent".to_string()));
    }

    #[test]
    fn has_ct() {
        let client = Client::debug(rocket()).unwrap();

        let req = client.post("/").header(ContentType::JSON);
        assert_eq!(req.clone().dispatch().into_string(), Some("Present".to_string()));
        assert_eq!(req.clone().dispatch().into_string(), Some("Present".to_string()));
        assert_eq!(req.dispatch().into_string(), Some("Present".to_string()));

        let req = client.post("/data").header(ContentType::JSON);
        assert_eq!(req.clone().dispatch().into_string(), Some("Data Present".to_string()));
        assert_eq!(req.clone().dispatch().into_string(), Some("Data Present".to_string()));
        assert_eq!(req.dispatch().into_string(), Some("Data Present".to_string()));
    }
}
