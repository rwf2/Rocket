#![feature(plugin, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;
extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket_contrib::MsgPack;

#[derive(Serialize, Deserialize)]
struct Message {
    id: usize,
    contents: String
}

#[get("/<id>", format = "msgpack")]
fn get(id: usize) -> MsgPack<Message> {
    MsgPack(Message {
        id: id,
        contents: "Hello, world!".to_string(),
    })
}

#[post("/", data = "<data>", format = "msgpack")]
fn create(data: MsgPack<Message>) -> Result<String, ()> {
    Ok(data.into_inner().contents)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/message", routes![get, create])
}

fn main() {
    rocket().launch();
}
