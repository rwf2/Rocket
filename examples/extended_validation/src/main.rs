#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

mod files;
#[cfg(test)]
mod tests;

use rocket::response::Redirect;
use rocket::request::{Form, FromFormValue};

#[derive(Debug)]
struct StrongPassword<'r>(&'r str);

#[derive(Debug)]
struct AdultAge(isize);

#[derive(FromForm)]
struct UserLogin<'r> {
    username: &'r str,
    password: Result<StrongPassword<'r>, &'static str>,
    age: Result<AdultAge, &'static str>,
}

impl<'v> FromFormValue<'v> for StrongPassword<'v> {
    type Error = &'static str;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        if v.len() < 8 {
            Err("Too short!")
        } else {
            Ok(StrongPassword(v))
        }
    }
}

impl<'v> FromFormValue<'v> for AdultAge {
    type Error = &'static str;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        let age = match isize::from_form_value(v) {
            Ok(v) => v,
            Err(_) => return Err("Age value is not a number."),
        };

        match age > 20 {
            true => Ok(AdultAge(age)),
            false => Err("Must be at least 21."),
        }
    }
}

#[post("/login", data = "<user_form>")]
fn login<'a>(user_form: Form<'a, UserLogin<'a>>) -> Result<Redirect, String> {
    let user = user_form.get();

    if let Err(e) = user.age {
        return Err(format!("Age is invalid: {}", e));
    }

    if let Err(e) = user.password {
        return Err(format!("Password is invalid: {}", e));
    }

    if user.username == "Sergio" {
        if let Ok(StrongPassword("password")) = user.password {
            Ok(Redirect::to("/user/Sergio"))
        } else {
            Err("Wrong password!".to_string())
        }
    } else {
        Err(format!("Unrecognized user, '{}'.", user.username))
    }
}

#[get("/user/<username>")]
fn user_page(username: &str) -> String {
    format!("This is {}'s page.", username)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![files::index, files::files, user_page, login])
}

fn main() {
    rocket().launch()
}
