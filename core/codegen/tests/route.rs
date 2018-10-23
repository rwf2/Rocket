#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use std::fmt;
use std::path::PathBuf;

use rocket::{Request, Outcome::*};
use rocket::local::Client;
use rocket::data::{self, Data, FromDataSimple};
use rocket::request::Form;
use rocket::http::{Status, RawStr, ContentType, uri::UriDisplay};

// Use all of the code generation avaiable at once.

#[derive(FromForm)]
struct Inner<'r> {
    field: &'r RawStr
}

// TODO: Make this deriveable.
impl<'a> UriDisplay for Inner<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "field={}", self.field)
    }
}

struct Simple(String);

impl FromDataSimple for Simple {
    type Error = ();

    fn from_data(_: &Request, data: Data) -> data::Outcome<Self, ()> {
        use std::io::Read;
        let mut string = String::new();
        data.open().take(64).read_to_string(&mut string).unwrap();
        Success(Simple(string))
    }
}

#[post("/<a>/<name>/name/<path..>?sky=blue&<sky>&<query..>", format = "json", data = "<simple>", rank = 138)]
fn post1(
    sky: usize,
    name: &RawStr,
    a: String,
    query: Form<Inner>,
    path: PathBuf,
    simple: Simple,
) -> String {
    let string = format!("{}, {}, {}, {}, {}, {}",
        sky, name, a, query.field, path.display(), simple.0);

    let uri = uri!(post2: a, name.url_decode_lossy(), path, sky, query.into_inner());

    format!("({}) ({})", string, uri.to_string())
}

#[route(POST, path = "/<a>/<name>/name/<path..>?sky=blue&<sky>&<query..>", format = "json", data = "<simple>", rank = 138)]
fn post2(
    sky: usize,
    name: &RawStr,
    a: String,
    query: Form<Inner>,
    path: PathBuf,
    simple: Simple,
) -> String {
    let string = format!("{}, {}, {}, {}, {}, {}",
        sky, name, a, query.field, path.display(), simple.0);

    let uri = uri!(post2: a, name.url_decode_lossy(), path, sky, query.into_inner());

    format!("({}) ({})", string, uri.to_string())
}

#[test]
fn test_full_route() {
    let rocket = rocket::ignite()
        .mount("/1", routes![post1])
        .mount("/2", routes![post2]);

    let client = Client::new(rocket).unwrap();

    let a = "A%20A";
    let name = "Bob%20McDonald";
    let path = "this/path/here";
    let sky = 777;
    let query = "field=inside";
    let simple = "data internals";

    let path_part = format!("/{}/{}/name/{}", a, name, path);
    let query_part = format!("?sky={}&sky=blue&{}", sky, query);
    let uri = format!("{}{}", path_part, query_part);
    let expected_uri = format!("{}?sky=blue&sky={}&{}", path_part, sky, query);

    let response = client.post(&uri).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.post(format!("/1{}", uri)).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let mut response = client
        .post(format!("/1{}", uri))
        .header(ContentType::JSON)
        .body(simple)
        .dispatch();

    assert_eq!(response.body_string().unwrap(), format!("({}, {}, {}, {}, {}, {}) ({})",
            sky, name, "A A", "inside", path, simple, expected_uri));

    let response = client.post(format!("/2{}", uri)).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let mut response = client
        .post(format!("/2{}", uri))
        .header(ContentType::JSON)
        .body(simple)
        .dispatch();

    assert_eq!(response.body_string().unwrap(), format!("({}, {}, {}, {}, {}, {}) ({})",
            sky, name, "A A", "inside", path, simple, expected_uri));
}

// Check that we propogate span information correctly to allow re-expansion.

#[get("/easy/<id>")]
fn easy(id: i32) -> String {
    format!("easy id: {}", id)
}

macro_rules! make_handler {
    () => {
        #[get("/hard/<id>")]
        fn hard(id: i32) -> String {
            format!("hard id: {}", id)
        }
    }
}

make_handler!();

#[test]
fn test_reexpansion() {
    let rocket = rocket::ignite().mount("/", routes![easy, hard]);
    let client = Client::new(rocket).unwrap();

    let mut response = client.get("/easy/327").dispatch();
    assert_eq!(response.body_string().unwrap(), "easy id: 327");

    let mut response = client.get("/hard/72").dispatch();
    assert_eq!(response.body_string().unwrap(), "hard id: 72");
}
