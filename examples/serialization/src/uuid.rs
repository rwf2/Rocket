use std::collections::HashMap;

use rocket::State;
use rocket::serde::uuid::Uuid;

// A small people mapping in managed state for the sake of this example. In a
// real application this would be a database.
struct People(HashMap<Uuid, &'static str>);

#[get("/people/<id>")]
fn people(id: Uuid, people: &State<People>) -> Result<String, String> {
    people.0.get(&id)
        .map(|person| format!("We found: {}", person))
        .ok_or_else(|| format!("Missing person for UUID: {}", id))
}

pub fn stage() -> rocket::fairing::AdHoc {
    // Seed the "database".
    let mut map = HashMap::new();
    map.insert("7f205202-7ba1-4c39-b2fc-3e630722bf9f".parse().unwrap(), "Lacy");
    map.insert("4da34121-bc7d-4fc1-aee6-bf8de0795333".parse().unwrap(), "Bob");
    map.insert("ad962969-4e3d-4de7-ac4a-2d86d6d10839".parse().unwrap(), "George");

    rocket::fairing::AdHoc::on_ignite("UUID", |rocket| async {
        rocket
            .manage(People(map))
            .mount("/", routes![people])
    })
}
