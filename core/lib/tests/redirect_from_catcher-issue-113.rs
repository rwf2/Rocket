#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::response::Redirect;

#[catch(404)]
fn not_found() -> Redirect {
    Redirect::to("/")
}

mod tests {
    use super::*;
    use rocket::local::Client;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn error_catcher_redirect() {
        let client = Client::new(rocket::ignite().register(catchers![not_found])).await.unwrap();
        let response = client.get("/unknown").dispatch().await;
        println!("Response:\n{:?}", response);

        let location: Vec<_> = response.headers().get("location").collect();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(location, vec!["/"]);
    }
}
