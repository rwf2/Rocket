#[macro_use]
extern crate rocket;

#[cfg(test)]
mod tests;

use rocket::http::tls::ClientTls;
use std::borrow::Cow;

#[get("/")]
fn hello(auth: Option<ClientTls>) -> Cow<'static, str> {
    match auth {
        None => "Hello, anonymous user".into(),
        Some(info) => {
            let fields = match info.end_entity.parse() {
                Ok(t) => t,
                Err(err) => return format!("I did not understand your certificate: {}", err).into()
            };
            format!("Hello, {:?} {:?}", fields.common_names, fields.organisation_units).into()
        }
    }
}

#[get("/secret")]
fn secret(_authenticated: ClientTls) -> &'static str {
    "secret stuff"
}

#[launch]
fn rocket() -> _ {
    // See `Rocket.toml` and `Cargo.toml` for TLS configuration.
    rocket::build().mount("/", routes![hello, secret])
}
