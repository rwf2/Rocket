use super::rocket;
use rocket::local::Client;
use rocket::http::Status;

macro_rules! run_test {
    ($query:expr, |$response:ident| $body:expr) => ({
        let client = Client::new(rocket()).await.unwrap();
        #[allow(unused_mut)]
        let mut $response = client.get(format!("/hello{}", $query)).dispatch().await;
        $body
    })
}

#[rocket::async_test]
async fn age_and_name_params() {
    run_test!("?age=10&name=john", |response| {
        assert_eq!(response.body_string().await,
            Some("Hello, 10 year old named john!".into()));
    });

    run_test!("?age=20&name=john", |response| {
        assert_eq!(response.body_string().await,
            Some("20 years old? Hi, john!".into()));
    });
}

#[rocket::async_test]
async fn age_param_only() {
    run_test!("?age=10", |response| {
        assert_eq!(response.body_string().await,
            Some("We're gonna need a name, and only a name.".into()));
    });

    run_test!("?age=20", |response| {
        assert_eq!(response.body_string().await,
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[rocket::async_test]
async fn name_param_only() {
    run_test!("?name=John", |response| {
        assert_eq!(response.body_string().await, Some("Hello John!".into()));
    });
}

#[rocket::async_test]
async fn no_params() {
    run_test!("", |response| {
        assert_eq!(response.body_string().await,
            Some("We're gonna need a name, and only a name.".into()));
    });

    run_test!("?", |response| {
        assert_eq!(response.body_string().await,
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[rocket::async_test]
async fn extra_params() {
    run_test!("?age=20&name=Bob&extra", |response| {
        assert_eq!(response.body_string().await,
            Some("20 years old? Hi, Bob!".into()));
    });

    run_test!("?age=30&name=Bob&extra", |response| {
        assert_eq!(response.body_string().await,
            Some("We're gonna need a name, and only a name.".into()));
    });
}

#[rocket::async_test]
async fn wrong_path() {
    run_test!("/other?age=20&name=Bob", |response| {
        assert_eq!(response.status(), Status::NotFound);
    });
}
