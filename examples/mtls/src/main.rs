#![feature(proc_macro_hygiene, decl_macro)]

use rocket::http::tls::MutualTlsUser;

#[cfg(test)] mod tests;

#[rocket::get("/")]
fn hello(mtls: MutualTlsUser) -> String {
    format!("Hello, MTLS world, {}!", mtls.subject_name())
}

fn main() {
    rocket::ignite().mount("/", rocket::routes![hello]).launch();
}
