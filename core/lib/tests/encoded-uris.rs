#[macro_use]
extern crate rocket;

#[get("/hello süper $?a&?&<value>")]
fn index(value: &str) -> &str {
    value
}

mod encoded_uris {
    use rocket::local::blocking::Client;

    #[test]
    fn can_route_to_encoded_uri() {
        let client = Client::debug_with(routes![super::index]).unwrap();
        let response = client
            .get("/hello%20s%C3%BCper%20%24?a&%3F&value=a+b")
            .dispatch()
            .into_string();

        assert_eq!(response.unwrap(), "a b");
    }
}
