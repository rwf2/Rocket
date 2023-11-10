#![cfg(feature = "secrets")]

use rocket::http::CookieJar;

#[rocket::get("/")]
fn return_private_cookie(cookies: &CookieJar<'_>) -> Option<String> {
    match cookies.get_private("cookie_name") {
        Some(cookie) => Some(cookie.value().into()),
        None => None,
    }
}

mod tests {
    use super::*;
    use rocket::routes;
    use rocket::local::blocking::Client;
    use rocket::http::Status;

    #[test]
    fn private_cookie_is_returned() {
        let rocket = rocket::build().mount("/", routes![return_private_cookie]);

        let client = Client::debug(rocket).unwrap();
        let req = client.get("/").private_cookie(("cookie_name", "cookie_value"));
        let response = req.dispatch();

        assert_eq!(response.headers().get_one("Set-Cookie"), None);
        assert_eq!(response.into_string(), Some("cookie_value".into()));
    }

    #[test]
    fn regular_cookie_is_not_returned() {
        let rocket = rocket::build().mount("/", routes![return_private_cookie]);

        let client = Client::debug(rocket).unwrap();
        let req = client.get("/").cookie(("cookie_name", "cookie_value"));
        let response = req.dispatch();

        assert_eq!(response.status(), Status::NotFound);
    }
}
