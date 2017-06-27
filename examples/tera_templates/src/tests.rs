use super::rocket;
use rocket::local::{Client, LocalResponse};
use rocket::http::Method::*;
use rocket::http::Status;
use rocket_contrib::Template;

const TEMPLATE_ROOT: &'static str = "templates/";

macro_rules! dispatch {
    ($method:expr, $path:expr, $test_fn:expr) => ({
        let client = Client::new(rocket()).unwrap();
        $test_fn(client.req($method, $path).dispatch());
    })
}

#[test]
fn test_root() {
    // Check that the redirect works.
    for method in &[Get, Head] {
        dispatch!(*method, "/", |mut response: LocalResponse| {
            assert_eq!(response.status(), Status::SeeOther);
            assert!(response.body().is_none());

            let location: Vec<_> = response.headers().get("Location").collect();
            assert_eq!(location, vec!["/hello/Unknown"]);
        });
    }

    // Check that other request methods are not accepted (and instead caught).
    for method in &[Post, Put, Delete, Options, Trace, Connect, Patch] {
        dispatch!(*method, "/", |mut response: LocalResponse| {
            let mut map = ::std::collections::HashMap::new();
            map.insert("path", "/");
            let expected = Template::show(TEMPLATE_ROOT, "error/404", &map).unwrap();

            assert_eq!(response.status(), Status::NotFound);
            assert_eq!(response.body_string(), Some(expected));
        });
    }
}

#[test]
fn test_name() {
    // Check that the /hello/<name> route works.
    dispatch!(Get, "/hello/Jack", |mut response: LocalResponse| {
        let context = super::TemplateContext {
            name: "Jack".to_string(),
            items: vec!["One", "Two", "Three"].iter().map(|s| s.to_string()).collect()
        };

        let expected = Template::show(TEMPLATE_ROOT, "index", &context).unwrap();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some(expected));
    });
}

#[test]
fn test_404() {
    // Check that the error catcher works.
    dispatch!(Get, "/hello/", |mut response: LocalResponse| {
        let mut map = ::std::collections::HashMap::new();
        map.insert("path", "/hello/");

        let expected = Template::show(TEMPLATE_ROOT, "error/404", &map).unwrap();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.body_string(), Some(expected));
    });
}
