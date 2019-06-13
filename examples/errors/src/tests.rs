use rocket::local::Client;
use rocket::http::Status;

fn test(uri: &str, status: Status, body: String) {
    let rocket = rocket::ignite()
        .mount("/", routes![super::hello])
        .register(catchers![super::not_found]);

    let client = Client::new(rocket).unwrap();
    let mut response = client.get(uri).dispatch();
    assert_eq!(response.status(), status);
    assert_eq!(response.body_string(), Some(body));
}

#[test]
fn test_hello() {
    let (name, age) = ("Arthur", 42);
    let uri = format!("/hello/{}/{}", name, age);
    test(&uri, Status::Ok, format!("Hello, {} year old named {}!", age, name));
}

#[test]
fn test_hello_invalid_age() {
    for &(name, age) in &[("Ford", -129), ("Trillian", 128)] {
        let uri = format!("/hello/{}/{}", name, age);
        let body = format!("<p>Sorry, but '{}' is not a valid path!</p>
            <p>Try visiting /hello/&lt;name&gt;/&lt;age&gt; instead.</p>",
                           uri);
        test(&uri, Status::NotFound, body);
    }
}
