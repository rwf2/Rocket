use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::Status;
use rocket::Response;

macro_rules! run_test {
    ($query:expr, $test_fn:expr) => ({
        let rocket = rocket::ignite()
            .mount("/", routes![super::hello]);

        let mut request = MockRequest::new(Get, format!("/hello{}", $query));
        $test_fn(request.dispatch_with(&rocket));
    })
}

#[test]
fn age_and_name_params() {
  run_test!("?age=10&name=john", |mut response: Response| {
    let body_str = response.body().and_then(|body| body.into_string());
    assert_eq!(body_str, Some("Hello, 10 year old named john!".to_string()));
  });
}

#[test]
fn age_param_only() {
  run_test!("?age=10", |response: Response| {
    assert_eq!(response.status(), Status::NotFound);
  });
}

#[test]
fn name_param_only() {
  run_test!("?name=John", |mut response: Response| {
    let body_str = response.body().and_then(|body| body.into_string());
    assert_eq!(body_str, Some("Hello John!".to_string()));
  });
}

#[test]
fn no_params() {
  run_test!("", |response: Response| {
    assert_eq!(response.status(), Status::NotFound);
  });

  run_test!("?", |response: Response| {
    assert_eq!(response.status(), Status::NotFound);
  });
}

#[test]
fn non_existent_params() {
  run_test!("?x=y", |response: Response| {
    assert_eq!(response.status(), Status::NotFound);
  });

  run_test!("?age=10&name=john&complete=true", |response: Response| {
    assert_eq!(response.status(), Status::NotFound);
  });
}
