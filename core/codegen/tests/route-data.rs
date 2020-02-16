#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::{Request, Data, Outcome::*};
use rocket::local::Client;
use rocket::request::Form;
use rocket::data::{self, FromDataSimple};
use rocket::http::{RawStr, ContentType, Status};

// Test that the data parameters works as expected.

#[derive(FromForm)]
struct Inner<'r> {
    field: &'r RawStr
}

struct Simple(String);

impl FromDataSimple for Simple {
    type Error = ();

    fn from_data(_: &Request<'_>, data: Data) -> data::FromDataFuture<'static, Self, ()> {
        Box::pin(async {
            use tokio::io::AsyncReadExt;

            let mut string = String::new();
            let mut stream = data.open().take(64);
            if let Err(_) = stream.read_to_string(&mut string).await {
                return Failure((Status::InternalServerError, ()));
            }

            Success(Simple(string))
        })
    }
}

#[post("/f", data = "<form>")]
fn form(form: Form<Inner<'_>>) -> String { form.field.url_decode_lossy() }

#[post("/s", data = "<simple>")]
fn simple(simple: Simple) -> String { simple.0 }

#[rocket::async_test]
async fn test_data() {
    let rocket = rocket::ignite().mount("/", routes![form, simple]);
    let client = Client::new(rocket).await.unwrap();

    let mut response = client.post("/f")
        .header(ContentType::Form)
        .body("field=this%20is%20here")
        .dispatch().await;

    assert_eq!(response.body_string().await.unwrap(), "this is here");

    let mut response = client.post("/s").body("this is here").dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "this is here");

    let mut response = client.post("/s").body("this%20is%20here").dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "this%20is%20here");
}
