use super::Person;
use rocket::http::{Accept, ContentType, Header, MediaType, Method, Status};
use rocket::local::Client;

fn test<H>(method: Method, uri: &str, header: H, status: Status, body: String)
    where H: Into<Header<'static>>
{
    let rocket = rocket::ignite()
        .mount("/hello", routes![super::get_hello, super::post_hello])
        .register(catchers![super::not_found]);

    let client = Client::new(rocket).unwrap();
    let mut response = client.req(method, uri).header(header).dispatch();
    assert_eq!(response.status(), status);
    assert_eq!(response.body_string(), Some(body));
}

#[test]
fn test_hello() {
    let person = Person { name: "Michael".to_string(), age: 80, };
    let body = serde_json::to_string(&person).unwrap();
    test(Method::Get, "/hello/Michael/80", Accept::JSON, Status::Ok, body.clone());
    test(Method::Get, "/hello/Michael/80", Accept::Any, Status::Ok, body.clone());

    // No `Accept` header is an implicit */*.
    test(Method::Get, "/hello/Michael/80", ContentType::XML, Status::Ok, body);

    let person = Person { name: "".to_string(), age: 99, };
    let body = serde_json::to_string(&person).unwrap();
    test(Method::Post, "/hello/99", ContentType::Plain, Status::Ok, body);
}

#[test]
fn test_hello_invalid_content_type() {
    let b = format!("<p>'{}' requests are not supported.</p>", MediaType::HTML);
    test(Method::Get, "/hello/Michael/80", Accept::HTML, Status::NotFound, b.clone());
    test(Method::Post, "/hello/80", ContentType::HTML, Status::NotFound, b);
}

#[test]
fn test_404() {
    let body = "<p>Sorry, '/unknown' is an invalid path! Try \
                /hello/&lt;name&gt;/&lt;age&gt; instead.</p>";
    test(Method::Get, "/unknown", Accept::JSON, Status::NotFound, body.to_string());
}
