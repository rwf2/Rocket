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

#[get("/repeated_query?<sort>")]
async fn repeated_query(sort: Vec<&str>) -> &str {
    noop().await;
    sort[0]
}

#[catch(404)]
async fn not_found(req: &Request<'_>) -> String {
    noop().await;
    format!("{} not found", req.uri())
}
