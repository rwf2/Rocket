#![feature(proc_macro_hygiene)]

#[macro_use]
#[cfg(feature = "private-cookies")]
extern crate rocket;

#[cfg(feature = "private-cookies")]
mod private_cookie_test {
    use rocket::http::Cookies;

    #[get("/")]
    fn return_private_cookie(mut cookies: Cookies) -> Option<String> {
        match cookies.get_private("cookie_name") {
            Some(cookie) => Some(cookie.value().into()),
            None => None,
        }
    }

    mod tests {
        use super::*;
        use rocket::local::Client;
        use rocket::http::Cookie;
        use rocket::http::Status;

        #[rocket::async_test]
        async fn private_cookie_is_returned() {
            let rocket = rocket::ignite().mount("/", routes![return_private_cookie]);

            let client = Client::new(rocket).await.unwrap();
            let req = client.get("/").private_cookie(Cookie::new("cookie_name", "cookie_value"));
            let mut response = req.dispatch().await;

            assert_eq!(response.body_string().await, Some("cookie_value".into()));
            assert_eq!(response.headers().get_one("Set-Cookie"), None);
        }

        #[rocket::async_test]
        async fn regular_cookie_is_not_returned() {
            let rocket = rocket::ignite().mount("/", routes![return_private_cookie]);

            let client = Client::new(rocket).await.unwrap();
            let req = client.get("/").cookie(Cookie::new("cookie_name", "cookie_value"));
            let response = req.dispatch().await;

            assert_eq!(response.status(), Status::NotFound);
        }
    }
}
