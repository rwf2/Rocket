#[macro_use] extern crate rocket;

mod hbs;
mod tera;

#[cfg(test)] mod tests;

use rocket::response::content::RawHtml;
use rocket_dyn_templates::Template;

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(r#"See <a href="tera">Tera</a> or <a href="hbs">Handlebars</a>."#)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/tera", routes![tera::index, tera::hello, tera::about])
        .mount("/hbs", routes![hbs::index, hbs::hello, hbs::about])
        .register("/hbs", catchers![hbs::not_found])
        .register("/tera", catchers![tera::not_found])
        .attach(Template::custom(|engines| {
            hbs::customize(&mut engines.handlebars);
            tera::customize(&mut engines.tera);
        }))
}
