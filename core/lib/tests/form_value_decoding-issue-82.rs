#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::request::Form;

#[derive(FromForm)]
struct FormData {
    form_data: String,
}

#[post("/", data = "<form_data>")]
fn bug(form_data: Form<FormData>) -> String {
    form_data.into_inner().form_data
}

mod tests {
    use super::*;
    use rocket::local::Client;
    use rocket::http::ContentType;
    use rocket::http::Status;

    fn check_decoding(raw: &str, decoded: &str) {
        let client = Client::new(rocket::ignite().mount("/", routes![bug])).unwrap();
        let mut response = client.post("/")
            .header(ContentType::Form)
            .body(format!("form_data={}", raw))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(Some(decoded.to_string()), response.body_string());
    }

    #[test]
    fn test_proper_decoding() {
        check_decoding("password", "password");
        check_decoding("", "");
        check_decoding("+", " ");
        check_decoding("%2B", "+");
        check_decoding("1+1", "1 1");
        check_decoding("1%2B1", "1+1");
        check_decoding("%3Fa%3D1%26b%3D2", "?a=1&b=2");
    }
}
