// normalize-stderr-test: "<(.*) as (.*)>" -> "$1 as $$TRAIT"
// normalize-stderr-test: "and \d+ others" -> "and $$N others"

#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::http::RawStr;
use rocket::request::FromParam;

struct S;

impl<'a> FromParam<'a> for S {
    type Error = ();
    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> { Ok(S) }
}

#[post("/<id>")]
fn simple(id: usize) {  }

#[post("/<id>/<name>")]
fn not_uri_display(id: i32, name: S) {  }

#[post("/<id>/<name>")]
fn not_uri_display_but_unused(id: i32, name: S) {  }

#[post("/<id>/<name>")]
fn optionals(id: Option<i32>, name: Result<String, &RawStr>) {  }

use rocket::request::{Query, FromQuery};

impl<'q> FromQuery<'q> for S {
    type Error = ();
    fn from_query(query: Query<'q>) -> Result<Self, Self::Error> { Ok(S) }
}

#[post("/?<id>")]
fn simple_q(id: isize) {  }

#[post("/?<id>&<rest..>")]
fn other_q(id: usize, rest: S) {  }

#[post("/?<id>&<name>")]
fn optionals_q(id: Option<i32>, name: Result<String, &RawStr>) {  }

fn main() {
    uri!(simple: id = "hi");
    //~^ ERROR usize: rocket::http::uri::FromUriParam<rocket::http::uri::Path, &str>

    uri!(simple: "hello");
    //~^ ERROR usize: rocket::http::uri::FromUriParam<rocket::http::uri::Path, &str>

    uri!(simple: id = 239239i64);
    //~^ ERROR usize: rocket::http::uri::FromUriParam<rocket::http::uri::Path, i64>

    uri!(not_uri_display: 10, S);
    //~^ ERROR S: rocket::http::uri::FromUriParam<rocket::http::uri::Path, _>

    // This one is okay. In paths, a value _must_ be supplied.
    uri!(optionals: id = 10, name = "bob".to_string());

    uri!(optionals: id = Some(10), name = Ok("bob".into()));
    //~^ ERROR i32: rocket::http::uri::FromUriParam<rocket::http::uri::Path, std::option::Option<{integer}>>
    //~^^ ERROR String: rocket::http::uri::FromUriParam<rocket::http::uri::Path, std::result::Result<_, _>>

    uri!(simple_q: "hi");
    //~^ ERROR isize: rocket::http::uri::FromUriParam<rocket::http::uri::Query, &str>

    uri!(simple_q: id = "hi");
    //~^ ERROR isize: rocket::http::uri::FromUriParam<rocket::http::uri::Query, &str>

    uri!(other_q: 100, S);
    //~^ ERROR S: rocket::http::uri::FromUriParam<rocket::http::uri::Query, _>

    uri!(other_q: rest = S, id = 100);
    //~^ ERROR S: rocket::http::uri::FromUriParam<rocket::http::uri::Query, _>

    uri!(other_q: rest = _, id = 100);
    //~^ ERROR S: rocket::http::uri::Ignorable<rocket::http::uri::Query>

    uri!(other_q: rest = S, id = _);
    //~^ ERROR S: rocket::http::uri::FromUriParam<rocket::http::uri::Query, _>
    //~^^ ERROR usize: rocket::http::uri::Ignorable<rocket::http::uri::Query>

    // These are all okay.
    uri!(optionals_q: _, _);
    uri!(optionals_q: id = 10, name = "Bob".to_string());
    uri!(optionals_q: _, "Bob".into());
    uri!(optionals_q: id = _, name = _);
}
