use std::collections::HashMap;

use super::rocket;
use rocket::local::asynchronous::Client;
use rocket::http::*;
use rocket_contrib::templates::Template;

#[rocket::async_test]
async fn test_submit() {
    let client = Client::new(rocket()).await.unwrap();
    let response = client.post("/submit")
        .header(ContentType::Form)
        .body("message=Hello from Rocket!")
        .dispatch().await;

    let cookie_headers: Vec<_> = response.headers().get("Set-Cookie").collect();
    let location_headers: Vec<_> = response.headers().get("Location").collect();

    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(cookie_headers, vec!["message=Hello%20from%20Rocket!".to_string()]);
    assert_eq!(location_headers, vec!["/".to_string()]);
}

async fn test_body(optional_cookie: Option<Cookie<'static>>, expected_body: String) {
    // Attach a cookie if one is given.
    let client = Client::new(rocket()).await.unwrap();
    let response = match optional_cookie {
        Some(cookie) => client.get("/").cookie(cookie).dispatch().await,
        None => client.get("/").dispatch().await,
    };

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().await, Some(expected_body));
}

#[rocket::async_test]
async fn test_index() {
    let client = Client::new(rocket()).await.unwrap();

    // Render the template with an empty context.
    let mut context: HashMap<&str, &str> = HashMap::new();
    let template = Template::show(client.cargo(), "index", &context).unwrap();
    test_body(None, template).await;

    // Render the template with a context that contains the message.
    context.insert("message", "Hello from Rocket!");
    let template = Template::show(client.cargo(), "index", &context).unwrap();
    test_body(Some(Cookie::new("message", "Hello from Rocket!")), template).await;
}
