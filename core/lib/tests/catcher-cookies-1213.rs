#[macro_use] extern crate rocket;

use rocket::request::Request;
use rocket::http::CookieJar;

#[catch(404)]
fn not_found(request: &Request) -> &'static str {
    request.cookies().add(("not_found", "404"));
    "404 - Not Found"
}

#[get("/")]
fn index(cookies: &CookieJar<'_>) -> &'static str {
    cookies.add(("index", "hi"));
    "Hello, world!"
}

mod tests {
    use super::*;
    use rocket::local::blocking::Client;
    use rocket::fairing::AdHoc;

    #[test]
    fn error_catcher_sets_cookies() {
        let rocket = rocket::build()
            .mount("/", routes![index])
            .register("/", catchers![not_found])
            .attach(AdHoc::on_request("Add Cookie", |req, _| Box::pin(async move {
                req.cookies().add(("fairing", "woo"));
            })));

        let client = Client::debug(rocket).unwrap();

        // Check that the index returns the `index` and `fairing` cookie.
        let response = client.get("/").dispatch();
        let cookies = response.cookies();
        assert_eq!(cookies.iter().count(), 2);
        assert_eq!(cookies.get("index").unwrap().value(), "hi");
        assert_eq!(cookies.get("fairing").unwrap().value(), "woo");

        // Check that the catcher returns only the `not_found` cookie.
        let response = client.get("/not-existent").dispatch();
        let cookies = response.cookies();
        assert_eq!(cookies.iter().count(), 1);
        assert_eq!(cookies.get("not_found").unwrap().value(), "404");
    }
}
