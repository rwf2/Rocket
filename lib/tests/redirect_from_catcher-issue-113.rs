#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::response::Redirect;

#[error(404)]
fn not_found() -> Redirect {
    Redirect::to("/")
}

mod tests {
    use super::*;
    use rocket::local::Client;
    use rocket::http::Status;

    #[test]
    fn error_catcher_redirect() {
        let client = Client::new(rocket::ignite().catch(errors![not_found])).unwrap();
        let response = client.get("/unknown").dispatch();
        println!("Response:\n{:?}", response);

        let location: Vec<_> = response.headers().get("location").collect();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(location, vec!["/"]);
    }
}
