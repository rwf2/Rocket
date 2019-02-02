#![feature(proc_macro_hygiene, decl_macro)]

extern crate rocket;

use rocket::{get, routes};
use rocket::request::{Form, FromForm, FromFormValue};
use rocket::response::Responder;

#[derive(FromFormValue)]
enum Thing {
    A,
    B,
    C,
}

impl std::fmt::Display for Thing {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Thing::A => write!(f, "a"),
            Thing::B => write!(f, "b"),
            Thing::C => write!(f, "c"),
        }
    }
}

#[derive(FromForm)]
struct ThingForm {
    thing: Thing,
}

#[derive(Responder)]
struct DerivedResponder {
    data: String,
}

#[get("/")]
fn index() -> DerivedResponder {
    DerivedResponder { data: "hello".to_string() }
}

#[get("/?<params..>")]
fn number(params: Form<ThingForm>) -> DerivedResponder {
    DerivedResponder { data: params.thing.to_string() }
}

#[test]
fn test_derive_reexports() {
    use rocket::local::Client;

    let rocket = rocket::ignite().mount("/", routes![index, number]);
    let client = Client::new(rocket).unwrap();

    let mut response = client.get("/").dispatch();
    assert_eq!(response.body_string().unwrap(), "hello");

    let mut response = client.get("/?thing=b").dispatch();
    assert_eq!(response.body_string().unwrap(), "b");
}
