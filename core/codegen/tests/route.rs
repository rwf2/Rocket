// Rocket sometimes generates mangled identifiers that activate the
// non_snake_case lint. We deny the lint in this test to ensure that
// code generation uses #[allow(non_snake_case)] in the appropriate places.
#![deny(non_snake_case)]

#[macro_use] extern crate rocket;

use std::path::PathBuf;

use rocket::request::Request;
use rocket::http::ext::Normalize;
use rocket::local::blocking::Client;
use rocket::data::{self, Data, FromData};
use rocket::http::{Status, RawStr, ContentType, uri::fmt::Path};

// Use all of the code generation available at once.

#[derive(FromForm, UriDisplayQuery)]
struct Inner<'r> {
    field: &'r str
}

struct Simple(String);

#[async_trait]
impl<'r> FromData<'r> for Simple {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        String::from_data(req, data).await.map(Simple)
    }
}

#[post(
    "/<a>/<name>/name/<path..>?sky=blue&<sky>&<query..>",
    format = "json",
    data = "<simple>",
    rank = 138
)]
fn post1(
    sky: usize,
    name: &str,
    a: String,
    query: Inner<'_>,
    path: PathBuf,
    simple: Simple,
) -> String {
    let string = format!("{}, {}, {}, {}, {}, {}",
        sky, name, a, query.field, path.normalized_str(), simple.0);

    let uri = uri!(post1(a, name, path, sky, query));

    format!("({}) ({})", string, uri.to_string())
}

#[route(
    POST,
    uri = "/<a>/<name>/name/<path..>?sky=blue&<sky>&<query..>",
    format = "json",
    data = "<simple>",
    rank = 138
)]
fn post2(
    sky: usize,
    name: &str,
    a: String,
    query: Inner<'_>,
    path: PathBuf,
    simple: Simple,
) -> String {
    let string = format!("{}, {}, {}, {}, {}, {}",
        sky, name, a, query.field, path.normalized_str(), simple.0);

    let uri = uri!(post2(a, name, path, sky, query));

    format!("({}) ({})", string, uri.to_string())
}

#[allow(dead_code)]
#[post("/<_unused_param>?<_unused_query>", data="<_unused_data>")]
fn test_unused_params(_unused_param: String, _unused_query: String, _unused_data: Data<'_>) {
}

#[test]
fn test_full_route() {
    let rocket = rocket::build()
        .mount("/1", routes![post1])
        .mount("/2", routes![post2]);

    let client = Client::debug(rocket).unwrap();

    let a = RawStr::new("A%20A");
    let name = RawStr::new("Bob%20McDonald");
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

    let response = client
        .post(format!("/1{}", uri))
        .header(ContentType::JSON)
        .body(simple)
        .dispatch();

    assert_eq!(response.into_string().unwrap(), format!("({}, {}, {}, {}, {}, {}) ({})",
            sky, name.percent_decode().unwrap(), "A A", "inside", path, simple, expected_uri));

    let response = client.post(format!("/2{}", uri)).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client
        .post(format!("/2{}", uri))
        .header(ContentType::JSON)
        .body(simple)
        .dispatch();

    assert_eq!(response.into_string().unwrap(), format!("({}, {}, {}, {}, {}, {}) ({})",
            sky, name.percent_decode().unwrap(), "A A", "inside", path, simple, expected_uri));
}

mod scopes {
    #![allow(dead_code)]

    mod other {
        #[get("/world")]
        pub fn world() -> &'static str {
            "Hello, world!"
        }
    }

    #[get("/hello")]
    pub fn hello() -> &'static str {
        "Hello, outside world!"
    }

    use other::world;

    fn _rocket() -> rocket::Rocket<rocket::Build> {
        rocket::build().mount("/", rocket::routes![hello, world, other::world])
    }
}

use rocket::form::Contextual;

#[derive(Default, Debug, PartialEq, FromForm)]
struct Filtered<'r> {
    bird: Option<&'r str>,
    color: Option<&'r str>,
    cat: Option<&'r str>,
    rest: Option<&'r str>,
}

#[get("/?bird=1&color=blue&<bird>&<color>&cat=bob&<rest..>")]
fn filtered_raw_query(bird: usize, color: &str, rest: Contextual<'_, Filtered<'_>>) -> String {
    assert_ne!(bird, 1);
    assert_ne!(color, "blue");
    assert_eq!(rest.value.unwrap(), Filtered::default());

    format!("{} - {}", bird, color)
}

#[test]
fn test_filtered_raw_query() {
    let rocket = rocket::build().mount("/", routes![filtered_raw_query]);
    let client = Client::debug(rocket).unwrap();

    #[track_caller]
    fn run(client: &Client, birds: &[&str], colors: &[&str], cats: &[&str]) -> (Status, String) {
        let join = |slice: &[&str], name: &str| slice.iter()
            .map(|v| format!("{}={}", name, v))
            .collect::<Vec<_>>()
            .join("&");

        let q = format!("{}&{}&{}",
            join(birds, "bird"),
            join(colors, "color"),
            join(cats, "cat"));

        let response = client.get(format!("/?{}", q)).dispatch();
        let status = response.status();
        let body = response.into_string().unwrap();

        (status, body)
    }

    let birds = &["2", "3"];
    let colors = &["red", "blue", "green"];
    let cats = &["bob", "bob"];
    assert_eq!(run(&client, birds, colors, cats).0, Status::NotFound);

    let birds = &["2", "1", "3"];
    let colors = &["red", "green"];
    let cats = &["bob", "bob"];
    assert_eq!(run(&client, birds, colors, cats).0, Status::NotFound);

    let birds = &["2", "1", "3"];
    let colors = &["red", "blue", "green"];
    let cats = &[];
    assert_eq!(run(&client, birds, colors, cats).0, Status::NotFound);

    let birds = &["2", "1", "3"];
    let colors = &["red", "blue", "green"];
    let cats = &["bob", "bob"];
    assert_eq!(run(&client, birds, colors, cats).1, "2 - red");

    let birds = &["1", "2", "1", "3"];
    let colors = &["blue", "red", "blue", "green"];
    let cats = &["bob"];
    assert_eq!(run(&client, birds, colors, cats).1, "2 - red");

    let birds = &["5", "1"];
    let colors = &["blue", "orange", "red", "blue", "green"];
    let cats = &["bob"];
    assert_eq!(run(&client, birds, colors, cats).1, "5 - orange");
}

#[derive(Debug, PartialEq, FromForm)]
struct Dog<'r> {
    name: &'r str,
    age: usize
}

#[derive(Debug, PartialEq, FromForm)]
struct Q<'r> {
    dog: Dog<'r>
}

#[get("/?<color>&color=red&<q..>")]
fn query_collection(color: Vec<&str>, q: Q<'_>) -> String {
    format!("{} - {} - {}", color.join("&"), q.dog.name, q.dog.age)
}

#[get("/?<color>&color=red&<dog>")]
fn query_collection_2(color: Vec<&str>, dog: Dog<'_>) -> String {
    format!("{} - {} - {}", color.join("&"), dog.name, dog.age)
}

#[test]
fn test_query_collection() {
    #[track_caller]
    fn run(client: &Client, colors: &[&str], dog: &[&str]) -> (Status, String) {
        let join = |slice: &[&str], prefix: &str| slice.iter()
            .map(|v| format!("{}{}", prefix, v))
            .collect::<Vec<_>>()
            .join("&");

        let q = format!("{}&{}", join(colors, "color="), join(dog, "dog."));
        let response = client.get(format!("/?{}", q)).dispatch();
        (response.status(), response.into_string().unwrap())
    }

    fn run_tests(rocket: rocket::Rocket<rocket::Build>) {
        let client = Client::debug(rocket).unwrap();

        let colors = &["blue", "green"];
        let dog = &["name=Fido", "age=10"];
        assert_eq!(run(&client, colors, dog).0, Status::NotFound);

        let colors = &["red"];
        let dog = &["name=Fido"];
        assert_eq!(run(&client, colors, dog).0, Status::NotFound);

        let colors = &["red"];
        let dog = &["name=Fido", "age=2"];
        assert_eq!(run(&client, colors, dog).1, " - Fido - 2");

        let colors = &["red", "blue", "green"];
        let dog = &["name=Fido", "age=10"];
        assert_eq!(run(&client, colors, dog).1, "blue&green - Fido - 10");

        let colors = &["red", "blue", "green"];
        let dog = &["name=Fido", "age=10", "toy=yes"];
        assert_eq!(run(&client, colors, dog).1, "blue&green - Fido - 10");

        let colors = &["blue", "red", "blue"];
        let dog = &["name=Fido", "age=10"];
        assert_eq!(run(&client, colors, dog).1, "blue&blue - Fido - 10");

        let colors = &["blue", "green", "red", "blue"];
        let dog = &["name=Max+Fido", "age=10"];
        assert_eq!(run(&client, colors, dog).1, "blue&green&blue - Max Fido - 10");
    }

    let rocket = rocket::build().mount("/", routes![query_collection]);
    run_tests(rocket);

    let rocket = rocket::build().mount("/", routes![query_collection_2]);
    run_tests(rocket);
}

use rocket::request::FromSegments;
use rocket::http::uri::Segments;

struct PathString(String);

impl FromSegments<'_> for PathString {
    type Error = std::convert::Infallible;

    fn from_segments(segments: Segments<'_, Path>) -> Result<Self, Self::Error> {
        Ok(PathString(segments.collect::<Vec<_>>().join("/")))
    }

}

#[get("/<_>/b/<path..>", rank = 1)]
fn segments(path: PathString) -> String {
    format!("nonempty+{}", path.0)
}

#[get("/<path..>", rank = 2)]
fn segments_empty(path: PathString) -> String {
    format!("empty+{}", path.0)
}

#[test]
fn test_inclusive_segments() {
    let rocket = rocket::build()
        .mount("/", routes![segments])
        .mount("/", routes![segments_empty]);

    let client = Client::debug(rocket).unwrap();
    let get = |uri| client.get(uri).dispatch().into_string().unwrap();

    assert_eq!(get("/"), "empty+");
    assert_eq!(get("//"), "empty+");
    assert_eq!(get("//a/"), "empty+a");
    assert_eq!(get("//a//"), "empty+a");
    assert_eq!(get("//a//c/d"), "empty+a/c/d");

    assert_eq!(get("//a/b"), "nonempty+");
    assert_eq!(get("//a/b/c"), "nonempty+c");
    assert_eq!(get("//a/b//c"), "nonempty+c");
    assert_eq!(get("//a//b////c"), "nonempty+c");
    assert_eq!(get("//a//b////c/d/e"), "nonempty+c/d/e");
}
