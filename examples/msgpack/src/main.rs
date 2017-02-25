#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket_contrib::MsgPack;

#[derive(Serialize, Deserialize)]
struct Message {
    id: usize,
    contents: String
}

#[get("/<id>", format = "application/msgpack")]
fn get(id: usize) -> MsgPack<Message> {
    MsgPack(Message {
        id: id,
        contents: "Hello, world!".to_string(),
    })
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/message", routes![get])
}

fn main() {
    rocket().launch();
}
