#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

#[get("/")]
fn hello() -> &'static str {
    "Hello, world!"
}

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![hello])
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::local::asynchronous::Client;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn test_hello() {
        let client = Client::new(rocket()).await.unwrap();
        let response = client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().await, Some("Hello, world!".into()));
    }
}
