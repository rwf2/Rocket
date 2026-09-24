#![cfg(feature = "xml")]

#[macro_use] extern crate rocket;

use rocket::http::ContentType;
use rocket::serde::{Serialize, Deserialize, xml::Xml};

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
