#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::http::RawStr;
use rocket::request::FromParam;

struct S;

impl<'a> FromParam<'a> for S {
    type Error = ();
    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> { Ok(S) }
}

#[post("/<id>")]
fn simple(id: i32) {  }

#[post("/<id>/<name>")]
fn not_uri_display(id: i32, name: S) {  }

#[post("/<id>/<name>")]
fn not_uri_display_but_unused(id: i32, name: S) {  }

fn main() {
    uri!(simple: id = "hi"); //~ ERROR i32: rocket::http::uri::FromUriParam<&str>
    uri!(simple: "hello"); //~ ERROR i32: rocket::http::uri::FromUriParam<&str>
    uri!(simple: id = 239239i64); //~ ERROR i32: rocket::http::uri::FromUriParam<i64>
    uri!(not_uri_display: 10, S); //~ ERROR S: rocket::http::uri::FromUriParam<_>
}
