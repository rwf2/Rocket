use rocket::local::Client;

#[test]
fn hello_world() {
    let rocket = rocket::ignite().mount("/", routes![super::hello]);
    let client = Client::new(rocket).unwrap();
    let mut response = client.get("/").dispatch();
    assert_eq!(response.body_string(), Some("Hello, world!".into()));
}
