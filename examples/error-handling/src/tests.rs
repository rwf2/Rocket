use rocket::local::blocking::Client;
use rocket::http::Status;

#[test]
fn test_hello() {
    let client = Client::tracked(super::rocket()).unwrap();

    let (name, age) = ("Arthur", 42);
    let uri = format!("/hello/{}/{}", name, age);
    let response = client.get(uri).dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), super::hello(name.into(), age));
}

#[test]
fn forced_error() {
    let client = Client::tracked(super::rocket()).unwrap();

    let request = client.get("/404");
    let expected = super::general_not_found();
    let response = request.dispatch();
    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.into_string().unwrap(), expected.0);

    let request = client.get("/405");
    let expected = super::default_catcher(Status::MethodNotAllowed, request.inner());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::MethodNotAllowed);
    assert_eq!(response.into_string().unwrap(), expected.1);

    let request = client.get("/533");
    let expected = super::default_catcher(Status::new(533), request.inner());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::new(533));
    assert_eq!(response.into_string().unwrap(), expected.1);

    let request = client.get("/700");
    let expected = super::default_catcher(Status::InternalServerError, request.inner());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.into_string().unwrap(), expected.1);
}

#[test]
fn test_hello_invalid_age() {
    let client = Client::tracked(super::rocket()).unwrap();

    for path in &["Ford/-129", "Trillian/128", "foo/bar/baz"] {
        let request = client.get(format!("/hello/{}", path));
        let expected = super::hello_not_found(request.inner());
        let response = request.dispatch();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.into_string().unwrap(), expected.0);
    }
}

#[test]
fn test_hello_sergio() {
    let client = Client::tracked(super::rocket()).unwrap();

    for path in &["oops", "-129", "foo/bar", "/foo/bar/baz"] {
        let request = client.get(format!("/hello/Sergio/{}", path));
        let expected = super::sergio_error();
        let response = request.dispatch();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.into_string().unwrap(), expected);
    }
}
