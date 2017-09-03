#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket_contrib;
extern crate rocket;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use rocket::request::Form;
use rocket::response::Redirect;
use rocket::http::{Cookie, Cookies};
use rocket_contrib::Template;

#[derive(FromForm)]
struct Message {
    message: String,
}

#[post("/submit", data = "<message>")]
fn submit(mut cookies: Cookies, message: Form<Message>) -> Redirect {
    cookies.add(Cookie::new("message", message.into_inner().message));
    Redirect::to("/")
}

#[get("/")]
fn index(cookies: Cookies) -> Template {
    let cookie = cookies.get("message");
    let mut context = HashMap::new();
    if let Some(ref cookie) = cookie {
        context.insert("message", cookie.value());
    }

    Template::render("index", &context)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![submit, index]).attach(Template::fairing())
}

fn main() {
    rocket().launch();
}
