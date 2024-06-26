#[macro_use] extern crate rocket;

mod hbs;
mod minijinja;
mod tera;

#[cfg(test)] mod tests;

use rocket::response::content::RawHtml;
use rocket_dyn_templates::Templating;

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(r#"See <a href="tera">Tera</a>,
        <a href="hbs">Handlebars</a>,
        or <a href="minijinja">MiniJinja</a>."#)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/tera", routes![tera::index, tera::hello, tera::about])
        .mount("/hbs", routes![hbs::index, hbs::hello, hbs::about])
        .mount("/minijinja", routes![minijinja::index, minijinja::hello, minijinja::about])
        .register("/hbs", catchers![hbs::not_found])
        .register("/tera", catchers![tera::not_found])
        .register("/minijinja", catchers![minijinja::not_found])
        .attach(Templating::init(|engines| {
            hbs::init(&mut engines.handlebars);
        }).on_load(|engines| {
            hbs::customize(&mut engines.handlebars);
            tera::customize(&mut engines.tera);
            minijinja::customize(&mut engines.minijinja);
        }))
}
