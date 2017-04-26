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
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;
    use rocket::http::Status;

    #[test]
    fn error_catcher_redirect() {
        let rocket = rocket::ignite().catch(errors![not_found]);
        let mut req = MockRequest::new(Get, "/unknown");
        let response = req.dispatch_with(&rocket);
        println!("Response:\n{:?}", response);

        let location: Vec<_> = response.headers().get("location").collect();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(location, vec!["/"]);
    }
}
