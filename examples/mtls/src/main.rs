#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::http::tls::MutualTlsUser;

#[cfg(test)] mod tests;

#[get("/")]
fn hello(mtls: MutualTlsUser) -> String {
    format!("Hello, {}!", mtls.get_common_names()[0])
    // format!("{}", mtls.get_common_names()[0])
}

fn main() {
    rocket::ignite().mount("/", routes![hello]).launch();
}
