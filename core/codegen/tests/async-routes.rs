#![allow(dead_code)]

#[macro_use] extern crate rocket;
use rocket::http::uri::Origin;
use rocket::request::Request;

async fn noop() { }

#[get("/")]
async fn hello(_origin: &Origin<'_>) -> &'static str {
    noop().await;
    "Hello, world!"
}

#[catch(404)]
async fn not_found(req: &Request<'_>) -> String {
    noop().await;
    format!("{} not found", req.uri())
}
