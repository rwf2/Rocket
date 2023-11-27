use rocket;

use rocket::form::{FromForm, FromFormField};
use rocket::response::Responder;
use rocket::{get, routes};

#[derive(FromFormField)]
enum Thing {
    A,
    B,
    C,
}

impl std::fmt::Display for Thing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    DerivedResponder {
        data: "hello".to_string(),
    }
}

#[get("/?<params..>")]
fn number(params: ThingForm) -> DerivedResponder {
    DerivedResponder {
        data: params.thing.to_string(),
    }
}

#[test]
fn test_derive_reexports() {
    use rocket::local::blocking::Client;

    let client = Client::debug_with(routes![index, number]).unwrap();

    let response = client.get("/").dispatch();
    assert_eq!(response.into_string().unwrap(), "hello");

    let response = client.get("/?thing=b").dispatch();
    assert_eq!(response.into_string().unwrap(), "b");
}
