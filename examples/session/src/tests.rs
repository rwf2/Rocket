use super::rocket;
use rocket::local::asynchronous::{Client, LocalResponse};
use rocket::http::{Status, Cookie, ContentType};

fn user_id_cookie(response: &LocalResponse<'_>) -> Option<Cookie<'static>> {
    let cookie = response.headers()
        .get("Set-Cookie")
        .filter(|v| v.starts_with("user_id"))
        .nth(0)
        .and_then(|val| Cookie::parse_encoded(val).ok());

    cookie.map(|c| c.into_owned())
}

async fn login(client: &Client, user: &str, pass: &str) -> Option<Cookie<'static>> {
    let response = client.post("/login")
        .header(ContentType::Form)
        .body(format!("username={}&password={}", user, pass))
        .dispatch().await;

    user_id_cookie(&response)
}

#[rocket::async_test]
async fn redirect_on_index() {
    let client = Client::new(rocket()).await.unwrap();
    let response = client.get("/").dispatch().await;
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/login"));
}

#[rocket::async_test]
async fn can_login() {
    let client = Client::new(rocket()).await.unwrap();

    let response = client.get("/login").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let body = response.into_string().await.unwrap();
    assert!(body.contains("Please login to continue."));
}

#[rocket::async_test]
async fn login_fails() {
    let client = Client::new(rocket()).await.unwrap();
    assert!(login(&client, "Seergio", "password").await.is_none());
    assert!(login(&client, "Sergio", "idontknow").await.is_none());
}

#[rocket::async_test]
async fn login_logout_succeeds() {
    let client = Client::new(rocket()).await.unwrap();
    let login_cookie = login(&client, "Sergio", "password").await.expect("logged in");

    // Ensure we're logged in.
    let response = client.get("/").cookie(login_cookie.clone()).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let body = response.into_string().await.unwrap();
    assert!(body.contains("Logged in with user ID 1"));

    // One more.
    let response = client.get("/login").cookie(login_cookie.clone()).dispatch().await;
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/"));

    // Logout.
    let response = client.post("/logout").cookie(login_cookie).dispatch().await;
    let cookie = user_id_cookie(&response).expect("logout cookie");
    assert!(cookie.value().is_empty());

    // The user should be redirected back to the login page.
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/login"));

    // The page should show the success message, and no errors.
    let response = client.get("/login").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let body = response.into_string().await.unwrap();
    assert!(body.contains("Successfully logged out."));
    assert!(!body.contains("Error"));
}
