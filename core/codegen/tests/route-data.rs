#[macro_use] extern crate rocket;

use rocket::{Request, Data};
use rocket::local::blocking::Client;
use rocket::request::Form;
use rocket::data::{self, FromData, ToByteUnit};
use rocket::http::{RawStr, ContentType, Status};

// Test that the data parameters works as expected.

#[derive(FromForm)]
struct Inner<'r> {
    field: &'r RawStr
}

struct Simple(String);

#[async_trait]
impl FromData for Simple {
    type Error = ();

    async fn from_data(_: &Request<'_>, data: Data) -> data::Outcome<Self, ()> {
        match data.open(64.bytes()).stream_to_string().await {
            Ok(string) => data::Outcome::Success(Simple(string)),
            Err(_) => data::Outcome::Failure((Status::InternalServerError, ())),
        }
    }
}

#[post("/f", data = "<form>")]
fn form(form: Form<Inner<'_>>) -> String { form.field.url_decode_lossy() }

#[post("/s", data = "<simple>")]
fn simple(simple: Simple) -> String { simple.0 }

#[test]
fn test_data() {
    let rocket = rocket::ignite().mount("/", routes![form, simple]);
    let client = Client::new(rocket).unwrap();

    let response = client.post("/f")
        .header(ContentType::Form)
        .body("field=this%20is%20here")
        .dispatch();

    assert_eq!(response.into_string().unwrap(), "this is here");

    let response = client.post("/s").body("this is here").dispatch();
    assert_eq!(response.into_string().unwrap(), "this is here");

    let response = client.post("/s").body("this%20is%20here").dispatch();
    assert_eq!(response.into_string().unwrap(), "this%20is%20here");
}
