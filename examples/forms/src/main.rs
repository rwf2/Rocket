#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

mod files;
#[cfg(test)] mod tests;

use rocket::request::Form;
use rocket::response::Redirect;

#[derive(FromForm)]
struct UserLogin<'r> {
    username: &'r str,
    password: String,
    age: Result<usize, &'r str>,
}

#[post("/login", data = "<user_form>")]
fn login<'a>(user_form: Form<'a, UserLogin<'a>>) -> Result<Redirect, String> {
    let user = user_form.get();
    match user.age {
        Ok(age) if age < 21 => return Err(format!("Sorry, {} is too young!", age)),
        Ok(age) if age > 120 => return Err(format!("Are you sure you're {}?", age)),
        Err(e) => return Err(format!("'{}' is not a valid integer.", e)),
        Ok(_) => { /* Move along, adult. */ }
    };

    if user.username == "Sergio" {
        match user.password.as_str() {
            "password" => Ok(Redirect::to("/user/Sergio")),
            _ => Err("Wrong password!".to_string())
        }
    } else {
        Err(format!("Unrecognized user, '{}'.", user.username))
    }
}


#[get("/user/<username>")]
fn user_page(username: &str) -> String {
    format!("This is {}'s page.", username)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![files::index, files::files, user_page, login])
        .launch();
}
