#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::mtls::Certificate;

#[get("/")]
fn mutual(cert: Certificate<'_>) -> String {
    format!("Hello! Here's what we know: [{}] {}", cert.serial(), cert.subject())
}

#[get("/", rank = 2)]
fn hello() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    // See `Rocket.toml` and `Cargo.toml` for TLS configuration.
    // Run `./private/gen_certs.sh` to generate a CA and key pairs.
    rocket::build().mount("/", routes![hello, mutual])
}
