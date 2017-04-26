#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::request::Form;

#[derive(FromForm)]
struct FormData {
    form_data: String,
}

#[patch("/", data = "<form_data>")]
fn bug(form_data: Form<FormData>) -> &'static str {
    assert_eq!("Form data", &form_data.get().form_data);
    "OK"
}

mod tests {
    use super::*;
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;
    use rocket::http::{Status, ContentType};

    #[test]
    fn method_eval() {
        let rocket = rocket::ignite().mount("/", routes![bug]);

        let mut req = MockRequest::new(Post, "/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data");

        let mut response = req.dispatch_with(&rocket);
        assert_eq!(response.body_string(), Some("OK".into()));
    }

    #[test]
    fn get_passes_through() {
        let rocket = rocket::ignite().mount("/", routes![bug]);

        let mut req = MockRequest::new(Get, "/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data");

        let response = req.dispatch_with(&rocket);
        assert_eq!(response.status(), Status::NotFound);
    }
}
