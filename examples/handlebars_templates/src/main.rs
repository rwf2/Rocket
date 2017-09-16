#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket_contrib;
extern crate rocket;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket::Request;
use rocket::response::Redirect;
use rocket_contrib::Template;
use rocket_contrib::handlebars::{Helper, Handlebars, RenderContext, RenderError};

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

#[catch(404)]
fn not_found(req: &Request) -> Template {
    let mut map = std::collections::HashMap::new();
    map.insert("path", req.uri().as_str());
    Template::render("error/404", &map)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, get])
        .attach(Template::fairing())
        .catch(catchers![not_found])
}

// when you want to register handlebars helpers
#[allow(dead_code)]
fn rocket_with_hbs_helper() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, get])
        .attach(Template::config_and_fairing(|ctxt| {
            ctxt.engines_mut()
                .handlebars()
                .register_helper("test",
                                 Box::new(|_: &Helper,
                                           _: &Handlebars,
                                           _: &mut RenderContext|
                                           -> Result<(), RenderError> {
                                              // do nothing
                                              Ok(())
                                          }));

        }))
        .catch(catchers![not_found])
}

fn main() {
    rocket().launch();
}
