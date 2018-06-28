// must-compile-successfully
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::response::content;

#[error(404)]
//~^ WARNING use of deprecated Rocket attribute `error` (deprecated since v0.3.15)
fn not_found(req: &rocket::Request) -> content::Html<String> {
    content::Html(format!("<p>whappen?</p>"))
}

fn main() {
    let e = rocket::ignite()
        .catch(errors![not_found])
        //~^ WARNING use of deprecated Rocket macro `errors` (deprecated since v0.3.15)
        .launch();
}
