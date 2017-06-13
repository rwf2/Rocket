use rocket::local::Client;
use rocket::http::*;

fn setup() -> Client {
    use super::*;
    let server = rocket::ignite().mount("/", routes![
            index,
            user_index,
            login,
            logout,
            login_user,
            login_page]);
    Client::new(server).unwrap()
}

#[test]
fn test_login() {
    let client = setup();
    let response = client
        .post("/login")
        .header(ContentType::Form)
        .body("username=Sergio&password=password")
        .dispatch();
    let cookie: Cookie = response
        .headers()
        .iter()
        .filter(|h| h.name == "Set-Cookie")
        .filter(|h| h.value.contains("user_id"))
        .map(|h| h.value)
        .nth(0)
        .unwrap()
        .parse()
        .unwrap();

    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(cookie.name(), "user_id");
    assert_eq!(cookie.path().unwrap(), "/");
    assert!(cookie.value().len() > 25);
    assert!(cookie.http_only());
    assert_eq!(cookie.same_site().unwrap().to_string(), "Strict");
}

#[test]
fn test_logout() {
    let client = setup();

    let response = client
        .post("/login")
        .header(ContentType::Form)
        .body("username=Sergio&password=password")
        .dispatch();

    let user_id: String = response
        .headers()
        .iter()
        .filter(|h| h.name == "Set-Cookie")
        .filter(|h| h.value.contains("user_id"))
        .map(|h| h.value)
        .nth(0)
        .unwrap()
        .parse::<Cookie>()
        .unwrap()
        .value()
        .into();

    let response = client
        .post("/logout")
        .header(ContentType::Form)
        .cookie(Cookie::new("user_id", user_id))
        .dispatch();

    let cookie: Cookie = response
        .headers()
        .iter()
        .filter(|h| h.name == "Set-Cookie")
        .filter(|h| h.value.contains("user_id"))
        .map(|h| h.value)
        .nth(0)
        .unwrap()
        .parse()
        .unwrap();

    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(cookie.name(), "user_id");
    assert_eq!(cookie.value(), "");
    assert_eq!(cookie.max_age().unwrap().num_seconds(), 0);
}
