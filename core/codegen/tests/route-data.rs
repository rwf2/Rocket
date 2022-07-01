#[macro_use] extern crate rocket;

use rocket::{Request, Data};
use rocket::local::blocking::Client;
use rocket::data::{self, FromData};
use rocket::http::ContentType;
use rocket::form::Form;

// Test that the data parameters works as expected.

#[derive(FromForm)]
struct Inner<'r> {
    field: &'r str
}

struct Simple<'r>(&'r str);

#[async_trait]
impl<'r> FromData<'r> for Simple<'r> {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        <&'r str>::from_data(req, data).await.map(Simple)
    }
}

#[post("/f", data = "<form>")]
fn form<'r>(form: Form<Inner<'r>>) -> &'r str { form.into_inner().field }

#[post("/s", data = "<simple>")]
fn simple<'r>(simple: Simple<'r>) -> &'r str { simple.0 }

#[test]
fn test_data() {
    let rocket = rocket::build().mount("/", routes![form, simple]);
    let client = Client::debug(rocket).unwrap();

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
