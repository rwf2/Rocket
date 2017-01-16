#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate lazy_static;
extern crate uuid;
extern crate rocket;
extern crate rocket_contrib;

use std::collections::HashMap;
use uuid::Uuid;
use rocket_contrib::UUID;

#[cfg(test)]
mod tests;

lazy_static! {
    // A small people lookup table. In a real application this would be a 
    // database lookup. Notice that we use the uuid::Uuid type here and not 
    // rockets UUID type.
    static ref PEOPLE: HashMap<Uuid, &'static str> = {
        let mut m = HashMap::new();
        let lacy_id = Uuid::parse_str("7f205202-7ba1-4c39-b2fc-3e630722bf9f").unwrap();
        let bob_id = Uuid::parse_str("4da34121-bc7d-4fc1-aee6-bf8de0795333").unwrap();
        let george_id = Uuid::parse_str("ad962969-4e3d-4de7-ac4a-2d86d6d10839").unwrap();
        m.insert(lacy_id, "Lacy");
        m.insert(bob_id, "Bob");
        m.insert(george_id, "George");
        m
    };
}

#[get("/<id>")]
fn show(id: UUID) -> Result<String, &'static str> {
    // Because UUID implements the Deref trait, we can use Rusts Deref coercion
    // to convert Rockets UUID type to the uuid::Uuid type.
    let person = PEOPLE.get(&id).ok_or("Person not found")?;

    Ok(format!("We found: {}", person))
}

fn main() {
    rocket::ignite()
        .mount("/", routes![show])
        .launch();
}
