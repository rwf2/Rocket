#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket_contrib::MsgPack;
use rocket::State;
use std::collections::HashMap;
use std::sync::Mutex;

// The type to represent the ID of a message.
type ID = usize;

// We're going to store all of the messages here. No need for a DB.
type MessageMap = Mutex<HashMap<ID, String>>;

#[derive(Serialize, Deserialize)]
struct Message {
    id: Option<ID>,
    contents: String
}

// TODO: This example can be improved by using `route` with multiple HTTP verbs.
#[post("/<id>", format = "application/msgpack", data = "<message>")]
fn new(id: ID, message: MsgPack<Message>, map: State<MessageMap>) -> MsgPack<HashMap<&'static str, &'static str>> {
    let mut hashmap = map.lock().expect("map lock.");
    if hashmap.contains_key(&id) {
        let response = [("status", "error"), ("reason", "ID exists. Try put.")].iter().cloned()
            .collect::<HashMap<&'static str, &'static str>>();
        MsgPack(response)
    } else {
        hashmap.insert(id, message.0.contents);
        let response = [("status", "ok")].iter().cloned().collect::<HashMap<&'static str, &'static str>>();
        MsgPack(response)
    }
}

#[put("/<id>", format = "application/msgpack", data = "<message>")]
fn update(id: ID, message: MsgPack<Message>, map: State<MessageMap>) -> Option<MsgPack<HashMap<&'static str, &'static str>>> {
    let mut hashmap = map.lock().unwrap();
    if hashmap.contains_key(&id) {
        hashmap.insert(id, message.0.contents);
        let response = [("status", "ok")].iter().cloned().collect::<HashMap<&'static str, &'static str>>();
        Some(MsgPack(response))
    } else {
        None
    }
}

#[get("/<id>", format = "application/msgpack")]
fn get(id: ID, map: State<MessageMap>) -> Option<MsgPack<Message>> {
    let hashmap = map.lock().unwrap();
    hashmap.get(&id).map(|contents| {
        MsgPack(Message {
            id: Some(id),
            contents: contents.clone()
        })
    })
}

#[error(404)]
fn not_found() -> MsgPack<HashMap<&'static str, &'static str>> {
    let response = [("status", "error"), ("reason", "Resource was not found.")].iter().cloned()
        .collect::<HashMap<&'static str, &'static str>>();
    MsgPack(response)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/message", routes![new, update, get])
        .catch(errors![not_found])
        .manage(Mutex::new(HashMap::<ID, String>::new()))
}

fn main() {
    rocket().launch();
}
