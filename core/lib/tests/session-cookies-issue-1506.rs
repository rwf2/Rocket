#![cfg(feature = "secrets")]

use rocket::http::{CookieJar, Cookie};

#[rocket::get("/")]
fn index(jar: &CookieJar<'_>) {
    jar.add_private(Cookie::build(("key", "value")).expires(None));
}

mod test_session_cookies {
    use super::*;
    use rocket::local::blocking::Client;

    #[test]
    fn session_cookie_is_session() {
        let rocket = rocket::build().mount("/", rocket::routes![index]);
        let client = Client::debug(rocket).unwrap();

        let response = client.get("/").dispatch();
        let cookie = response.cookies().get_private("key").unwrap();
        assert_eq!(cookie.expires_datetime(), None);
    }
}
