use std::collections::HashMap;

use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::*;
use rocket_contrib::Template;

#[test]
fn test_submit() {
    let rocket = rocket::ignite().mount("/", routes![super::submit]);
    let mut request = MockRequest::new(Method::Post, "/submit")
        .header(ContentType::Form)
        .body("message=Hello from Rocket!");
    let response = request.dispatch_with(&rocket);
    let cookie_headers: Vec<_> = response.header_values("Set-Cookie").collect();
    let location_headers: Vec<_> = response.header_values("Location").collect();

    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(cookie_headers, vec!["message=Hello%20from%20Rocket!".to_string()]);
    assert_eq!(location_headers, vec!["/".to_string()]);
}

fn test_body(optional_cookie: Option<Cookie>, expected_body: String) {
    let rocket = rocket::ignite().mount("/", routes![super::index]);
    let mut request = MockRequest::new(Method::Get, "/");

    // Attach a cookie if one is given.
    if let Some(cookie) = optional_cookie {
        request = request.cookie(cookie);
    }

    let mut response = request.dispatch_with(&rocket);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.body().and_then(|b| b.into_string()), Some(expected_body));
}

#[test]
fn test_index() {
    // Render the template with an empty context to test against.
    let mut context: HashMap<&str, &str> = HashMap::new();
    let template = Template::render("index", &context);

    // Test the route without sending the "message" cookie.
    test_body(None, template.to_string());

    // Render the template with a context that contains the message.
    context.insert("message", "Hello from Rocket!");
    let template = Template::render("index", &context);

    // Test the route with the "message" cookie.
    test_body(Some(Cookie::new("message".to_string(),
                               "Hello from Rocket!".to_string())),
              template.to_string());
}
