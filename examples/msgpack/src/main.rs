#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket_contrib::msgpack::MsgPack;

#[derive(Serialize, Deserialize)]
struct Message<'r> {
    id: usize,
    contents: &'r str
}

#[get("/<id>", format = "msgpack")]
fn get(id: usize) -> MsgPack<Message<'static>> {
    MsgPack(Message { id: id, contents: "Hello, world!", })
}

#[post("/", data = "<data>", format = "msgpack")]
fn create(data: MsgPack<Message<'_>>) -> String {
    data.contents.to_string()
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/message", routes![get, create])
}

fn main() {
    rocket().launch();
}
