#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use std::path::PathBuf;
use rocket::http::uri::SegmentError;

#[post("/<a>/<b..>")]
fn get(a: String, b: PathBuf) -> String {
    format!("{}/{}", a, b.to_string_lossy())
}

#[post("/<a>/<b..>")]
fn get2(a: String, b: Result<PathBuf, SegmentError>) -> String {
    format!("{}/{}", a, b.unwrap().to_string_lossy())
}

fn main() {  }
