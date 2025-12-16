#[macro_use]
extern crate rocket_community as rocket;

use rocket::form::Form;

#[derive(FromForm)]
struct FormData {
    form_data: String,
}

#[patch("/", data = "<form_data>")]
fn patch(form_data: Form<FormData>) -> &'static str {
    assert_eq!("Form data", form_data.into_inner().form_data);
    "PATCH OK"
}

#[route("/", method = UPDATEREDIRECTREF, data = "<form_data>")]
fn urr(form_data: Form<FormData>) -> &'static str {
    assert_eq!("Form data", form_data.into_inner().form_data);
    "UPDATEREDIRECTREF OK"
}

#[route("/", method = "VERSION-CONTROL", data = "<form_data>")]
fn vc(form_data: Form<FormData>) -> &'static str {
    assert_eq!("Form data", form_data.into_inner().form_data);
    "VERSION-CONTROL OK"
}

mod tests {
    use super::*;
    use rocket::http::{ContentType, Method, Status};
    use rocket::local::blocking::Client;

    #[test]
    fn method_eval() {
        let client = Client::debug_with(routes![patch, urr, vc]).unwrap();
        let response = client
            .post("/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data")
            .dispatch();

        assert_eq!(response.into_string(), Some("PATCH OK".into()));

        let response = client
            .post("/")
            .header(ContentType::Form)
            .body("_method=updateredirectref&form_data=Form+data")
            .dispatch();

        assert_eq!(response.into_string(), Some("UPDATEREDIRECTREF OK".into()));

        let response = client
            .req(Method::UpdateRedirectRef, "/")
            .header(ContentType::Form)
            .body("form_data=Form+data")
            .dispatch();

        assert_eq!(response.into_string(), Some("UPDATEREDIRECTREF OK".into()));

        let response = client
            .post("/")
            .header(ContentType::Form)
            .body("_method=version-control&form_data=Form+data")
            .dispatch();

        assert_eq!(response.into_string(), Some("VERSION-CONTROL OK".into()));
    }

    #[test]
    fn get_passes_through() {
        let client = Client::debug_with(routes![patch, urr, vc]).unwrap();
        let response = client
            .get("/")
            .header(ContentType::Form)
            .body("_method=patch&form_data=Form+data")
            .dispatch();

        assert_eq!(response.status(), Status::NotFound);
    }
}
