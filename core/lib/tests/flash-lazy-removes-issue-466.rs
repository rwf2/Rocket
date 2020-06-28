#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::request::FlashMessage;
use rocket::response::Flash;

const FLASH_MESSAGE: &str = "Hey! I'm a flash message. :)";

#[post("/")]
fn set() -> Flash<&'static str> {
    Flash::success("This is the page.", FLASH_MESSAGE)
}

#[get("/unused")]
fn unused(flash: Option<FlashMessage<'_, '_>>) -> Option<()> {
    flash.map(|_| ())
}

#[get("/use")]
fn used(flash: Option<FlashMessage<'_, '_>>) -> Option<String> {
    flash.map(|flash| flash.msg().into())
}

mod flash_lazy_remove_tests {
    use rocket::local::asynchronous::Client;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn test() {
        use super::*;
        let r = rocket::ignite().mount("/", routes![set, unused, used]);
        let client = Client::new(r).await.unwrap();

        // Ensure the cookie's not there at first.
        let response = client.get("/unused").dispatch().await;
        assert_eq!(response.status(), Status::NotFound);

        // Set the flash cookie.
        client.post("/").dispatch().await;

        // Try once.
        let response = client.get("/unused").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        // Try again; should still be there.
        let response = client.get("/unused").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        // Now use it.
        let response = client.get("/use").dispatch().await;
        assert_eq!(response.into_string().await, Some(FLASH_MESSAGE.into()));

        // Now it should be gone.
        let response = client.get("/unused").dispatch().await;
        assert_eq!(response.status(), Status::NotFound);

        // Still gone.
        let response = client.get("/use").dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }
}
