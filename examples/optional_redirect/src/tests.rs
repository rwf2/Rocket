use rocket::local::Client;
use rocket::http::Status;

async fn client() -> Client {
    let rocket = rocket::ignite()
        .mount("/", routes![super::root, super::user, super::login]);
    Client::new(rocket).await.unwrap()

}

async fn test_200(uri: &str, expected_body: &str) {
    let client = client().await;
    let mut response = client.get(uri).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.body_string().await, Some(expected_body.to_string()));
}

async fn test_303(uri: &str, expected_location: &str) {
    let client = client().await;
    let response = client.get(uri).dispatch().await;
    let location_headers: Vec<_> = response.headers().get("Location").collect();
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(location_headers, vec![expected_location]);
}

#[rocket::async_test]
async fn test() {
    test_200("/users/Sergio", "Hello, Sergio!").await;
    test_200("/users/login",
             "Hi! That user doesn't exist. Maybe you need to log in?").await;
}

#[rocket::async_test]
async fn test_redirects() {
    test_303("/", "/users/login").await;
    test_303("/users/unknown", "/users/login").await;
}
