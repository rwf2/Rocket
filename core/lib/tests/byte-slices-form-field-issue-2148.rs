#[macro_use] extern crate rocket;

use std::str::from_utf8;

use rocket::form::Form;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;

#[derive(FromForm)]
struct DataForm<'r> {
    foo: &'r [u8],
    bar: &'r [u8],
}

#[post("/", data = "<form>")]
fn form(form: Form<DataForm<'_>>) -> String {
    from_utf8(form.foo).unwrap().to_string() + from_utf8(form.bar).unwrap()
}

#[test]
fn test_from_form_fields_of_multipart_files_into_byte_slices() {
    let body = &[
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="foo"; filename="foo.txt""#,
        "Content-Type: text/plain",
        "",
        "start>",
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="foo"; filename="foo2.txt""#,
        "Content-Type: text/plain",
        "",
        "second-start...",
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="bar"; filename="bar.txt""#,
        "Content-Type: text/plain",
        "",
        "<finish",
        "--X-BOUNDARY--",
        "",
    ].join("\r\n");

    let client = Client::debug_with(routes![form]).unwrap();
    let response = client.post("/")
        .header("multipart/form-data; boundary=X-BOUNDARY".parse::<ContentType>().unwrap())
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), "start><finish");
}

#[test]
fn test_from_form_fields_of_values_into_byte_slices() {
    let client = Client::debug_with(routes![form]).unwrap();
    let response = client.post("/")
        .header(ContentType::Form)
        .body(format!("bar={}&foo={}", "...finish", "start..."))
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().unwrap(), "start......finish");
}
