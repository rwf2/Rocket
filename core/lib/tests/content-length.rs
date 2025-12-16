#[macro_use]
extern crate rocket_community as rocket;

#[get("/")]
fn index() -> String {
    "Hello, world!".into()
}

#[test]
fn content_length_header() {
    let rocket = rocket::build().mount("/", routes![index]);
    let client = rocket::local::blocking::Client::debug(rocket).unwrap();
    let response = client.get("/").dispatch();
    assert!(response.headers().get_one("Content-Length").is_some());
}
