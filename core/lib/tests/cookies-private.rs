#![cfg(feature = "secrets")]
#![deny(warnings)]

use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::{get, post, routes};

#[post("/")]
fn cookie_add_private(jar: &CookieJar<'_>) {
    let mut cookie_a = Cookie::new("a", "v1");
    jar.add(cookie_a.clone());
    let mut cookie_b = Cookie::new("b", "v2");
    jar.add_private(cookie_b.clone());
    jar.add(Cookie::new("c", "v3"));

    // private: CookieJar::set_defaults(&mut cookie_a);
    cookie_a.set_path("/");
    cookie_a.set_same_site(SameSite::Strict);
    assert_eq!(jar.get_pending(cookie_a.name()), Some(cookie_a));

    // private: CookieJar::set_private_defaults(&mut cookie_b);
    cookie_b.set_path("/");
    cookie_b.set_same_site(SameSite::Strict);
    cookie_b.set_http_only(true);
    let expires = time::OffsetDateTime::now_utc() + time::Duration::weeks(1);
    cookie_b.set_expires(expires);
    let mut cookie_b_pending = jar
        .get_pending(cookie_b.name())
        .expect("cookie_b_pending None");
    cookie_b_pending.set_expires(expires);
    assert_eq!(cookie_b_pending, cookie_b);
}

#[get("/")]
fn cookie_get_private(jar: &CookieJar<'_>) -> String {
    let (a, b, c) = (jar.get("a"), jar.get_private("b"), jar.get("c"));
    assert_ne!(a, b.as_ref());
    assert_ne!(a, c);
    assert_ne!(b.as_ref(), c);

    format!(
        "{}{}{}",
        a.unwrap().value(),
        b.unwrap().value(),
        c.unwrap().value()
    )
}

/// For test if we got really a private cookie
#[get("/oh-no")]
fn cookie_get(jar: &CookieJar<'_>) -> String {
    let (a, b, c) = (jar.get("a"), jar.get("b"), jar.get("c"));

    format!(
        "{}{}{}",
        a.unwrap().value(),
        b.unwrap().value(),
        c.unwrap().value()
    )
}

#[cfg(test)]
mod cookies_private_tests {
    use super::*;
    use rocket::local::blocking::Client;
    use rocket::{Build, Rocket};

    fn rocket() -> Rocket<Build> {
        rocket::build().mount(
            "/",
            routes![cookie_add_private, cookie_get, cookie_get_private],
        )
    }

    #[test]
    fn test_cookie_add_private() {
        let client = Client::debug(rocket()).unwrap();
        let response = client.post("/").dispatch();
        let cookies = response.cookies();
        assert_eq!(cookies.iter().count(), 3);
        assert_eq!(cookies.get("a").unwrap().value(), "v1");
        assert_eq!(cookies.get_private("b").unwrap().value(), "v2");
        assert_ne!(cookies.get("b").unwrap().value(), "v2");
        assert_eq!(cookies.get("c").unwrap().value(), "v3");
    }

    #[test]
    fn test_cookie_get_private() {
        let client = Client::debug(rocket()).unwrap();
        let response = client
            .get("/")
            .cookie(Cookie::new("a", "Cookie"))
            .private_cookie(Cookie::new("b", " tastes "))
            .cookie(Cookie::new("c", "good!"))
            .dispatch();

        assert_eq!(response.into_string().unwrap(), "Cookie tastes good!");
    }

    /// Test if we got really a private cookie
    #[test]
    fn test_cookie_get_ohno() {
        let client = Client::debug(rocket()).unwrap();
        let response = client
            .get("/oh-no")
            .cookie(Cookie::new("a", "Cookie"))
            .private_cookie(Cookie::new("b", " tastes "))
            .cookie(Cookie::new("c", "good!"))
            .dispatch();

        assert_ne!(response.into_string().unwrap(), "Cookie tastes good!");
    }
}
