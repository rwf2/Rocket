#[macro_use] extern crate rocket;

use rocket::request::FromParam;

struct S;

impl<'a> FromParam<'a> for S {
    type Error = ();
    fn from_param(param: &'a str) -> Result<Self, Self::Error> { Ok(S) }
}

#[post("/<id>")]
fn simple(id: usize) {  }

#[post("/<id>/<name>")]
fn not_uri_display(id: i32, name: S) {  }

#[post("/<id>/<name>")]
fn not_uri_display_but_unused(id: i32, name: S) {  }

#[post("/<id>/<name>")]
fn optionals(id: Option<i32>, name: Result<String, &str>) {  }

use rocket::form::{FromFormField, Errors, ValueField, DataField};

#[rocket::async_trait]
impl<'v> FromFormField<'v> for S {
    fn default() -> Option<Self> { None }

    fn from_value(_: ValueField<'v>) -> Result<Self, Errors<'v>> { Ok(S) }

    async fn from_data(_: DataField<'v, '_>) -> Result<Self, Errors<'v>> { Ok(S) }
}

#[post("/?<id>")]
fn simple_q(id: isize) {  }

#[post("/?<id>&<rest..>")]
fn other_q(id: usize, rest: S) {  }

#[post("/?<id>&<name>")]
fn optionals_q(id: Option<i32>, name: Result<String, Errors<'_>>) {  }

fn main() {
    uri!(simple(id = "hi"));

    uri!(simple("hello"));

    uri!(simple(id = 239239i64));

    uri!(not_uri_display(10, S));

    // This one is okay. In paths, a value _must_ be supplied.
    uri!(optionals(id = 10, name = "bob".to_string()));

    uri!(optionals(id = Some(10), name = Ok("bob".into())));

    uri!(simple_q("hi"));

    uri!(simple_q(id = "hi"));

    uri!(other_q(100, S));

    uri!(other_q(rest = S, id = 100));

    uri!(other_q(rest = _, id = 100));

    uri!(other_q(rest = S, id = _));

    // These are all okay.
    uri!(optionals_q(_, _));
    uri!(optionals_q(id = Some(10), name = Some("Bob".to_string())));
    uri!(optionals_q(_, Some("Bob".into())));
    uri!(optionals_q(id = _, name = _));

    // Invalid prefixes.
    uri!(uri!("?foo#bar"), simple(id = "hi"));
    uri!(uri!("*"), simple(id = "hi"));

    // Invalid suffix.
    uri!(_, simple(id = "hi"), uri!("*"));
    uri!(_, simple(id = "hi"), uri!("/foo/bar"));
}
