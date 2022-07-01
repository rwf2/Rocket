#[macro_use] extern crate rocket;

use rocket::local::blocking::Client;

// Test that raw idents can be used for route parameter names

#[get("/<enum>?<type>")]
fn get(r#enum: String, r#type: i32) -> String {
    format!("{} is {}", r#enum, r#type)
}

#[get("/swap/<raw>/<bare>")]
fn swap(r#raw: String, bare: String) -> String {
    format!("{}, {}", raw, bare)
}

#[catch(400)]
fn catch(r#raw: &rocket::Request) -> String {
    format!("{}", raw.method())
}

#[test]
fn test_raw_ident() {
    let rocket = rocket::build()
        .mount("/", routes![get, swap])
        .register("/", catchers![catch]);

    let client = Client::debug(rocket).unwrap();

    let response = client.get("/example?type=1").dispatch();
    assert_eq!(response.into_string().unwrap(), "example is 1");

    let uri_named = uri!(get(r#enum = "test_named", r#type = 1));
    assert_eq!(uri_named.to_string(), "/test_named?type=1");

    let uri_unnamed = uri!(get("test_unnamed", 2));
    assert_eq!(uri_unnamed.to_string(), "/test_unnamed?type=2");

    let uri_raws = uri!(swap(r#raw = "1", r#bare = "2"));
    assert_eq!(uri_raws.to_string(), "/swap/1/2");
    let uri_bare = uri!(swap(raw = "1", bare = "2"));
    assert_eq!(uri_bare.to_string(), "/swap/1/2");
}
