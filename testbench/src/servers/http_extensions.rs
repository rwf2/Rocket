//! Test that HTTP method extensions unlike POST or GET work.

use crate::prelude::*;

use rocket::http::Method;

#[route(PROPFIND, uri = "/")]
fn route() -> &'static str {
    "Hello, World!"
}

pub fn test_http_extensions() -> Result<()> {
    let server = spawn! {
        Rocket::default().mount("/", routes![route])
    }?;

    let client = Client::default();
    let response = client.request(&server, Method::PropFind, "/")?.send()?;
    assert_eq!(response.status(), 200);
    assert_eq!(response.text()?, "Hello, World!");

    // Make sure that verbs outside of extensions are marked as errors
    let res = client.request(&server, "BAKEMEACOOKIE", "/")?.send()?;
    assert_eq!(res.status(), 400);

    Ok(())
}

register!(test_http_extensions);
