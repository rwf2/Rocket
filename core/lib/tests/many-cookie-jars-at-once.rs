#[macro_use] extern crate rocket;

use rocket::http::CookieJar;

#[post("/")]
fn multi_add(jar_a: &CookieJar<'_>, jar_b: &CookieJar<'_>) {
    jar_a.add(("a", "v1"));
    jar_b.add(("b", "v2"));
}

#[get("/")]
fn multi_get(jar_a: &CookieJar<'_>, jar_b: &CookieJar<'_>, jar_c: &CookieJar<'_>) -> String {
    let (a, a2, a3) = (jar_a.get("a"), jar_b.get("a"), jar_c.get("a"));
    let (b, b2, b3) = (jar_a.get("b"), jar_b.get("b"), jar_c.get("b"));
    assert_eq!(a, a2); assert_eq!(a2, a3);
    assert_eq!(b, b2); assert_eq!(b2, b3);
    format!("{}{}", a.unwrap().value(), b.unwrap().value())
}

#[cfg(test)]
mod many_cookie_jars_tests {
    use super::*;
    use rocket::{Rocket, Build};
    use rocket::local::blocking::Client;

    fn rocket() -> Rocket<Build> {
        rocket::build().mount("/", routes![multi_add, multi_get])
    }

    #[test]
    fn test_multi_add() {
        let client = Client::debug(rocket()).unwrap();
        let response = client.post("/").dispatch();
        let cookies = response.cookies();
        assert_eq!(cookies.iter().count(), 2);
        assert_eq!(cookies.get("a").unwrap().value(), "v1");
        assert_eq!(cookies.get("b").unwrap().value(), "v2");
    }

    #[test]
    fn test_multi_get() {
        let client = Client::debug(rocket()).unwrap();
        let response = client.get("/")
            .cookie(("a", "a_val"))
            .cookie(("b", "hi!"))
            .dispatch();

        assert_eq!(response.into_string().unwrap(), "a_valhi!");
    }
}
