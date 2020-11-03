#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

#[cfg(test)]
mod tests;

#[get("/")]
fn world() -> &'static str {
    "hello, world!"
}

#[get("/")]
fn japan() -> &'static str {
    "hi, japan!"
}

fn main() {
    rocket::ignite()
        .mount("/hello", routes![world])
        .mount("/hi", routes![japan])
        .launch();
}
