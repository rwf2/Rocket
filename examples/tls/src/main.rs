#[macro_use] extern crate rocket;

use rocket::http::tls::MutualTlsUser;

#[cfg(test)] mod tests;

#[get("/")]
fn hello() -> &'static str {
    "Hello, world!"
}

#[get("/mtls")]
fn hello2(mtls: MutualTlsUser) -> String {
    format!("Hello, MTLS world, {}!", mtls.subject_name())
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![hello, hello2])
}
