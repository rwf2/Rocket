#[macro_use]extern crate rocket;

use rocket::http::{Status, ContentType};
use rocket::form::{Form, Contextual, FromForm, FromFormField, Context};
use rocket::data::TempFile;

use rocket_contrib::serve::{StaticFiles, crate_relative};
use rocket_contrib::templates::Template;


struct User {
    id: usize,
    username: String,
    roles: Vec<Role>
}

#[derive(Debug, FromFormField)]
enum Role {
    Admin
    Editor,
    Author,
}

#[derive(Debug, FromForm)]
struct UserForm<'v> {
    #[field(validate = len(3..))]
    username: &'v str,

    roles: Vec<Role>
}

#[get("/")]
fn index<'r>() -> Template {
    Template::render("index", &Context::default())
}

#[get("users/<user_id>/edit/")]
fn edit<'r>(user_id: usize) -> Template {
    Template::render("edit", &Context::default())
}

#[post("users/<user_id>/edit/", data = "<form>")]
fn update<'r>(user_id: usize, form: Form<Contextual<'r, UserForm<'r>>>) -> Template {
    Template::render("edit", &Context::default())
}

#[post("/", data = "<form>")]
fn submit<'r>() -> (Status, Template) {
    let template = match form.value {
        Some(ref submission) => {
            println!("submission: {:#?}", submission);
            Template::render("success", &form.context)
        }
        None => Template::render("index", &form.context),
    };

    (form.context.status(), template)
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, edit, update])
        .attach(Template::fairing())
        .mount("/", StaticFiles::from(crate_relative!("/static")))
}
