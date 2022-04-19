#[macro_use] extern crate rocket;

use rocket::form::Form;
use rocket::http::ContentType;
use rocket::local::blocking::Client;

#[derive(FromForm)]
struct Data<'r> {
    foo: &'r str,
    bar: &'r str,
    baz: &'r str,
}

#[rocket::post("/", data = "<form>")]
fn form(form: Form<Data<'_>>) -> String {
    form.foo.to_string() + form.bar + form.baz
}

#[test]
fn test_multipart_raw_strings_from_files() {
    let body = &[
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="foo"; filename="foo.txt""#,
        "Content-Type: text/plain",
        "",
        "hi",
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="bar"; filename="bar.txt""#,
        "Content-Type: text/plain",
        "",
        "hey",
        "--X-BOUNDARY",
        r#"Content-Disposition: form-data; name="baz"; filename="baz.txt""#,
        "Content-Type: text/plain",
        "",
        "bye",
        "--X-BOUNDARY--",
        "",
    ].join("\r\n");

    let client = Client::debug_with(rocket::routes![form]).unwrap();
    let response = client.post("/")
        .header("multipart/form-data; boundary=X-BOUNDARY".parse::<ContentType>().unwrap())
        .body(body)
        .dispatch();

    assert_eq!(response.into_string().unwrap(), "hiheybye");
}
