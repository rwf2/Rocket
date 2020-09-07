#[macro_use] extern crate rocket;

use rocket::http::tls::MutualTlsUser;

#[cfg(test)] mod tests;

#[get("/")]
fn hello(mtls: MutualTlsUser) -> String {
    format!("Hello, MTLS world, {}!", mtls.subject_name())
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![hello])
}
