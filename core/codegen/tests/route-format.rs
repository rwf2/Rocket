#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::local::Client;
use rocket::http::{ContentType, MediaType, Accept, Status};

// Test that known formats work as expected, including not colliding.

#[post("/", format = "json")]
fn json() -> &'static str { "json" }

#[post("/", format = "xml")]
fn xml() -> &'static str { "xml" }

// Unreachable. Written for codegen.
#[post("/", format = "application/json", rank = 2)]
fn json_long() -> &'static str { "json_long" }

#[post("/", format = "application/msgpack")]
fn msgpack_long() -> &'static str { "msgpack_long" }

// Unreachable. Written for codegen.
#[post("/", format = "msgpack", rank = 2)]
fn msgpack() -> &'static str { "msgpack" }

#[get("/", format = "plain")]
fn plain() -> &'static str { "plain" }

#[get("/", format = "binary", rank = 2)]
fn binary() -> &'static str { "binary" }

#[get("/", rank = 3)]
fn other() -> &'static str { "other" }

#[rocket::async_test]
async fn test_formats() {
    let rocket = rocket::ignite()
        .mount("/", routes![json, xml, json_long, msgpack_long, msgpack,
               plain, binary, other]);

    let client = Client::new(rocket).await.unwrap();

    let mut response = client.post("/").header(ContentType::JSON).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "json");

    let mut response = client.post("/").header(ContentType::MsgPack).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "msgpack_long");

    let mut response = client.post("/").header(ContentType::XML).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "xml");

    let mut response = client.get("/").header(Accept::Plain).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "plain");

    let mut response = client.get("/").header(Accept::Binary).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "binary");

    let mut response = client.get("/").header(ContentType::JSON).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "plain");

    let mut response = client.get("/").dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "plain");

    let response = client.put("/").header(ContentType::HTML).dispatch().await;
    assert_eq!(response.status(), Status::NotFound);
}

// Test custom formats.

#[get("/", format = "application/foo")]
fn get_foo() -> &'static str { "get_foo" }

#[post("/", format = "application/foo")]
fn post_foo() -> &'static str { "post_foo" }

#[get("/", format = "bar/baz", rank = 2)]
fn get_bar_baz() -> &'static str { "get_bar_baz" }

#[put("/", format = "bar/baz")]
fn put_bar_baz() -> &'static str { "put_bar_baz" }

#[rocket::async_test]
async fn test_custom_formats() {
    let rocket = rocket::ignite()
        .mount("/", routes![get_foo, post_foo, get_bar_baz, put_bar_baz]);

    let client = Client::new(rocket).await.unwrap();

    let foo_a = Accept::new(&[MediaType::new("application", "foo").into()]);
    let foo_ct = ContentType::new("application", "foo");
    let bar_baz_ct = ContentType::new("bar", "baz");
    let bar_baz_a = Accept::new(&[MediaType::new("bar", "baz").into()]);

    let mut response = client.get("/").header(foo_a).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "get_foo");

    let mut response = client.post("/").header(foo_ct).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "post_foo");

    let mut response = client.get("/").header(bar_baz_a).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "get_bar_baz");

    let mut response = client.put("/").header(bar_baz_ct).dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "put_bar_baz");

    let mut response = client.get("/").dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "get_foo");

    let response = client.put("/").header(ContentType::HTML).dispatch().await;
    assert_eq!(response.status(), Status::NotFound);

    let response = client.post("/").header(ContentType::HTML).dispatch().await;
    assert_eq!(response.status(), Status::NotFound);
}
