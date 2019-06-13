#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;

use std::collections::HashMap;
use rocket_contrib::uuid::Uuid;

#[cfg(test)] mod tests;

lazy_static! {
    // A small people lookup table for the sake of this example. In a real
    // application this could be a database lookup. Notice that we use the
    // uuid::Uuid type here and not the rocket_contrib::uuid::Uuid type.
    static ref PEOPLE: HashMap<uuid::Uuid, &'static str> = {
        let mut m = HashMap::new();
        let lacy_id = uuid::Uuid::parse_str("7f205202-7ba1-4c39-b2fc-3e630722bf9f").unwrap();
        let bob_id = uuid::Uuid::parse_str("4da34121-bc7d-4fc1-aee6-bf8de0795333").unwrap();
        let george_id = uuid::Uuid::parse_str("ad962969-4e3d-4de7-ac4a-2d86d6d10839").unwrap();
        m.insert(lacy_id, "Lacy");
        m.insert(bob_id, "Bob");
        m.insert(george_id, "George");
        m
    };
}

#[get("/people/<id>")]
fn people(id: Uuid) -> Result<String, String> {
    // Because Uuid implements the Deref trait, we use Deref coercion to convert
    // rocket_contrib::uuid::Uuid to uuid::Uuid.
    Ok(PEOPLE.get(&id)
        .map(|person| format!("We found: {}", person))
        .ok_or_else(|| format!("Person not found for UUID: {}", id))?)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![people])
}

fn main() {
    rocket().launch();
}
