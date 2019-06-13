use super::rocket;
use rocket::local::{Client, LocalResponse as Response};
use rocket::http::Status;

macro_rules! run_test {
    ($query:expr, $test_fn:expr) => ({
        let client = Client::new(rocket()).unwrap();
        $test_fn(client.get(format!("/hello{}", $query)).dispatch());
    })
}

#[test]
fn age_and_name_params() {
    run_test!("?age=10&name=john", |mut response: Response<'_>| {
        assert_eq!(response.body_string(),
            Some("Hello, 10 year old named john!".into()));
    });

    run_test!("?age=20&name=john", |mut response: Response<'_>| {
        assert_eq!(response.body_string(),
            Some("20 years old? Hi, john!".into()));
    });
}

#[test]
fn age_param_only() {
    run_test!("?age=10", |mut response: Response<'_>| {
        assert_eq!(response.body_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });

    run_test!("?age=20", |mut response: Response<'_>| {
        assert_eq!(response.body_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[test]
fn name_param_only() {
    run_test!("?name=John", |mut response: Response<'_>| {
        assert_eq!(response.body_string(), Some("Hello John!".into()));
    });
}

#[test]
fn no_params() {
    run_test!("", |mut response: Response<'_>| {
        assert_eq!(response.body_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });

    run_test!("?", |mut response: Response<'_>| {
        assert_eq!(response.body_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[test]
fn extra_params() {
    run_test!("?age=20&name=Bob&extra", |mut response: Response<'_>| {
        assert_eq!(response.body_string(),
            Some("20 years old? Hi, Bob!".into()));
    });

    run_test!("?age=30&name=Bob&extra", |mut response: Response<'_>| {
        assert_eq!(response.body_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[test]
fn wrong_path() {
    run_test!("/other?age=20&name=Bob", |response: Response<'_>| {
        assert_eq!(response.status(), Status::NotFound);
    });
}
