#[macro_use] extern crate rocket;

use rocket::local::blocking::Client;
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

#[test]
fn test_formats() {
    let rocket = rocket::build()
        .mount("/", routes![json, xml, json_long, msgpack_long, msgpack,
               plain, binary, other]);

    let client = Client::debug(rocket).unwrap();

    let response = client.post("/").header(ContentType::JSON).dispatch();
    assert_eq!(response.into_string().unwrap(), "json");

    let response = client.post("/").header(ContentType::MsgPack).dispatch();
    assert_eq!(response.into_string().unwrap(), "msgpack_long");

    let response = client.post("/").header(ContentType::XML).dispatch();
    assert_eq!(response.into_string().unwrap(), "xml");

    let response = client.get("/").header(Accept::Plain).dispatch();
    assert_eq!(response.into_string().unwrap(), "plain");

    let response = client.get("/").header(Accept::Binary).dispatch();
    assert_eq!(response.into_string().unwrap(), "binary");

    let response = client.get("/").header(ContentType::JSON).dispatch();
    assert_eq!(response.into_string().unwrap(), "plain");

    let response = client.get("/").dispatch();
    assert_eq!(response.into_string().unwrap(), "plain");

    let response = client.put("/").header(ContentType::HTML).dispatch();
    assert_eq!(response.status(), Status::NotFound);
}

// Test custom formats.

// TODO: #[rocket(allow(unknown_format))]
#[get("/", format = "application/foo")]
fn get_foo() -> &'static str { "get_foo" }

// TODO: #[rocket(allow(unknown_format))]
#[post("/", format = "application/foo")]
fn post_foo() -> &'static str { "post_foo" }

// TODO: #[rocket(allow(unknown_format))]
#[get("/", format = "bar/baz", rank = 2)]
fn get_bar_baz() -> &'static str { "get_bar_baz" }

// TODO: #[rocket(allow(unknown_format))]
#[put("/", format = "bar/baz")]
fn put_bar_baz() -> &'static str { "put_bar_baz" }

#[test]
fn test_custom_formats() {
    let rocket = rocket::build()
        .mount("/", routes![get_foo, post_foo, get_bar_baz, put_bar_baz]);

    let client = Client::debug(rocket).unwrap();

    let foo_a = Accept::new([MediaType::new("application", "foo").into()]);
    let foo_ct = ContentType::new("application", "foo");
    let bar_baz_ct = ContentType::new("bar", "baz");
    let bar_baz_a = Accept::new([MediaType::new("bar", "baz").into()]);

    let response = client.get("/").header(foo_a).dispatch();
    assert_eq!(response.into_string().unwrap(), "get_foo");

    let response = client.post("/").header(foo_ct).dispatch();
    assert_eq!(response.into_string().unwrap(), "post_foo");

    let response = client.get("/").header(bar_baz_a).dispatch();
    assert_eq!(response.into_string().unwrap(), "get_bar_baz");

    let response = client.put("/").header(bar_baz_ct).dispatch();
    assert_eq!(response.into_string().unwrap(), "put_bar_baz");

    let response = client.get("/").dispatch();
    assert_eq!(response.into_string().unwrap(), "get_foo");

    let response = client.put("/").header(ContentType::HTML).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.post("/").header(ContentType::HTML).dispatch();
    assert_eq!(response.status(), Status::NotFound);
}
