#[macro_use] extern crate rocket;

use std::collections::HashMap;

use rocket::State;
use rocket_contrib::uuid::Uuid;
use rocket_contrib::uuid::uuid_crate as uuid;

#[cfg(test)] mod tests;

// A small people mapping in managed state for the sake of this example. In a
// real application this would be a database. Notice that we use the uuid::Uuid
// type here and not the rocket_contrib::uuid::Uuid type.
struct People(HashMap<uuid::Uuid, &'static str>);

#[get("/people/<id>")]
fn people(id: Uuid, people: State<People>) -> Result<String, String> {
    // Because Uuid implements the Deref trait, we use Deref coercion to convert
    // rocket_contrib::uuid::Uuid to uuid::Uuid.
    Ok(people.0.get(&id)
        .map(|person| format!("We found: {}", person))
        .ok_or_else(|| format!("Person not found for UUID: {}", id))?)
}

#[launch]
fn rocket() -> rocket::Rocket {
    let mut map = HashMap::new();
    map.insert("7f205202-7ba1-4c39-b2fc-3e630722bf9f".parse().unwrap(), "Lacy");
    map.insert("4da34121-bc7d-4fc1-aee6-bf8de0795333".parse().unwrap(), "Bob");
    map.insert("ad962969-4e3d-4de7-ac4a-2d86d6d10839".parse().unwrap(), "George");

    rocket::ignite()
        .manage(People(map))
        .mount("/", routes![people])
}
