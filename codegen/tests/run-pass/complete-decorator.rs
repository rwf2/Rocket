#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::http::{Cookies, RawStr};
use rocket::request::Form;

#[derive(FromForm)]
struct User<'a> {
    name: &'a RawStr,
    nickname: String,
}

#[post("/<name>?<query>", format = "application/json", data = "<user>", rank = 2)]
fn get<'r>(name: &RawStr,
           query: User<'r>,
           user: Form<'r, User<'r>>,
           cookies: Cookies)
           -> &'static str {
    "hi"
}

fn main() {
    let _ = routes![get];
}
