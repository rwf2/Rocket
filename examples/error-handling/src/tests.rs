use rocket::local::blocking::Client;
use rocket::http::Status;
use rocket::serde::json::to_string as json_string;

#[test]
fn test_hello() {
    let client = Client::tracked(super::rocket()).unwrap();

    let (name, age) = ("Arthur", 42);
    let uri = format!("/hello/{}/{}", name, age);
    let response = client.get(uri).dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), super::hello(name, age));
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
    let expected = super::default_catcher(Status::MethodNotAllowed, request.uri());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::MethodNotAllowed);
    assert_eq!(response.into_string().unwrap(), expected.1);

    let request = client.get("/533");
    let expected = super::default_catcher(Status::new(533), request.uri());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::new(533));
    assert_eq!(response.into_string().unwrap(), expected.1);

    let request = client.get("/700");
    let expected = super::default_catcher(Status::InternalServerError, request.uri());
    let response = request.dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.into_string().unwrap(), expected.1);
}

#[test]
fn test_hello_invalid_age() {
    let client = Client::tracked(super::rocket()).unwrap();

    for path in &["Ford/-129", "Trillian/128"] {
        let request = client.get(format!("/hello/{}", path));
        let expected = super::ErrorInfo {
            invalid_value: path.split_once("/").unwrap().1,
            description: format!(
                "{}",
                path.split_once("/").unwrap().1.parse::<i8>().unwrap_err()
            ),
        };
        let response = request.dispatch();
        assert_eq!(response.status(), Status::UnprocessableEntity);
        assert_eq!(response.into_string().unwrap(), json_string(&expected).unwrap());
    }

    {
        let path = &"foo/bar/baz";
        let request = client.get(format!("/hello/{}", path));
        let expected = super::hello_not_found(request.uri());
        let response = request.dispatch();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.into_string().unwrap(), expected.0);
    }
}

#[test]
fn test_hello_sergio() {
    let client = Client::tracked(super::rocket()).unwrap();

    // TODO: typed: This logic has changed, either needs to be fixed
    // or this test changed.
    for path in &["oops", "-129"] {
        let request = client.get(format!("/hello/Sergio/{}", path));
        let expected = super::sergio_error();
        let response = request.dispatch();
        assert_eq!(response.status(), Status::UnprocessableEntity);
        assert_eq!(response.into_string().unwrap(), expected);
    }

    for path in &["foo/bar", "/foo/bar/baz"] {
        let request = client.get(format!("/hello/Sergio/{}", path));
        let expected = super::sergio_error();
        let response = request.dispatch();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.into_string().unwrap(), expected);
    }
}
