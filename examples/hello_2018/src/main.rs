#[cfg(test)] mod tests;

#[rocket::get("/")]
fn hello() -> &'static str {
    "Hello, Rust 2018!"
}

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", rocket::routes![hello])
}
