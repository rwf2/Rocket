#[macro_use] extern crate rocket;

use rocket::response::Redirect;

#[get("/google")]
fn google() -> Redirect {
    Redirect::to("https://www.google.com")
}

#[get("/rocket")]
fn redirect() -> Redirect {
    Redirect::to("https://rocket.rs:80")
}

mod test_absolute_uris_okay {
    use super::*;
    use rocket::local::blocking::Client;

    #[test]
    fn redirect_works() {
        let rocket = rocket::ignite().mount("/", routes![google, redirect]);
        let client = Client::tracked(rocket).unwrap();

        let response = client.get("/google").dispatch();
        let location = response.headers().get_one("Location");
        assert_eq!(location, Some("https://www.google.com"));

        let response = client.get("/rocket").dispatch();
        let location = response.headers().get_one("Location");
        assert_eq!(location, Some("https://rocket.rs:80"));
    }
}
