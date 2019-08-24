#![feature(proc_macro_hygiene)]

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

    async fn check_decoding(raw: &str, decoded: &str) {
        let client = Client::new(rocket::ignite().mount("/", routes![bug])).unwrap();
        let mut response = client.post("/")
            .header(ContentType::Form)
            .body(format!("form_data={}", raw))
            .dispatch().await;

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(Some(decoded.to_string()), response.body_string().await);
    }

    #[rocket::async_test]
    async fn test_proper_decoding() {
        check_decoding("password", "password").await;
        check_decoding("", "").await;
        check_decoding("+", " ").await;
        check_decoding("%2B", "+").await;
        check_decoding("1+1", "1 1").await;
        check_decoding("1%2B1", "1+1").await;
        check_decoding("%3Fa%3D1%26b%3D2", "?a=1&b=2").await;
    }
}
