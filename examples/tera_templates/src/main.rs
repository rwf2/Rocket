#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket_contrib;
extern crate rocket;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket::Request;
use rocket::response::Redirect;
use rocket_contrib::Template;

#[derive(Serialize)]
struct TemplateContext {
    name: String,
    items: Vec<String>
}

#[get("/")]
fn index() -> Redirect {
    Redirect::to("/hello/Unknown")
}

#[get("/hello/<name>")]
fn get(name: String) -> Template {
    let context = TemplateContext {
        name: name,
        items: vec!["One", "Two", "Three"].iter().map(|s| s.to_string()).collect()
    };

    Template::render("index", &context)
}

#[error(404)]
fn not_found(req: &Request) -> Template {
    let mut map = std::collections::HashMap::new();
    map.insert("path", req.uri().as_str());
    Template::render("error/404", &map)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, get])
        .catch(errors![not_found])
        .launch();
}
