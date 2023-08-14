#[macro_use] extern crate rocket;

use std::path::PathBuf;

use rocket::local::blocking::Client;
use rocket::fairing::AdHoc;

#[get("/foo")]
fn foo() -> &'static str { "foo" }

#[get("/bar")]
fn not_bar() -> &'static str { "not_bar" }

#[get("/bar/")]
fn bar() -> &'static str { "bar" }

#[get("/foo/<_>/<_baz..>")]
fn baz(_baz: PathBuf) -> &'static str { "baz" }

#[get("/doggy/<_>/<_baz..>?doggy")]
fn doggy(_baz: PathBuf) -> &'static str { "doggy" }

#[get("/<_..>")]
fn rest() -> &'static str { "rest" }

macro_rules! assert_response {
    ($client:ident : $path:expr => $response:expr) => {
        let response = $client.get($path).dispatch().into_string().unwrap();
        assert_eq!(response, $response, "\nGET {}: got {} but expected {}",
            $path, response, $response);
    };
}

#[test]
fn test_adhoc_normalizer_works_as_expected () {
    let rocket = rocket::build()
        .mount("/", routes![foo, bar, not_bar, baz, doggy])
        .mount("/base", routes![foo, bar, not_bar, baz, doggy, rest])
        .attach(AdHoc::uri_normalizer());

    let client = match Client::debug(rocket) {
        Ok(client) => client,
        Err(e) => { e.pretty_print(); panic!("failed to build client"); }
    };

    assert_response!(client: "/foo" => "foo");
    assert_response!(client: "/foo/" => "foo");
    assert_response!(client: "/bar/" => "bar");
    assert_response!(client: "/bar" => "not_bar");
    assert_response!(client: "/foo/bar" => "baz");
    assert_response!(client: "/doggy/bar?doggy" => "doggy");
    assert_response!(client: "/foo/bar/" => "baz");
    assert_response!(client: "/foo/bar/baz" => "baz");
    assert_response!(client: "/base/foo/" => "foo");
    assert_response!(client: "/base/foo" => "foo");
    assert_response!(client: "/base/bar/" => "bar");
    assert_response!(client: "/base/bar" => "not_bar");
    assert_response!(client: "/base/foo/bar" => "baz");
    assert_response!(client: "/doggy/foo/bar?doggy" => "doggy");
    assert_response!(client: "/base/foo/bar/" => "baz");
    assert_response!(client: "/base/foo/bar/baz" => "baz");

    assert_response!(client: "/base/cat" => "rest");
    assert_response!(client: "/base/cat/" => "rest");
    assert_response!(client: "/base/cat/dog" => "rest");
}
