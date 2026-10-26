use super::rocket;

use rocket::http::{ContentType, RawStr, Status};
use rocket::local::blocking::Client;

#[test]
fn test_empty_form() {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::HTML));

    let body = response.into_string().unwrap();
    assert!(body.contains("name=\"account.name\""));
    assert!(body.contains("name=\"submission.file\""));
    assert!(body.contains("name=\"submission.category\" value=\"Biology\""));
}

#[test]
fn test_invalid_form() {
    let client = Client::tracked(rocket()).unwrap();
    let name = RawStr::new("<b>'\"&</b>").percent_encode();
    let text = RawStr::new("</textarea><script>x</script>").percent_encode();
    let body = format!("account.name={name}&account.email=invalid&\
        submission.abstract={text}&submission.rights=Reserved&\
        submission.category=Biology&submission.category=CS&submission.ready=yes");

    let response = client.post("/").header(ContentType::Form).body(body).dispatch();

    assert_eq!(response.status(), Status::UnprocessableEntity);
    assert_eq!(response.content_type(), Some(ContentType::HTML));
    let body = response.into_string().unwrap();
    let body = body.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(body.contains("invalid email address"));
    assert!(body.contains("value='&lt;b&gt;&#39;&quot;&amp;&lt;/b&gt;'"));
    assert!(body.contains("&lt;/textarea&gt;&lt;script&gt;x&lt;/script&gt;"));
    assert!(body.contains("name=\"submission.category\" value=\"Biology\" checked"));
    assert!(body.contains("name=\"submission.category\" value=\"CS\" checked"));
    assert!(body.contains("name=\"submission.category\" value=\"Chemistry\" >"));
    assert!(body.contains("name=\"submission.ready\" value=\"yes\" checked"));
    assert!(body.contains("<option value=\"Reserved\" selected"));
}

#[test]
fn test_valid_form() {
    use std::fmt::Write;

    let client = Client::tracked(rocket()).unwrap();
    let mut body = String::new();
    for (name, value) in [
        ("account.name", "<b>Alice</b>"),
        ("account.email", "alice@example.com"),
        ("account.password.first", "password"),
        ("account.password.second", "password"),
        ("submission.title", "A paper"),
        ("submission.date", "2026-10-26"),
        ("submission.abstract", "An abstract"),
        ("submission.category", "Biology"),
        ("submission.category", "CS"),
        ("submission.rights", "Reserved"),
        ("submission.ready", "yes"),
    ] {
        write!(body,
            "--FORM\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        ).unwrap();
    }

    body.push_str(concat!(
        "--FORM\r\n",
        "Content-Disposition: form-data; name=\"submission.file\"; filename=\"paper.pdf\"\r\n",
        "Content-Type: application/pdf\r\n\r\n",
        "%PDF-1.7\r\n",
        "--FORM--\r\n",
    ));

    let response = client.post("/")
        .header("multipart/form-data; boundary=FORM".parse::<ContentType>().unwrap())
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::HTML));
    let body = response.into_string().unwrap();
    assert!(body.contains("<h1>Success!</h1>"));
    assert!(body.contains("<strong>account.name</strong> - [&lt;b&gt;Alice&lt;/b&gt;]"));
    assert!(body.contains("<strong>submission.category</strong> - [Biology, CS]"));
}
