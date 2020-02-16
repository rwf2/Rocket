#![feature(proc_macro_hygiene)]

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

    #[rocket::async_test]
    async fn method_eval() {
        let client = Client::new(rocket::ignite().mount("/", routes![bug])).await.unwrap();
        let mut response = client.post("/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data")
            .dispatch().await;

        assert_eq!(response.body_string().await, Some("OK".into()));
    }

    #[rocket::async_test]
    async fn get_passes_through() {
        let client = Client::new(rocket::ignite().mount("/", routes![bug])).await.unwrap();
        let response = client.get("/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data")
            .dispatch().await;

        assert_eq!(response.status(), Status::NotFound);
    }
}
