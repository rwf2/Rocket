#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::request::Form;

#[derive(FromForm)]
struct FormData {
    form_data: String,
}

#[patch("/", data = "<form_data>")]
fn bug(form_data: Form<FormData>) -> &'static str {
    assert_eq!("Form data", form_data.form_data);
    "OK"
}

mod tests {
    use super::*;
    use rocket::local::Client;
    use rocket::http::{Status, ContentType};

    #[test]
    fn method_eval() {
        let client = Client::new(rocket::ignite().mount("/", routes![bug])).unwrap();
        let mut response = client.post("/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data")
            .dispatch();

        assert_eq!(response.body_string(), Some("OK".into()));
    }

    #[test]
    fn get_passes_through() {
        let client = Client::new(rocket::ignite().mount("/", routes![bug])).unwrap();
        let response = client.get("/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data")
            .dispatch();

        assert_eq!(response.status(), Status::NotFound);
    }
}
