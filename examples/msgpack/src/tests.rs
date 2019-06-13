use crate::rocket;
use rocket::local::Client;
use rocket::http::{Status, ContentType};

#[derive(Serialize, Deserialize)]
struct Message {
    id: usize,
    contents: String
}

#[test]
fn msgpack_get() {
    let client = Client::new(rocket()).unwrap();
    let mut res = client.get("/message/1").header(ContentType::MsgPack).dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.content_type(), Some(ContentType::MsgPack));

    // Check that the message is `[1, "Hello, world!"]`
    assert_eq!(&res.body_bytes().unwrap(),
               &[146, 1, 173, 72, 101, 108, 108, 111, 44, 32, 119, 111, 114, 108, 100, 33]);
}

#[test]
fn msgpack_post() {
    // Dispatch request with a message of `[2, "Goodbye, world!"]`.
    let client = Client::new(rocket()).unwrap();
    let mut res = client.post("/message")
        .header(ContentType::MsgPack)
        .body(&[146, 2, 175, 71, 111, 111, 100, 98, 121, 101, 44, 32, 119, 111, 114, 108, 100, 33])
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("Goodbye, world!".into()));
}
