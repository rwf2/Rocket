use super::rocket;

use rocket::http::{RawStr, Status, Method::*};
use rocket::local::blocking::Client;
use rocket_dyn_templates::{Template, context};

fn test_root(kind: &str) {
    // Check that the redirect works.
    let client = Client::tracked(rocket()).unwrap();
    for method in &[Get, Head] {
        let response = client.req(*method, format!("/{}", kind)).dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert!(response.body().is_none());

        let location = response.headers().get_one("Location").unwrap();
        assert_eq!(location, format!("/{}/hello/Your%20Name", kind));
    }

    // Check that other request methods are not accepted (and instead caught).
    for method in &[Post, Put, Delete, Options, Trace, Connect, Patch] {
        let context = context! { uri: format!("/{}", kind) };
        let expected = Template::show(client.rocket(), format!("{}/error/404", kind), &context);

        let response = client.req(*method, format!("/{}", kind)).dispatch();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.into_string(), expected);
    }
}

fn test_name(base: &str) {
    // Check that the /hello/<name> route works.
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get(format!("/{}/hello/Jack%20Daniels", base)).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(response.into_string().unwrap().contains("Hi Jack Daniels!"));
}

fn test_404(base: &str) {
    // Check that the error catcher works.
    let client = Client::tracked(rocket()).unwrap();
    for bad_path in &["/hello", "/foo/bar", "/404"] {
        let path = format!("/{}{}", base, bad_path);
        let escaped_path = RawStr::new(&path).html_escape();

        let response = client.get(&path).dispatch();
        assert_eq!(response.status(), Status::NotFound);
        let response = response.into_string().unwrap();

        assert!(response.contains(base));
        assert! {
            response.contains(&format!("{} does not exist", path))
                || response.contains(&format!("{} does not exist", escaped_path))
        };
    }
}

fn test_about(base: &str) {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get(format!("/{}/about", base)).dispatch();
    assert!(response.into_string().unwrap().contains("About - Here's another page!"));
}

#[test]
fn test_index() {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get("/").dispatch().into_string().unwrap();
    assert!(response.contains("Tera"));
    assert!(response.contains("Handlebars"));
}

#[test]
fn hbs() {
    test_root("hbs");
    test_name("hbs");
    test_404("hbs");
    test_about("hbs");
}

#[test]
fn tera() {
    test_root("tera");
    test_name("tera");
    test_404("tera");
    test_about("tera");
}
