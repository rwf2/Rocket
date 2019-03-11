#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::local::Client;

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


macro_rules! foo {
    ($addr:expr, $name:ident) => {
        #[get($addr)]
        fn hi($name: String) -> String {
            $name
        }
    };
}

// regression test for `#[get] panicking if used inside a macro
foo!("/hello/<name>", name);

#[test]
fn test_reexpansion() {
    let rocket = rocket::ignite().mount("/", routes![easy, hard, hi]);
    let client = Client::new(rocket).unwrap();

    let mut response = client.get("/easy/327").dispatch();
    assert_eq!(response.body_string().unwrap(), "easy id: 327");

    let mut response = client.get("/hard/72").dispatch();
    assert_eq!(response.body_string().unwrap(), "hard id: 72");

    let mut response = client.get("/hello/fish").dispatch();
    assert_eq!(response.body_string().unwrap(), "fish");
}

macro_rules! index {
    ($type:ty) => {
        #[get("/")]
        fn index(thing: rocket::State<$type>) -> String {
            format!("Thing: {}", *thing)
        }
    }
}

index!(i32);

#[test]
fn test_index() {
    let rocket = rocket::ignite().mount("/", routes![index]).manage(100i32);
    let client = Client::new(rocket).unwrap();

    let mut response = client.get("/").dispatch();
    assert_eq!(response.body_string().unwrap(), "Thing: 100");
}
