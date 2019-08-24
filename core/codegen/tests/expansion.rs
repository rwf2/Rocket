#![feature(proc_macro_hygiene)]

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

#[rocket::async_test]
async fn test_reexpansion() {
    let rocket = rocket::ignite().mount("/", routes![easy, hard, hi]);
    let client = Client::new(rocket).unwrap();

    let mut response = client.get("/easy/327").dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "easy id: 327");

    let mut response = client.get("/hard/72").dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "hard id: 72");

    let mut response = client.get("/hello/fish").dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "fish");
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

#[rocket::async_test]
async fn test_index() {
    let rocket = rocket::ignite().mount("/", routes![index]).manage(100i32);
    let client = Client::new(rocket).unwrap();

    let mut response = client.get("/").dispatch().await;
    assert_eq!(response.body_string().await.unwrap(), "Thing: 100");
}
