#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::request::{FromForm, FormItems};

#[derive(PartialEq, Debug, FromForm)]
struct Form {  }

fn main() {
    // Same number of arguments: simple case.
    let task = Form::from_form(&mut FormItems::from(""), true);
    assert_eq!(task, Ok(Form { }));

    let task = Form::from_form(&mut FormItems::from(""), false);
    assert_eq!(task, Ok(Form { }));
}
