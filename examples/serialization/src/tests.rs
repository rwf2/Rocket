use rocket::local::blocking::Client;
use rocket::http::{Status, ContentType, Accept};
use rocket::serde::{Serialize, Deserialize, uuid::Uuid};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    id: Option<usize>,
    message: String
}

impl Message {
    fn new(message: impl Into<String>) -> Self {
        Message { message: message.into(), id: None }
    }

    fn with_id(mut self, id: usize) -> Self {
        self.id = Some(id);
        self
    }
}

#[test]
fn json_bad_get_put() {
    let client = Client::tracked(super::rocket()).unwrap();

    // Try to get a message with an ID that doesn't exist.
    let res = client.get("/json/99").header(ContentType::JSON).dispatch();
    assert_eq!(res.status(), Status::NotFound);

    let body = res.into_string().unwrap();
    assert!(body.contains("error"));
    assert!(body.contains("Resource was not found."));

    // Try to get a message with an invalid ID.
    let res = client.get("/json/hi").header(ContentType::JSON).dispatch();
    assert_eq!(res.status(), Status::NotFound);
    assert!(res.into_string().unwrap().contains("error"));

    // Try to put a message without a proper body.
    let res = client.put("/json/80").header(ContentType::JSON).dispatch();
    assert_eq!(res.status(), Status::BadRequest);

    // Try to put a message with a semantically invalid body.
    let res = client.put("/json/0")
        .header(ContentType::JSON)
        .body(r#"{ "dogs?": "love'em!" }"#)
        .dispatch();

    assert_eq!(res.status(), Status::UnprocessableEntity);

    // Try to put a message for an ID that doesn't exist.
    let res = client.put("/json/80")
        .json(&Message::new("hi"))
        .dispatch();

    assert_eq!(res.status(), Status::NotFound);
}

#[test]
fn json_post_get_put_get() {
    let client = Client::tracked(super::rocket()).unwrap();

    // Create/read/update/read a few messages.
    for id in 0..10 {
        let uri = format!("/json/{}", id);

        // Check that a message with doesn't exist.
        let res = client.get(&uri).header(ContentType::JSON).dispatch();
        assert_eq!(res.status(), Status::NotFound);

        // Add a new message. This should be ID 0.
        let message = Message::new(format!("Hello, JSON {}!", id));
        let res = client.post("/json").json(&message).dispatch();
        assert_eq!(res.status(), Status::Ok);

        // Check that the message exists with the correct contents.
        let res = client.get(&uri).header(Accept::JSON).dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Message>().unwrap(), message.with_id(id));

        // Change the message contents.
        let message = Message::new("Bye bye, world!");
        let res = client.put(&uri).json(&message).dispatch();
        assert_eq!(res.status(), Status::Ok);

        // Check that the message exists with the updated contents.
        let res = client.get(&uri).header(Accept::JSON).dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Message>().unwrap(), message.with_id(id));
    }
}

#[test]
fn msgpack_get() {
    let client = Client::tracked(super::rocket()).unwrap();
    let res = client.get("/msgpack/1").header(ContentType::MsgPack).dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.content_type(), Some(ContentType::MsgPack));

    // Check that the message is `[1, "Hello, world!"]`
    let msg = Message::new("Hello, world!").with_id(1);
    assert_eq!(res.into_msgpack::<Message>().unwrap(), msg);
}

#[test]
fn msgpack_post() {
    // Dispatch request with a message of `[2, "Goodbye, world!"]`.
    let client = Client::tracked(super::rocket()).unwrap();
    let res = client.post("/msgpack")
        .msgpack(&Message::new("Goodbye, world!").with_id(2))
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.into_string().unwrap(), "Goodbye, world!");
}

#[test]
fn uuid() {
    let client = Client::tracked(super::rocket()).unwrap();

    let pairs = &[
        ("7f205202-7ba1-4c39-b2fc-3e630722bf9f", "We found: Lacy"),
        ("4da34121-bc7d-4fc1-aee6-bf8de0795333", "We found: Bob"),
        ("ad962969-4e3d-4de7-ac4a-2d86d6d10839", "We found: George"),
        ("e18b3a5c-488f-4159-a240-2101e0da19fd",
            "Missing person for UUID: e18b3a5c-488f-4159-a240-2101e0da19fd"),
    ];

    for (uuid, response) in pairs {
        let uuid = Uuid::parse_str(uuid).unwrap();
        let res = client.get(uri!(super::uuid::people(uuid))).dispatch();
        assert_eq!(res.into_string().unwrap(), *response);
    }

    let res = client.get("/people/not-a-uuid").dispatch();
    assert_eq!(res.status(), Status::NotFound);
}
