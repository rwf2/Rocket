#[macro_use] extern crate rocket;

use rocket::local::blocking::Client;

#[get("/<id>")]
fn id(id: &str) -> &str { id }

#[get("/x")]
fn x() -> &'static str { "x" }

#[get("/a%20b")]
fn static_path() -> &'static str { "static path" }

#[get("/q?a%20b")]
fn static_query() -> &'static str { "static query" }

#[get("/wörld")]
fn utf8_path() -> &'static str { "utf8 path" }

#[catch(404)]
fn not_found() -> &'static str { "not found" }

#[test]
fn test_utf8_mount_point() {
    let rocket = rocket::build()
        .mount("/häcker", routes![id])
        .register("/häcker", catchers![not_found]);

    let client = Client::debug(rocket).unwrap();
    assert_eq!(client.get("/h%C3%A4cker/w%C3%B6rld").dispatch().into_string().unwrap(), "wörld");
    assert_eq!(client.get("/h%C3%A4cker/a/b").dispatch().into_string().unwrap(), "not found");
    assert_ne!(client.get("/a/b").dispatch().into_string().unwrap(), "not found");
}

#[test]
fn test_mount_point_as_written() {
    let rocket = rocket::build()
        .mount("/m%20n", routes![x])
        .register("/m%20n", catchers![not_found]);

    let client = Client::debug(rocket).unwrap();
    let uri = uri!("/m%20n", x);
    assert_eq!(uri.to_string(), "/m%2520n/x");
    assert_eq!(client.get(uri).dispatch().into_string().unwrap(), "x");
    assert_eq!(client.get("/m%2520n/nope").dispatch().into_string().unwrap(), "not found");

    let body = client.get("/m%20n/x").dispatch().into_string().unwrap();
    assert_ne!(body, "x");
    assert_ne!(body, "not found");
}

#[test]
fn test_uri_macro_round_trip() {
    let rocket = rocket::build()
        .mount("/", routes![static_path, static_query, utf8_path])
        .mount("/häcker", routes![id]);

    let client = Client::debug(rocket).unwrap();
    let uris = [
        (uri!(static_path), "/a%2520b", "static path"),
        (uri!(static_query), "/q?a%2520b", "static query"),
        (uri!(utf8_path), "/w%C3%B6rld", "utf8 path"),
        (uri!("/häcker", id("a b")), "/h%C3%A4cker/a%20b", "a b"),
    ];

    for (uri, expected, body) in uris {
        assert_eq!(uri.to_string(), expected);
        assert_eq!(client.get(uri).dispatch().into_string().unwrap(), body);
    }
}

#[test]
fn test_whitespace_routing() {
    use rocket::Route;
    use rocket::http::{Method, Status, uri::Origin};

    #[get("/foo ")]
    fn foo() -> &'static str { "foo" }

    #[get("/bar")]
    fn bar() -> &'static str { "bar" }

    let new = Route::new(Method::Get, "/new ", rocket::route::dummy_handler);
    assert_eq!(routes![foo][0].uri, "/foo ");
    assert_eq!(new.uri, "/new ");

    let rocket = rocket::build()
        .mount("/", routes![foo, bar])
        .mount("/", vec![new])
        .mount("/m ", routes![x])
        .register("/c ", catchers![not_found]);

    let client = Client::debug(rocket).unwrap();
    for (path, status) in [
        ("/foo%20", Status::Ok),
        ("/foo", Status::NotFound),
        ("/foo%20%20", Status::NotFound),
        ("/bar", Status::Ok),
        ("/bar%20", Status::NotFound),
        ("/new%20", Status::Ok),
        ("/new", Status::NotFound),
        ("/m%20/x", Status::Ok),
        ("/m/x", Status::NotFound),
    ] {
        assert_eq!(client.get(path).dispatch().status(), status, "{path:?}");
    }

    assert_eq!(client.get(Origin::path_only("/foo ")).dispatch().status(), Status::Ok);
    assert_eq!(client.get(Origin::path_only("/bar ")).dispatch().status(), Status::NotFound);
    assert_eq!(client.get("/c%20/x").dispatch().into_string().unwrap(), "not found");
    assert_ne!(client.get("/c/x").dispatch().into_string().unwrap(), "not found");
}

#[test]
fn test_whitespace_prefix_round_trip() {
    use rocket::http::Status;

    #[get("/")]
    fn root() -> &'static str { "root" }

    for (base, uri, expected) in [
        ("/ ", uri!("/ ", root), "/%20"),
        ("/a ", uri!("/a ", root), "/a%20"),
        ("/a/ \t", uri!("/a/ \t", root), "/a/%20%09"),
        ("/a\u{a0}", uri!("/a\u{a0}", root), "/a%C2%A0"),
        ("/a/\u{a0}", uri!("/a/\u{a0}", root), "/a/%C2%A0"),
        ("/a/ ?ignored", uri!("/a/ ", root), "/a/%20"),
        ("//a/// ", uri!("/a/ ", root), "/a/%20"),
        ("/a b ", uri!("/a b ", root), "/a%20b%20"),
        ("/a /b ", uri!("/a /b ", root), "/a%20/b%20"),
        ("/a%20 ", uri!("/a%20 ", root), "/a%2520%20"),
        ("/a%2Fb", uri!("/a%2Fb", root), "/a%252Fb"),
        ("/a#b", uri!("/a#b", root), "/a%23b"),
        ("/häcker", uri!("/häcker", root), "/h%C3%A4cker"),
    ] {
        assert_eq!(uri.to_string(), expected);
        let client = Client::debug(rocket::build().mount(base, routes![root])).unwrap();
        assert_eq!(client.get(uri).dispatch().into_string().unwrap(), "root");

        // A mounted root still distinguishes `/a` from `/a/`.
        if expected != "/" {
            let other = match expected.strip_suffix('/') {
                Some(path) => path.to_string(),
                None => format!("{expected}/"),
            };

            assert_eq!(client.get(other).dispatch().status(), Status::NotFound);
        }
    }
}
