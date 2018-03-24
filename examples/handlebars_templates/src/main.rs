#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket_contrib;
extern crate rocket;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket::Request;
use rocket::response::Redirect;
use rocket_contrib::{Template, handlebars};
use handlebars::{Helper, Handlebars, RenderContext, RenderError, JsonRender};

#[derive(Serialize)]
struct TemplateContext {
    name: String,
    items: Vec<String>,
    title: String,
    parent: String,
}

#[get("/")]
fn index() -> Redirect {
    Redirect::to("/hello/Unknown")
}

#[get("/hello/<name>")]
fn hello(name: String) -> Template {
    let page = "index".to_string();
    let title = format!("Rocket Example - {}", page).to_string();
    let context = TemplateContext {
        name: name,
        items: vec!["One".into(), "Two".into(), "Three".into()],
        parent: "layout".to_string(),
        title: title,
    };

    Template::render(page, &context)
}

#[get("/about")]
fn about() -> Template {
    let page = "about".to_string();
    let title = format!("Rocket Example - {}", page).to_string();
    let context = TemplateContext {
        name: "Unknown".to_string(),
        items: vec!["One".into(), "Two".into(), "Three".into()],
        parent: "layout".to_string(),
        title: title,
    };

    Template::render(page, &context)
}

#[catch(404)]
fn not_found(req: &Request) -> Template {
    let mut map = std::collections::HashMap::new();
    map.insert("path", req.uri().as_str());
    Template::render("error/404", &map)
}

type HelperResult = Result<(), RenderError>;

fn wow_helper(h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> HelperResult {
    if let Some(param) = h.param(0) {
        write!(rc.writer, "<b><i>{}</i></b>", param.value().render())?;
    }

    Ok(())
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, hello, about])
        .catch(catchers![not_found])
        .attach(Template::custom(|engines| {
            engines.handlebars.register_helper("wow", Box::new(wow_helper));
        }))
}

fn main() {
    rocket().launch();
}
