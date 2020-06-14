use rocket::local::Client;
use rocket::http::Status;

async fn client() -> Client {
    let rocket = rocket::ignite().mount("/", routes![super::root, super::login]);
    Client::new(rocket).await.unwrap()
}

#[rocket::async_test]
async fn test_root() {
    let client = client().await;
    let mut response = client.get("/").dispatch().await;

    assert!(response.body().is_none());
    assert_eq!(response.status(), Status::SeeOther);
    for h in response.headers().iter() {
        match h.name.as_str() {
            "Location" => assert_eq!(h.value, "/login"),
            "Content-Length" => assert_eq!(h.value.parse::<i32>().unwrap(), 0),
            _ => { /* let these through */ }
        }
    }
}

#[rocket::async_test]
async fn test_login() {
    let client = client().await;
    let mut r = client.get("/login").dispatch().await;
    assert_eq!(r.body_string().await, Some("Hi! Please log in before continuing.".into()));
}
