#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::request::{FromForm, FromFormValue, FormItems};
use rocket::http::RawStr;

#[derive(Debug, PartialEq, FromForm)]
struct Form {
    single: usize,
    #[form(field = "camelCase")]
    camel_case: String,
    #[form(field = "TitleCase")]
    title_case: String,
    #[form(field = "type")]
    field_type: isize,
    #[form(field = "DOUBLE")]
    double: String,
    #[form(field = "a.b")]
    dot: isize,
}

fn parse<'f, T: FromForm<'f>>(string: &'f str, strict: bool) -> Option<T> {
    let mut items = FormItems::from(string);
    let result = T::from_form(items.by_ref(), strict);
    if !items.exhaust() {
        panic!("Invalid form input.");
    }

    result.ok()
}

fn parse_strict<'f, T: FromForm<'f>>(string: &'f str) -> Option<T> {
    parse(string, true)
}

fn main() {
    let form_string = &[
        "single=100", "camelCase=helloThere", "TitleCase=HiHi", "type=-2",
        "DOUBLE=bing_bong", "a.b=123",
    ].join("&");

    let form: Option<Form> = parse_strict(&form_string);
    assert_eq!(form, Some(Form {
        single: 100,
        camel_case: "helloThere".into(),
        title_case: "HiHi".into(),
        field_type: -2,
        double: "bing_bong".into(),
        dot: 123,
    }));

    let form_string = &[
        "single=100", "camel_case=helloThere", "TitleCase=HiHi", "type=-2",
        "DOUBLE=bing_bong", "dot=123",
    ].join("&");

    let form: Option<Form> = parse_strict(&form_string);
    assert!(form.is_none());
}
