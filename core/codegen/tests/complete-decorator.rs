#![feature(plugin, decl_macro, proc_macro_hygiene)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;

use rocket::http::{Cookies, RawStr};
use rocket::request::Form;

#[derive(FromForm)]
struct User<'a> {
    name: &'a RawStr,
    nickname: String,
}

#[post("/<_name>?<_query>", format = "application/json", data = "<user>", rank = 2)]
fn get(
    _name: &RawStr,
    _query: User,
    user: Form<User>,
    _cookies: Cookies
) -> String {
    format!("{}:{}", user.name, user.nickname)
}

#[test]
fn main() {
    let _ = routes![get];
}
