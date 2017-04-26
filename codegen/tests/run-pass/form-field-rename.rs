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
}

fn parse<'f, T: FromForm<'f>>(string: &'f str) -> Option<T> {
    let mut items = FormItems::from(string);
    let result = T::from_form_items(items.by_ref());
    if !items.exhaust() {
        panic!("Invalid form input.");
    }

    result.ok()
}

fn main() {
    let form_string = &[
        "single=100", "camelCase=helloThere", "TitleCase=HiHi", "type=-2",
        "DOUBLE=bing_bong"
    ].join("&");

    let form: Option<Form> = parse(&form_string);
    assert_eq!(form, Some(Form {
        single: 100,
        camel_case: "helloThere".into(),
        title_case: "HiHi".into(),
        field_type: -2,
        double: "bing_bong".into()
    }));

    let form_string = &[
        "single=100", "camel_case=helloThere", "TitleCase=HiHi", "type=-2",
        "DOUBLE=bing_bong"
    ].join("&");

    let form: Option<Form> = parse(&form_string);
    assert!(form.is_none());
}
