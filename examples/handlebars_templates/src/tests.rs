use super::rocket;
use rocket::local::{Client, LocalResponse};
use rocket::http::Method::*;
use rocket::http::Status;
use rocket_contrib::Template;

macro_rules! dispatch {
    ($method:expr, $path:expr, $test_fn:expr) => ({
        let client = Client::new(rocket()).unwrap();
        $test_fn(&client, client.req($method, $path).dispatch());
    })
}

#[test]
fn test_root() {
    // Check that the redirect works.
    for method in &[Get, Head] {
        dispatch!(*method, "/", |_: &Client, mut response: LocalResponse| {
            assert_eq!(response.status(), Status::SeeOther);
            assert!(response.body().is_none());

            let location: Vec<_> = response.headers().get("Location").collect();
            assert_eq!(location, vec!["/hello/Unknown"]);
        });
    }

    // Check that other request methods are not accepted (and instead caught).
    for method in &[Post, Put, Delete, Options, Trace, Connect, Patch] {
        dispatch!(*method, "/", |client: &Client, mut response: LocalResponse| {
            let mut map = ::std::collections::HashMap::new();
            map.insert("path", "/");
            let expected = Template::show(client.rocket(), "error/404", &map).unwrap();

            assert_eq!(response.status(), Status::NotFound);
            assert_eq!(response.body_string(), Some(expected));
        });
    }
}

#[test]
fn test_name() {
    // Check that the /hello/<name> route works.
    dispatch!(Get, "/hello/Jack", |_client: &Client, mut response: LocalResponse| {
        let _context = super::TemplateContext {
            name: "Jack".into(),
            items: vec!["One".into(), "Two".into(), "Three".into()],
            title: "hello".to_string(),
            parent: "layout".to_string(),
        };

        assert_eq!(response.status(), Status::Ok);
        assert!(response.body_string().unwrap().contains("Jack"));
    });
}

#[test]
fn test_404() {
    // Check that the error catcher works.
    dispatch!(Get, "/hello/", |client: &Client, mut response: LocalResponse| {
        let mut map = ::std::collections::HashMap::new();
        map.insert("path", "/hello/");

        let expected = Template::show(client.rocket(), "error/404", &map).unwrap();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.body_string(), Some(expected));
    });
}
