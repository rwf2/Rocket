use super::rocket;
use rocket::local::blocking::Client;
use rocket::http::Status;

macro_rules! run_test {
    ($query:expr, |$response:ident| $body:expr) => ({
        let client = Client::new(rocket()).unwrap();
        #[allow(unused_mut)]
        let mut $response = client.get(format!("/hello{}", $query)).dispatch();
        $body
    })
}

#[test]
fn age_and_name_params() {
    run_test!("?age=10&first-name=john", |response| {
        assert_eq!(response.into_string(),
            Some("Hello, 10 year old named john!".into()));
    });

    run_test!("?age=20&first-name=john", |response| {
        assert_eq!(response.into_string(),
            Some("20 years old? Hi, john!".into()));
    });
}

#[test]
fn age_param_only() {
    run_test!("?age=10", |response| {
        assert_eq!(response.into_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });

    run_test!("?age=20", |response| {
        assert_eq!(response.into_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[test]
fn name_param_only() {
    run_test!("?first-name=John", |response| {
        assert_eq!(response.into_string(), Some("Hello John!".into()));
    });
}

#[test]
fn no_params() {
    run_test!("", |response| {
        assert_eq!(response.into_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });

    run_test!("?", |response| {
        assert_eq!(response.into_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[test]
fn extra_params() {
    run_test!("?age=20&first-name=Bob&extra", |response| {
        assert_eq!(response.into_string(),
            Some("20 years old? Hi, Bob!".into()));
    });

    run_test!("?age=30&first-name=Bob&extra", |response| {
        assert_eq!(response.into_string(),
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[test]
fn wrong_path() {
    run_test!("/other?age=20&first-name=Bob", |response| {
        assert_eq!(response.status(), Status::NotFound);
    });
}
