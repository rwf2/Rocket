#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::request::{Form, LenientForm};
use rocket::http::RawStr;

#[derive(FromForm)]
struct MyForm<'r> {
    field: &'r RawStr,
}

#[post("/strict", data = "<form>")]
fn strict<'r>(form: Form<MyForm<'r>>) -> String {
    form.field.as_str().into()
}

#[post("/lenient", data = "<form>")]
fn lenient<'r>(form: LenientForm<MyForm<'r>>) -> String {
    form.field.as_str().into()
}

mod strict_and_lenient_forms_tests {
    use super::*;
    use rocket::local::Client;
    use rocket::http::{Status, ContentType};

    const FIELD_VALUE: &str = "just_some_value";

    fn client() -> Client {
        Client::new(rocket::ignite().mount("/", routes![strict, lenient])).unwrap()
    }

    #[test]
    fn test_strict_form() {
        let client = client();
        let mut response = client.post("/strict")
            .header(ContentType::Form)
            .body(format!("field={}", FIELD_VALUE))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some(FIELD_VALUE.into()));

        let response = client.post("/strict")
            .header(ContentType::Form)
            .body(format!("field={}&extra=whoops", FIELD_VALUE))
            .dispatch();

        assert_eq!(response.status(), Status::UnprocessableEntity);
    }

    #[test]
    fn test_lenient_form() {
        let client = client();
        let mut response = client.post("/lenient")
            .header(ContentType::Form)
            .body(format!("field={}", FIELD_VALUE))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some(FIELD_VALUE.into()));

        let mut response = client.post("/lenient")
            .header(ContentType::Form)
            .body(format!("field={}&extra=whoops", FIELD_VALUE))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some(FIELD_VALUE.into()));
    }
}
