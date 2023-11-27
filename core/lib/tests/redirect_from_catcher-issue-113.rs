#[macro_use]
extern crate rocket;

use rocket::response::Redirect;

#[catch(404)]
fn not_found() -> Redirect {
    Redirect::to("/")
}

mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn error_catcher_redirect() {
        let client = Client::debug(rocket::build().register("/", catchers![not_found])).unwrap();
        let response = client.get("/unknown").dispatch();

        let location: Vec<_> = response.headers().get("location").collect();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(location, vec!["/"]);
    }
}
