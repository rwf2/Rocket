#![cfg(feature = "xml")]

#[macro_use] extern crate rocket;

use rocket::http::{ContentType, Status};
use rocket::serde::{Serialize, Deserialize, xml::{self, Xml}};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Task {
    id: usize,
    complete: bool,
    text: String,
}

#[post("/echo", format = "xml", data = "<task>")]
fn echo(task: Xml<Task>) -> Xml<Task> { task }

fn task() -> Task {
    Task { id: 10, complete: false, text: "write <tests> & docs".into() }
}

#[test]
fn blocking_xml_works() {
    use rocket::local::blocking::Client;

    let client = Client::debug_with(routes![echo]).unwrap();
    let request = client.post("/echo").xml(&task());
    assert_eq!(request.content_type(), Some(&ContentType::XML));

    let response = request.dispatch();
    assert_eq!(response.content_type(), Some(ContentType::XML));
    assert_eq!(response.into_xml::<Task>(), Some(task()));

    #[derive(Debug, Deserialize)]
    #[serde(crate = "rocket::serde")]
    struct Missing { #[allow(dead_code)] missing: String }

    let response = client.post("/echo").xml(&task()).dispatch();
    assert!(response.into_xml::<Missing>().is_none());
}

#[async_test]
async fn async_xml_works() {
    use rocket::local::asynchronous::Client;

    let client = Client::debug_with(routes![echo]).await.unwrap();
    let response = client.post("/echo").xml(&task()).dispatch().await;
    assert_eq!(response.into_xml::<Task>().await, Some(task()));
}

#[test]
fn trailing_content_is_rejected() {
    use rocket::local::blocking::Client;

    let task = "<Task><id>1</id><complete>true</complete><text>hi</text></Task>";
    for rest in ["\n ", "<!-- comment -->", "\n<?pi x?>\n"] {
        assert!(xml::from_str::<Task>(&format!("{task}{rest}")).is_ok(), "{rest:?}");
    }

    for rest in [task, "text", "</Task>"] {
        assert!(xml::from_str::<Task>(&format!("{task}{rest}")).is_err(), "{rest:?}");
    }

    let sequence: Vec<String> = xml::from_str("<a>hi</a><a>yo</a>").unwrap();
    assert_eq!(sequence, ["hi", "yo"]);

    let client = Client::debug_with(routes![echo]).unwrap();
    let request = client.post("/echo").header(ContentType::XML).body(format!("{task}{task}"));
    assert_eq!(request.dispatch().status(), Status::BadRequest);
}
