#![warn(rust_2018_idioms)]
#[cfg(test)] mod tests;

#[rocket::get("/")]
fn hello() -> &'static str {
    "Hello, Rust 2018!"
}

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", rocket::routes![hello])
}

#[rocket::catch(404)]
fn not_foundr(_req: &'_ rocket::Request<'_>) -> String {
    "404 Not Found".to_owned()
}
