#[macro_use]
extern crate rocket;

use rocket::form::{Form, Strict};

#[derive(FromForm)]
struct MyForm<'r> {
    field: &'r str,
}

#[post("/strict", data = "<form>")]
fn strict<'r>(form: Form<Strict<MyForm<'r>>>) -> &'r str {
    form.field
}

#[post("/lenient", data = "<form>")]
fn lenient<'r>(form: Form<MyForm<'r>>) -> &'r str {
    form.field
}

mod strict_and_lenient_forms_tests {
    use super::*;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;

    const FIELD_VALUE: &str = "just_some_value";

    fn client() -> Client {
        Client::debug_with(routes![strict, lenient]).unwrap()
    }

    #[test]
    fn test_strict_form() {
        let client = client();
        let response = client
            .post("/strict")
            .header(ContentType::Form)
            .body(format!("field={}", FIELD_VALUE))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string(), Some(FIELD_VALUE.into()));

        let response = client
            .post("/strict")
            .header(ContentType::Form)
            .body(format!("field={}&extra=whoops", FIELD_VALUE))
            .dispatch();

        assert_eq!(response.status(), Status::UnprocessableEntity);
    }

    #[test]
    fn test_lenient_form() {
        let client = client();
        let response = client
            .post("/lenient")
            .header(ContentType::Form)
            .body(format!("field={}", FIELD_VALUE))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string(), Some(FIELD_VALUE.into()));

        let response = client
            .post("/lenient")
            .header(ContentType::Form)
            .body(format!("field={}&extra=whoops", FIELD_VALUE))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string(), Some(FIELD_VALUE.into()));
    }
}
