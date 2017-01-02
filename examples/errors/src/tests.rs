use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::{Method, Status};

fn test(uri: &str, status: Status, body: String) {
    let rocket = rocket::ignite()
        .mount("/", routes![super::hello])
        .catch(errors![super::not_found]);
    let mut req = MockRequest::new(Method::Get, uri);
    let mut response = req.dispatch_with(&rocket);

    assert_eq!(response.status(), status);
    assert_eq!(response.body().and_then(|b| b.into_string()), Some(body));
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
