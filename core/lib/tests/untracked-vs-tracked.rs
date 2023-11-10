#[macro_use] extern crate rocket;

use rocket::http::CookieJar;

#[post("/")]
fn add(jar: &CookieJar<'_>) {
    jar.add(("name", "value"));
}

#[get("/")]
fn get<'a>(jar: &'a CookieJar<'_>) -> Option<&'a str> {
    jar.get("name").map(|c| c.value())
}

#[cfg(test)]
mod many_cookie_jars_tests {
    use super::*;
    use rocket::{Rocket, local::blocking::Client, Build};
    use rocket::http::Status;

    fn rocket() -> Rocket<Build> {
        rocket::custom(rocket::Config::debug_default())
            .mount("/", routes![add, get])
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
