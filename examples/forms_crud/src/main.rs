#[macro_use]
extern crate rocket;

use std::collections::HashMap;

use rocket::{
    form::{Context, Contextual, Form, FromForm, FromFormField},
    http::Status,
    State,
};
use rocket_contrib::serve::{crate_relative, StaticFiles};
use rocket_contrib::templates::Template;
use serde::Serialize;

struct DB(HashMap<usize, User>);

#[derive(Serialize)]
struct User {
    username: String,
    roles: Vec<Role>,
}

#[derive(Debug, FromFormField, Serialize)]
enum Role {
    Admin,
    Editor,
    Author,
}

#[derive(Debug, FromForm)]
struct UserForm<'v> {
    #[field(validate = len(3..))]
    username: &'v str,
    roles: Vec<Role>,
}

#[derive(Serialize)]
struct FormContext<'a> {
    /// Make it look like the values field of a Form Context.
    /// &str => &[&str]   | map from a field name to its submitted values  |
    values: HashMap<&'a str, Vec<&'a str>>,
    errors: HashMap<String, String>,
}

impl<'a> From<&'a User> for FormContext<'a> {
    fn from(user: &'a User) -> Self {
        let mut values = HashMap::new();

        let username = user.username.as_ref();
        let roles = user
            .roles
            .iter()
            .map(|r| match r {
                Role::Admin => "Admin",
                Role::Editor => "Editor",
                Role::Author => "Author",
            })
            .collect();

        values.insert("username", vec![username]);
        values.insert("roles", roles);

        FormContext {
            values,
            errors: HashMap::new(),
        }
    }
}

#[get("/")]
fn index(db: State<'_, DB>) -> Template {
    #[derive(Serialize)]
    struct ViewModel<'a> {
        users: &'a HashMap<usize, User>,
    }

    Template::render("index", ViewModel { users: &db.0 })
}

#[get("/users/<user_id>/edit")]
fn edit(db: State<'_, DB>, user_id: usize) -> Result<Template, Status> {
    let user = db.0.get(&user_id).ok_or(Status::NotFound)?;

    #[derive(Serialize)]
    struct ViewModel<'a, T: Serialize> {
        user: &'a User,
        form: T,
    }

    let form_context: FormContext = user.into();

    Ok(Template::render(
        "edit",
        ViewModel {
            user: &user,
            form: form_context,
        },
    ))
}

#[post("/users/<user_id>/edit", data = "<form>")]
fn update<'r>(
    db: State<'_, DB>,
    user_id: usize,
    form: Form<Contextual<'r, UserForm<'r>>>,
) -> Template {
    let template = match form.value {
        Some(ref submission) => {
            println!("submission: {:#?}", submission);
            Template::render("success", &form.context)
        }
        None => Template::render("index", &form.context),
    };

    // (form.context.status(), template)

    Template::render("edit", &Context::default())
}

#[launch]
fn rocket() -> rocket::Rocket {
    let brol = HashMap::<usize, User>::from_iter(IntoIter::new([(1, 2), (3, 4)]));

    let mut initial_users = HashMap::new();

    initial_users.insert(
        1,
        User {
            username: String::from("Nora"),
            roles: vec![Role::Admin, Role::Author, Role::Editor],
        },
    );

    initial_users.insert(
        42,
        User {
            username: String::from("Rob"),
            roles: vec![Role::Editor],
        },
    );

    initial_users.insert(
        136,
        User {
            username: String::from("Alexander"),
            roles: vec![Role::Author],
        },
    );

    rocket::ignite()
        .manage(DB(initial_users))
        .mount("/", routes![index, edit, update])
        .attach(Template::fairing())
        .mount("/", StaticFiles::from(crate_relative!("/static")))
}
