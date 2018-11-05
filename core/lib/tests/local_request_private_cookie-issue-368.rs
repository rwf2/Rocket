#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::http::Cookies;

#[cfg(feature = "private-cookies")]
#[get("/")]
fn return_private_cookie(mut cookies: Cookies) -> Option<String> {
    match cookies.get_private("cookie_name") {
        Some(cookie) => Some(cookie.value().into()),
        None => None,
    }
}

#[cfg(feature = "private-cookies")]
mod tests {
    use super::*;
    use rocket::local::Client;
    use rocket::http::Cookie;
    use rocket::http::Status;

    #[test]
    fn private_cookie_is_returned() {
        let rocket = rocket::ignite().mount("/", routes![return_private_cookie]);

        let client = Client::new(rocket).unwrap();
        let req = client.get("/").private_cookie(Cookie::new("cookie_name", "cookie_value"));
        let mut response = req.dispatch();

        assert_eq!(response.body_string(), Some("cookie_value".into()));
        assert_eq!(response.headers().get_one("Set-Cookie"), None);
    }

    #[test]
    fn regular_cookie_is_not_returned() {
        let rocket = rocket::ignite().mount("/", routes![return_private_cookie]);

        let client = Client::new(rocket).unwrap();
        let req = client.get("/").cookie(Cookie::new("cookie_name", "cookie_value"));
        let response = req.dispatch();

        assert_eq!(response.status(), Status::NotFound);
    }
}
