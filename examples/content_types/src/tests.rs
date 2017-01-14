use super::rocket;
use super::serde_json;
use super::Person;
use rocket::http::{ContentType, Method, Status};
use rocket::testing::MockRequest;

fn test(uri: &str, content_type: ContentType, status: Status, body: String) {
    let rocket = rocket::ignite()
        .mount("/hello", routes![super::hello])
        .catch(errors![super::not_found]);
    let mut request = MockRequest::new(Method::Get, uri).header(content_type);
    let mut response = request.dispatch_with(&rocket);

    assert_eq!(response.status(), status);
    assert_eq!(response.body().and_then(|b| b.into_string()), Some(body));
}

#[test]
fn test_hello() {
    let person = Person {
        name: "Michael".to_string(),
        age: 80,
    };
    let body = serde_json::to_string(&person).unwrap();
    test("/hello/Michael/80", ContentType::JSON, Status::Ok, body);
}

#[test]
fn test_hello_invalid_content_type() {
    let body = format!("<p>This server only supports JSON requests, not '{}'.</p>",
                       ContentType::HTML);
    test("/hello/Michael/80", ContentType::HTML, Status::NotFound, body);
}

#[test]
fn test_404() {
    let body = "<p>Sorry, '/unknown' is an invalid path! Try \
                /hello/&lt;name&gt;/&lt;age&gt; instead.</p>";
    test("/unknown", ContentType::JSON, Status::NotFound, body.to_string());
}
