#[macro_use] extern crate rocket;

use rocket::http::{Cookie, CookieJar};

#[post("/")]
fn add(jar: &CookieJar<'_>) {
    jar.add(Cookie::new("name", "value"));
}

#[get("/")]
fn get<'a>(jar: &'a CookieJar<'_>) -> Option<&'a str> {
    jar.get("name").map(|c| c.value())
}

#[cfg(test)]
mod many_cookie_jars_tests {
    use super::*;
    use rocket::local::blocking::Client;
    use rocket::http::Status;

    fn rocket() -> rocket::Rocket {
        rocket::ignite().mount("/", routes![add, get])
    }

    #[test]
    fn test_tracked() {
        let client = Client::tracked(rocket()).unwrap();

        assert_eq!(client.get("/").dispatch().status(), Status::NotFound);
        assert_eq!(client.post("/").dispatch().status(), Status::Ok);

        let response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "value");
    }

    #[test]
    fn test_untracked() {
        let client = Client::untracked(rocket()).unwrap();

        assert_eq!(client.get("/").dispatch().status(), Status::NotFound);
        assert_eq!(client.post("/").dispatch().status(), Status::Ok);
        assert_eq!(client.get("/").dispatch().status(), Status::NotFound);
    }
}
