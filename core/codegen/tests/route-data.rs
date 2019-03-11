#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use std::io::Read;

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

    fn from_data(_: &Request, data: Data) -> data::Outcome<Self, ()> {
        let mut string = String::new();
        if let Err(_) = data.open().take(64).read_to_string(&mut string) {
            return Failure((Status::InternalServerError, ()));
        }

        Success(Simple(string))
    }
}

#[post("/f", data = "<form>")]
fn form(form: Form<Inner>) -> String { form.field.url_decode_lossy() }

#[post("/s", data = "<simple>")]
fn simple(simple: Simple) -> String { simple.0 }

#[test]
fn test_data() {
    let rocket = rocket::ignite().mount("/", routes![form, simple]);
    let client = Client::new(rocket).unwrap();

    let mut response = client.post("/f")
        .header(ContentType::Form)
        .body("field=this%20is%20here")
        .dispatch();

    assert_eq!(response.body_string().unwrap(), "this is here");

    let mut response = client.post("/s").body("this is here").dispatch();
    assert_eq!(response.body_string().unwrap(), "this is here");

    let mut response = client.post("/s").body("this%20is%20here").dispatch();
    assert_eq!(response.body_string().unwrap(), "this%20is%20here");
}
