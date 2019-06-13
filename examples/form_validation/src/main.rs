#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

mod files;
#[cfg(test)] mod tests;

use rocket::response::Redirect;
use rocket::request::{Form, FromFormValue};
use rocket::http::RawStr;

#[derive(Debug)]
struct StrongPassword<'r>(&'r str);

#[derive(Debug)]
struct AdultAge(isize);

#[derive(FromForm)]
struct UserLogin<'r> {
    username: &'r RawStr,
    password: Result<StrongPassword<'r>, &'static str>,
    age: Result<AdultAge, &'static str>,
}

impl<'v> FromFormValue<'v> for StrongPassword<'v> {
    type Error = &'static str;

    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        if v.len() < 8 {
            Err("too short!")
        } else {
            Ok(StrongPassword(v.as_str()))
        }
    }
}

impl<'v> FromFormValue<'v> for AdultAge {
    type Error = &'static str;

    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        let age = match isize::from_form_value(v) {
            Ok(v) => v,
            Err(_) => return Err("value is not a number."),
        };

        match age > 20 {
            true => Ok(AdultAge(age)),
            false => Err("must be at least 21."),
        }
    }
}

#[post("/login", data = "<user>")]
fn login(user: Form<UserLogin<'_>>) -> Result<Redirect, String> {
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
fn user_page(username: &RawStr) -> String {
    format!("This is {}'s page.", username)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![files::index, files::files, user_page, login])
}

fn main() {
    rocket().launch();
}
