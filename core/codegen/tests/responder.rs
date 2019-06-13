#![feature(proc_macro_hygiene, decl_macro)]

use rocket::local::Client;
use rocket::response::Responder;
use rocket::http::{Status, ContentType, Cookie};

#[derive(Responder)]
pub enum Foo<'r> {
    First(String),
    #[response(status = 500)]
    Second(Vec<u8>),
    #[response(status = 404, content_type = "html")]
    Third {
        responder: &'r str,
        ct: rocket::http::ContentType,
    },
    #[response(status = 105)]
    Fourth {
        string: &'r str,
        ct: rocket::http::ContentType,
    },
}

#[test]
fn responder_foo() {
    let client = Client::new(rocket::ignite()).expect("valid rocket");
    let local_req = client.get("/");
    let req = local_req.inner();

    let mut response = Foo::First("hello".into())
        .respond_to(req)
        .expect("response okay");

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.body_string(), Some("hello".into()));

    let mut response = Foo::Second("just a test".into())
        .respond_to(req)
        .expect("response okay");

    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.content_type(), Some(ContentType::Binary));
    assert_eq!(response.body_string(), Some("just a test".into()));

    let mut response = Foo::Third { responder: "well, hi", ct: ContentType::JSON }
        .respond_to(req)
        .expect("response okay");

    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.content_type(), Some(ContentType::HTML));
    assert_eq!(response.body_string(), Some("well, hi".into()));

    let mut response = Foo::Fourth { string: "goodbye", ct: ContentType::JSON }
        .respond_to(req)
        .expect("response okay");

    assert_eq!(response.status(), Status::raw(105));
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    assert_eq!(response.body_string(), Some("goodbye".into()));
}

#[derive(Responder)]
#[response(content_type = "plain")]
pub struct Bar<'r> {
    responder: Foo<'r>,
    other: ContentType,
    third: Cookie<'static>,
    #[response(ignore)]
    _yet_another: String,
}

#[test]
fn responder_bar() {
    let client = Client::new(rocket::ignite()).expect("valid rocket");
    let local_req = client.get("/");
    let req = local_req.inner();

    let mut response = Bar {
        responder: Foo::Second("foo foo".into()),
        other: ContentType::HTML,
        third: Cookie::new("cookie", "here!"),
        _yet_another: "uh..hi?".into()
    }.respond_to(req).expect("response okay");

    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.body_string(), Some("foo foo".into()));
    assert_eq!(response.headers().get_one("Set-Cookie"), Some("cookie=here!"));
}

#[derive(Responder)]
#[response(content_type = "application/x-custom")]
pub struct Baz {
    responder: &'static str,
}

#[test]
fn responder_baz() {
    let client = Client::new(rocket::ignite()).expect("valid rocket");
    let local_req = client.get("/");
    let req = local_req.inner();

    let mut response = Baz { responder: "just a custom" }
        .respond_to(req)
        .expect("response okay");

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::new("application", "x-custom")));
    assert_eq!(response.body_string(), Some("just a custom".into()));
}
