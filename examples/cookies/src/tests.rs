use std::collections::HashMap;

use super::rocket;
use rocket::local::Client;
use rocket::http::*;
use rocket_contrib::Template;

const TEMPLATE_ROOT: &'static str = "templates/";

#[test]
fn test_submit() {
    let client = Client::new(rocket()).unwrap();
    let response = client.post("/submit")
        .header(ContentType::Form)
        .body("message=Hello from Rocket!")
        .dispatch();

    let cookie_headers: Vec<_> = response.headers().get("Set-Cookie").collect();
    let location_headers: Vec<_> = response.headers().get("Location").collect();

    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(cookie_headers, vec!["message=Hello%20from%20Rocket!".to_string()]);
    assert_eq!(location_headers, vec!["/".to_string()]);
}

fn test_body(optional_cookie: Option<Cookie<'static>>, expected_body: String) {
    // Attach a cookie if one is given.
    let client = Client::new(rocket()).unwrap();
    let mut response = match optional_cookie {
        Some(cookie) => client.get("/").cookie(cookie).dispatch(),
        None => client.get("/").dispatch(),
    };

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.body_string(), Some(expected_body));
}

#[test]
fn test_index() {
    // Render the template with an empty context.
    let mut context: HashMap<&str, &str> = HashMap::new();
    let template = Template::show(TEMPLATE_ROOT, "index", &context).unwrap();
    test_body(None, template);

    // Render the template with a context that contains the message.
    context.insert("message", "Hello from Rocket!");
    let template = Template::show(TEMPLATE_ROOT, "index", &context).unwrap();
    test_body(Some(Cookie::new("message", "Hello from Rocket!")), template);
}
