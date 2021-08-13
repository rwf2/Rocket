#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

#[get("/")]
fn hi() -> &'static str {
    "Hello, service user!"
}

pub async fn make() -> rocket::service::RocketService {
    let server = rocket::build()
        .mount("/", routes![hi]);
    server
        .ignite()
        .await
        .expect("invalid server configuration")
        .into_service()
}
