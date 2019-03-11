#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use std::fmt;

use rocket::http::{Cookies, RawStr};

#[post("/<id>")]
fn has_one(id: i32) {  }

#[post("/<id>")]
fn has_one_guarded(cookies: Cookies, id: i32) {  }

#[post("/<id>?<name>")]
fn has_two(cookies: Cookies, id: i32, name: String) {  }

#[post("/<id>/<name>")]
fn optionals(id: Option<i32>, name: Result<String, &RawStr>) {  }

fn main() {
    uri!(has_one); //~ ERROR expects 1 parameter but 0

    uri!(has_one: 1, 23); //~ ERROR expects 1 parameter but 2
    uri!(has_one: "Hello", 23, ); //~ ERROR expects 1 parameter but 2
    uri!(has_one_guarded: "hi", 100); //~ ERROR expects 1 parameter but 2

    uri!(has_two: 10, "hi", "there"); //~ ERROR expects 2 parameters but 3
    uri!(has_two: 10); //~ ERROR expects 2 parameters but 1

    uri!(has_one: id = 100, name = "hi"); //~ ERROR invalid parameters
    //~^ HELP unknown parameter: `name`

    uri!(has_one: name = 100, id = 100); //~ ERROR invalid parameters
    //~^ HELP unknown parameter: `name`

    uri!(has_one: name = 100, age = 50, id = 100); //~ ERROR invalid parameters
    //~^ HELP unknown parameters: `name`, `age`

    uri!(has_one: name = 100, age = 50, id = 100, id = 50); //~ ERROR invalid parameters
    //~^ HELP unknown parameters: `name`, `age`
    //~^^ HELP duplicate parameter: `id`

    uri!(has_one: id = 100, id = 100); //~ ERROR invalid parameters
    //~^ HELP duplicate parameter: `id`

    uri!(has_one: id = 100, id = 100, ); //~ ERROR invalid parameters
    //~^ HELP duplicate parameter: `id`

    uri!(has_one: name = "hi"); //~ ERROR invalid parameters
    //~^ HELP unknown parameter: `name`
    //~^^ HELP missing parameter: `id`

    uri!(has_one_guarded: cookies = "hi", id = 100); //~ ERROR invalid parameters
    //~^ HELP unknown parameter: `cookies`

    uri!(has_one_guarded: id = 100, cookies = "hi"); //~ ERROR invalid parameters
    //~^ HELP unknown parameter: `cookies`

    uri!(has_two: id = 100, id = 100, ); //~ ERROR invalid parameters
    //~^ HELP duplicate parameter: `id`
    //~^^ HELP missing parameter: `name`

    uri!(has_two: name = "hi"); //~ ERROR invalid parameters
    //~^ HELP missing parameter: `id`

    uri!(has_two: cookies = "hi", id = 100, id = 10, id = 10); //~ ERROR invalid parameters
    //~^ HELP duplicate parameter: `id`
    //~^^ HELP missing parameter: `name`
    //~^^^ HELP unknown parameter: `cookies`

    uri!(has_two: id = 100, cookies = "hi"); //~ ERROR invalid parameters
    //~^ HELP missing parameter: `name`
    //~^^ HELP unknown parameter: `cookies`

    uri!(optionals: id = _, name = "bob".into());
    //~^ ERROR cannot be ignored

    uri!(optionals: id = 10, name = _);
    //~^ ERROR cannot be ignored
}
