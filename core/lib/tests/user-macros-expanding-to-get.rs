#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

#[get("/hello/<name>/<age>")]
fn hello(name: String, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

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

fn main() {
    rocket::ignite().mount("/", routes![hello, hi]).launch();
}
