#[macro_use] extern crate rocket;

use rocket::response::Redirect;

#[get("/http")]
fn http() -> Redirect {
    Redirect::to(uri!("http://rocket.rs"))
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
        let client = Client::debug_with(routes![http, redirect]).unwrap();

        let response = client.get(uri!(http)).dispatch();
        let location = response.headers().get_one("Location");
        assert_eq!(location, Some("http://rocket.rs"));

        let response = client.get(uri!(redirect)).dispatch();
        let location = response.headers().get_one("Location");
        assert_eq!(location, Some("https://rocket.rs:80"));
    }
}
