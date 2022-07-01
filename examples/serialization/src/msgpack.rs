use rocket::serde::{Serialize, Deserialize, msgpack::MsgPack};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message<'r> {
    id: usize,
    message: &'r str
}

#[get("/<id>", format = "msgpack")]
fn get(id: usize) -> MsgPack<Message<'static>> {
    MsgPack(Message { id, message: "Hello, world!", })
}

#[post("/", data = "<data>", format = "msgpack")]
fn echo(data: MsgPack<Message<'_>>) -> &str {
    data.message
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("MessagePack", |rocket| async {
        rocket.mount("/msgpack", routes![echo, get])
    })
}
