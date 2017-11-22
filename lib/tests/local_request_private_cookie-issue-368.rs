#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::http::{Cookie, Cookies};

#[get("/")]
fn return_private_cookie(mut cookies: Cookies) -> String {
    if let Some(ref cookie) = cookies.get_private("hello") {
        return String::from(cookie.value());
    }

    String::from("")
}

mod tests {
    use super::*;

    use rocket::local::Client;

    #[test]
    fn private_cookie_works() {
        let rocket = rocket::ignite().mount("/", routes![return_private_cookie]);
        let client = Client::new(rocket).unwrap();
        let cookie = Cookie::new("hello", "world");
        let req = client.get("/").private_cookie(cookie);
        let mut response = req.dispatch();
        assert_eq!(response.body_string(), Some("world".into()));
    }
}
