#[macro_use]
extern crate rocket;

use rocket::form::Form;

#[post("/", data = "<form_data>")]
fn bug(form_data: Form<String>) -> String {
    form_data.into_inner()
}

mod tests {
    use super::*;
    use rocket::http::ContentType;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    fn check_decoding(raw: &str, decoded: &str) {
        let client = Client::debug_with(routes![bug]).unwrap();
        let response = client
            .post("/")
            .header(ContentType::Form)
            .body(format!("form_data={}", raw))
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(Some(decoded.to_string()), response.into_string());
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
