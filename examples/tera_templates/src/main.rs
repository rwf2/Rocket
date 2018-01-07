#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket_contrib;
extern crate rocket;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use std::collections::HashMap;

use rocket::Request;
use rocket::response::Redirect;
use rocket_contrib::Template;

#[derive(Serialize)]
struct TemplateContext {
    name: String,
    items: Vec<&'static str>
}

#[get("/")]
fn index() -> Redirect {
    Redirect::to("/hello/Unknown")
}

#[get("/hello/<name>")]
fn get(name: String) -> Template {
    let context = TemplateContext { name, items: vec!["One", "Two", "Three"] };
    Template::render("index", &context)
}

#[catch(404)]
fn not_found(req: &Request) -> Template {
    let mut map = HashMap::new();
    map.insert("path", req.uri().as_str());
    Template::render("error/404", &map)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, get])
        .attach(Template::fairing())
        .catch(catchers![not_found])
}

fn main() {
    rocket().launch();
}
