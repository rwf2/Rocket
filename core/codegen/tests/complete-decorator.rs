#![feature(plugin, decl_macro, proc_macro_non_items)]
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
fn get<'r>(
    _name: &RawStr,
    _query: User<'r>,
    user: Form<'r, User<'r>>,
    _cookies: Cookies
) -> String {
    format!("{}:{}", user.get().name, user.get().nickname)
}

#[test]
fn main() {
    let _ = routes![get];
}
