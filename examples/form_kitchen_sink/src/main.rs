#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use std::io;
use rocket::request::{Form, FromFormValue};
use rocket::response::NamedFile;
use rocket::http::RawStr;

// TODO: Make deriving `FromForm` for this enum possible.
#[derive(Debug)]
enum FormOption {
    A, B, C
}

impl<'v> FromFormValue<'v> for FormOption {
    type Error = &'v RawStr;

    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        let variant = match v.as_str() {
            "a" => FormOption::A,
            "b" => FormOption::B,
            "c" => FormOption::C,
            _ => return Err(v)
        };

        Ok(variant)
    }
}

#[derive(Debug, FromForm)]
struct FormInput {
    checkbox: bool,
    number: usize,
    #[form(field = "type")]
    radio: FormOption,
    password: String,
    #[form(field = "textarea")]
    text_area: String,
    select: FormOption,
}

#[post("/", data = "<sink>")]
fn sink(sink: Result<Form<FormInput>, Option<String>>) -> String {
    match sink {
        Ok(form) => format!("{:?}", form.get()),
        Err(Some(f)) => format!("Invalid form input: {}", f),
        Err(None) => format!("Form input was invalid UTF8."),
    }
}

#[get("/")]
fn index() -> io::Result<NamedFile> {
    NamedFile::open("static/index.html")
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, sink])
        .launch();
}
