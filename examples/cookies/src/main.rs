#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

mod session;
mod message;

use rocket::response::content::RawHtml;
use rocket_dyn_templates::Template;

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(r#"<a href="message">Set a Message</a> or <a href="session">Use Sessions</a>."#)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .mount("/", routes![index])
        .mount("/message", message::routes())
        .mount("/session", session::routes())
}
