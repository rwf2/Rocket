#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::request::Request;
use rocket::http::{Cookie, Cookies};

#[catch(404)]
fn not_found(request: &Request) -> &'static str {
    request.cookies().add(Cookie::new("not_found", "hi"));
    "404 - Not Found"
}

#[get("/")]
fn index(mut cookies: Cookies) -> &'static str {
    cookies.add(Cookie::new("index", "hi"));
    "Hello, world!"
}

mod tests {
    use super::*;
    use rocket::local::Client;
    use rocket::fairing::AdHoc;

    #[test]
    fn error_catcher_sets_cookies() {
        let rocket = rocket::ignite()
            .mount("/", routes![index])
            .register(catchers![not_found])
            .attach(AdHoc::on_request("Add Fairing Cookie", |req, _| {
                req.cookies().add(Cookie::new("fairing", "hi"));
            }));

        let client = Client::new(rocket).unwrap();

        // Check that the index returns the `index` and `fairing` cookie.
        let response = client.get("/").dispatch();
        let cookies = response.cookies();
        assert_eq!(cookies.len(), 2);
        assert!(cookies.iter().find(|c| c.name() == "index").is_some());
        assert!(cookies.iter().find(|c| c.name() == "fairing").is_some());

        // Check that the catcher returns only the `not_found` cookie.
        let response = client.get("/not-existent").dispatch();
        assert_eq!(response.cookies(), vec![Cookie::new("not_found", "hi")]);
    }
}
