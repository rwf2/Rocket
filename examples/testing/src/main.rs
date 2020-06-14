#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

#[get("/")]
fn hello() -> &'static str {
    "Hello, world!"
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![hello])
}

#[rocket::main]
async fn main() {
    let _ = rocket().launch().await;
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::local::Client;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn test_hello() {
        let client = Client::new(rocket()).await.unwrap();
        let mut response = client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string().await, Some("Hello, world!".into()));
    }
}
