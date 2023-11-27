#[macro_use]
extern crate rocket;

use rocket::form::Form;

#[derive(FromForm)]
struct FormData {
    form_data: String,
}

#[patch("/", data = "<form_data>")]
fn bug(form_data: Form<FormData>) -> &'static str {
    assert_eq!("Form data", form_data.into_inner().form_data);
    "OK"
}

mod tests {
    use super::*;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;

    #[test]
    fn method_eval() {
        let client = Client::debug_with(routes![bug]).unwrap();
        let response = client
            .post("/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data")
            .dispatch();

        assert_eq!(response.into_string(), Some("OK".into()));
    }

    #[test]
    fn get_passes_through() {
        let client = Client::debug_with(routes![bug]).unwrap();
        let response = client
            .get("/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data")
            .dispatch();

        assert_eq!(response.status(), Status::NotFound);
    }
}
