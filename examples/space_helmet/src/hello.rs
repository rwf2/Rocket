#![feature(decl_macro)]
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
use rocket::http::uri::Uri;

use rocket_contrib::space_helmet::{Helmet};
use rocket_contrib::space_helmet::*;

#[get("/")]
fn index() -> &'static str {
        "Hello, world!"
}

fn rocket() -> rocket::Rocket {
    let allow_uri  = Uri::parse("https://www.google.com").unwrap();
    let report_uri= Uri::parse("https://www.google.com").unwrap();
    let helmet = Helmet::new()
                        .no_sniff(None)
                        .frameguard(FramePolicy::AllowFrom(allow_uri))
                        .xss_protect(XSSPolicy::EnableReport(report_uri))
                        .hsts(HSTSPolicy::default())
                        .expect_ct(ExpectCTPolicy::default());
    rocket::ignite()
        .mount("/", routes![index])
        .attach(helmet)
}

fn main() {
    rocket().launch();
}
