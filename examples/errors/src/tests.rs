use rocket::local::blocking::Client;
use rocket::http::Status;

#[test]
fn test_hello() {
    let client = Client::new(super::rocket()).unwrap();

    let (name, age) = ("Arthur", 42);
    let uri = format!("/hello/{}/{}", name, age);
    let response = client.get(uri).dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), super::hello(name.into(), age));
}

#[test]
fn forced_error_and_default_catcher() {
    let client = Client::new(super::rocket()).unwrap();

    let request = client.get("/404");
    let expected = super::not_found(request.inner());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.into_string().unwrap(), expected.0);

    let request = client.get("/405");
    let expected = super::default_catcher(Status::MethodNotAllowed, request.inner());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::MethodNotAllowed);
    assert_eq!(response.into_string().unwrap(), expected.1);

    let request = client.get("/533");
    let expected = super::default_catcher(Status::raw(533), request.inner());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::raw(533));
    assert_eq!(response.into_string().unwrap(), expected.1);

    let request = client.get("/700");
    let expected = super::default_catcher(Status::InternalServerError, request.inner());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.into_string().unwrap(), expected.1);
}

#[test]
fn test_hello_invalid_age() {
    let client = Client::new(super::rocket()).unwrap();

    for &(name, age) in &[("Ford", -129), ("Trillian", 128)] {
        let request = client.get(format!("/hello/{}/{}", name, age));
        let expected = super::not_found(request.inner());
        let response = request.dispatch();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.into_string().unwrap(), expected.0);
    }
}
