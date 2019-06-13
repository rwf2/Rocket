#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use std::collections::HashMap;

use rocket::Request;
use rocket::response::Redirect;
use rocket_contrib::templates::Template;

#[derive(Serialize)]
struct TemplateContext {
    name: String,
    items: Vec<&'static str>
}

#[get("/")]
fn index() -> Redirect {
    Redirect::to(uri!(get: name = "Unknown"))
}

#[get("/hello/<name>")]
fn get(name: String) -> Template {
    let context = TemplateContext { name, items: vec!["One", "Two", "Three"] };
    Template::render("index", &context)
}

#[catch(404)]
fn not_found(req: &Request<'_>) -> Template {
    let mut map = HashMap::new();
    map.insert("path", req.uri().path());
    Template::render("error/404", &map)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, get])
        .attach(Template::fairing())
        .register(catchers![not_found])
}

fn main() {
    rocket().launch();
}
