#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

#[get("/")]
fn req(request: &rocket::Request) -> String {
    format!("Hello {}", request.uri().path())
}

mod request_guard_tests {
    use super::*;
    use rocket::local::Client;

    #[test]
    fn check_request_is_parsed() {
        let rocket = rocket::ignite()
            .mount("/req", routes![req]);

        let client = Client::new(rocket).unwrap();
        let mut res = client.get("/req").dispatch();
        assert_eq!(res.body_string(), Some("Hello /req".to_owned()));
    }
}
