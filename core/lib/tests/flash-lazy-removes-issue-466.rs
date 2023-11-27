#[macro_use]
extern crate rocket;

use rocket::request::FlashMessage;
use rocket::response::Flash;

const FLASH_MESSAGE: &str = "Hey! I'm a flash message. :)";

#[post("/")]
fn set() -> Flash<&'static str> {
    Flash::success("This is the page.", FLASH_MESSAGE)
}

#[get("/unused")]
fn unused(flash: Option<FlashMessage<'_>>) -> Option<()> {
    flash.map(|_| ())
}

#[get("/use")]
fn used(flash: Option<FlashMessage<'_>>) -> Option<String> {
    flash.map(|f| f.message().into())
}

mod flash_lazy_remove_tests {
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn test() {
        use super::*;

        // Ensure the cookie's not there at first.
        let client = Client::debug_with(routes![set, unused, used]).unwrap();
        let response = client.get("/unused").dispatch();
        assert_eq!(response.status(), Status::NotFound);

        // Set the flash cookie.
        client.post("/").dispatch();

        // Try once.
        let response = client.get("/unused").dispatch();
        assert_eq!(response.status(), Status::Ok);

        // Try again; should still be there.
        let response = client.get("/unused").dispatch();
        assert_eq!(response.status(), Status::Ok);

        // Now use it.
        let response = client.get("/use").dispatch();
        assert_eq!(response.into_string(), Some(FLASH_MESSAGE.into()));

        // Now it should be gone.
        let response = client.get("/unused").dispatch();
        assert_eq!(response.status(), Status::NotFound);

        // Still gone.
        let response = client.get("/use").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}
