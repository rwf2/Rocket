#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::request::Form;

#[derive(FromForm)]
struct FormData {
    form_data: String,
}

#[post("/", data = "<form_data>")]
fn bug(form_data: Form<FormData>) -> String {
    form_data.into_inner().form_data
}

#[cfg(feature = "testing")]
mod tests {
    use super::*;
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;
    use rocket::http::ContentType;
    use rocket::http::Status;

    fn check_decoding(raw: &str, decoded: &str) {
        let rocket = rocket::ignite().mount("/", routes![bug]);
        let mut req = MockRequest::new(Post, "/")
            .header(ContentType::Form)
            .body(format!("form_data={}", raw));

        let mut response = req.dispatch_with(&rocket);
        let body_string = response.body().and_then(|b| b.into_string());
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(Some(decoded.to_string()), body_string);
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
