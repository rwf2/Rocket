#[macro_use]
extern crate rocket_community as rocket;

use rocket::http::{ContentType, Status};
use rocket::{data::Limits, form::Form};
use rocket::{Build, Config, Rocket};
use ubyte::{ByteUnit, ToByteUnit};

#[derive(FromForm)]
struct Data<'r> {
    foo: Option<&'r str>,
}

#[rocket::post("/", data = "<form>")]
fn form<'r>(form: Form<Data<'r>>) -> &'r str {
    form.foo.unwrap_or("missing")
}

fn rocket_with_form_data_limit(limit: ByteUnit) -> Rocket<Build> {
    rocket::custom(Config {
        limits: Limits::default().limit("data-form", limit),
        ..Config::debug_default()
    })
    .mount("/", routes![form])
}

#[test]
fn test_multipart_limit() {
    use rocket::local::blocking::Client;

    let body = &[
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="foo"; filename="foo.txt""#,
        "Content-Type: text/plain",
        "",
        "hi",
        "--X-BOUNDARY--",
        "",
    ]
    .join("\r\n");

    let client = Client::debug(rocket_with_form_data_limit(body.len().bytes())).unwrap();
    let response = client
        .post("/")
        .header(
            "multipart/form-data; boundary=X-BOUNDARY"
                .parse::<ContentType>()
                .unwrap(),
        )
        .body(body)
        .dispatch();

    assert_eq!(response.into_string().unwrap(), "hi");

    let client = Client::debug(rocket_with_form_data_limit(body.len().bytes() - 1)).unwrap();
    let response = client
        .post("/")
        .header(
            "multipart/form-data; boundary=X-BOUNDARY"
                .parse::<ContentType>()
                .unwrap(),
        )
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::PayloadTooLarge);
}
