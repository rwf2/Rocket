#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::request::{Form, LenientForm};

#[derive(FromForm)]
struct Person {
    /// Use the `form` attribute to expect an invalid Rust identifier in the HTTP form.
    #[form(field = "first-name")]
    name: String,
    age: Option<u8>
}

#[get("/hello?<person..>")]
fn hello(person: Option<Form<Person>>) -> String {
    if let Some(person) = person {
        if let Some(age) = person.age {
            format!("Hello, {} year old named {}!", age, person.name)
        } else {
            format!("Hello {}!", person.name)
        }
    } else {
        "We're gonna need a name, and only a name.".into()
    }
}

#[get("/hello?age=20&<person..>")]
fn hello_20(person: LenientForm<Person>) -> String {
    format!("20 years old? Hi, {}!", person.name)
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![hello, hello_20])
}
