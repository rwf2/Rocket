#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::response::Redirect;

#[get("/google")]
fn google() -> Redirect {
    Redirect::to("https://www.google.com")
}

#[get("/rocket")]
fn rocket() -> Redirect {
    Redirect::to("https://rocket.rs:80")
}

mod test_absolute_uris_okay {
    use super::*;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    async fn redirect_works() {
        let rocket = rocket::ignite().mount("/", routes![google, rocket]);
        let client = Client::new(rocket).await.unwrap();

        let response = client.get("/google").dispatch().await;
        let location = response.headers().get_one("Location");
        assert_eq!(location, Some("https://www.google.com"));

        let response = client.get("/rocket").dispatch().await;
        let location = response.headers().get_one("Location");
        assert_eq!(location, Some("https://rocket.rs:80"));
    }
}
