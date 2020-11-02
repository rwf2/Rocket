use std::collections::HashMap;

use super::rocket;
use rocket::local::blocking::Client;
use rocket::http::*;
use rocket_contrib::templates::Template;

#[test]
fn test_submit() {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.post("/submit")
        .header(ContentType::Form)
        .body("message=Hello from Rocket!")
        .dispatch();

    let cookie_headers: Vec<_> = response.headers().get("Set-Cookie").collect();
    assert_eq!(cookie_headers.len(), 1);
    assert!(cookie_headers[0].starts_with("message=Hello%20from%20Rocket!"));

    let location_headers: Vec<_> = response.headers().get("Location").collect();
    assert_eq!(location_headers, vec!["/".to_string()]);
    assert_eq!(response.status(), Status::SeeOther);
}

fn test_body(optional_cookie: Option<Cookie<'static>>, expected_body: String) {
    // Attach a cookie if one is given.
    let client = Client::tracked(rocket()).unwrap();
    let response = match optional_cookie {
        Some(cookie) => client.get("/").cookie(cookie).dispatch(),
        None => client.get("/").dispatch(),
    };

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string(), Some(expected_body));
}

#[test]
fn test_index() {
    let client = Client::tracked(rocket()).unwrap();

    // Render the template with an empty context.
    let mut context: HashMap<&str, &str> = HashMap::new();
    let template = Template::show(client.rocket(), "index", &context).unwrap();
    test_body(None, template);

    // Render the template with a context that contains the message.
    context.insert("message", "Hello from Rocket!");
    let template = Template::show(client.rocket(), "index", &context).unwrap();
    test_body(Some(Cookie::new("message", "Hello from Rocket!")), template);
}
