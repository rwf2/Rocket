use rocket::local::asynchronous::Client;
use rocket::http::{Status, ContentType, Cookie};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::http::Accept;

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

#[rocket::async_test]
async fn responder_foo() {
    let client = Client::debug_with(vec![]).await.expect("valid rocket");
    let local_req = client.get("/");
    let req = local_req.inner();

    let mut r = Foo::First("hello".into())
        .respond_to(req)
        .expect("response okay");

    assert_eq!(r.status(), Status::Ok);
    assert_eq!(r.content_type(), Some(ContentType::Plain));
    assert_eq!(r.body_mut().to_string().await.unwrap(), "hello");

    let mut r = Foo::Second("just a test".into())
        .respond_to(req)
        .expect("response okay");

    assert_eq!(r.status(), Status::InternalServerError);
    assert_eq!(r.content_type(), Some(ContentType::Binary));
    assert_eq!(r.body_mut().to_string().await.unwrap(), "just a test");

    let mut r = Foo::Third { responder: "well, hi", ct: ContentType::JSON }
        .respond_to(req)
        .expect("response okay");

    assert_eq!(r.status(), Status::NotFound);
    assert_eq!(r.content_type(), Some(ContentType::HTML));
    assert_eq!(r.body_mut().to_string().await.unwrap(), "well, hi");

    let mut r = Foo::Fourth { string: "goodbye", ct: ContentType::JSON }
        .respond_to(req)
        .expect("response okay");

    assert_eq!(r.status().code, 105);
    assert_eq!(r.content_type(), Some(ContentType::JSON));
    assert_eq!(r.body_mut().to_string().await.unwrap(), "goodbye");
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

#[rocket::async_test]
async fn responder_bar() {
    let client = Client::debug_with(vec![]).await.expect("valid rocket");
    let local_req = client.get("/");
    let req = local_req.inner();

    let mut r = Bar {
        responder: Foo::Second("foo foo".into()),
        other: ContentType::HTML,
        third: Cookie::new("cookie", "here!"),
        _yet_another: "uh..hi?".into()
    }.respond_to(req).expect("response okay");

    assert_eq!(r.status(), Status::InternalServerError);
    assert_eq!(r.content_type(), Some(ContentType::Plain));
    assert_eq!(r.body_mut().to_string().await.unwrap(), "foo foo");
    assert_eq!(r.headers().get_one("Set-Cookie"), Some("cookie=here!"));
}

#[derive(Responder)]
#[response(content_type = "application/x-custom")]
pub struct Baz {
    responder: &'static str,
}

#[rocket::async_test]
async fn responder_baz() {
    let client = Client::debug_with(vec![]).await.expect("valid rocket");
    let local_req = client.get("/");
    let req = local_req.inner();

    let mut r = Baz { responder: "just a custom" }
        .respond_to(req)
        .expect("response okay");

    assert_eq!(r.status(), Status::Ok);
    assert_eq!(r.content_type(), Some(ContentType::new("application", "x-custom")));
    assert_eq!(r.body_mut().to_string().await.unwrap(), "just a custom");
}

// The bounds `Json<T>: Responder, E: Responder` will be added to the generated
// implementation. This would fail to compile otherwise.
#[derive(Responder)]
enum MyResult<'a, T, E, H1, H2> {
    Ok(Json<T>),
    #[response(status = 404)]
    Err(E, H1, H2),
    #[response(status = 500)]
    Other(&'a str),
}

#[rocket::async_test]
async fn generic_responder() {
    let client = Client::debug_with(vec![]).await.expect("valid rocket");
    let local_req = client.get("/");
    let req = local_req.inner();

    let v: MyResult<_, (), ContentType, Cookie<'static>> = MyResult::Ok(Json("hi"));
    let mut r = v.respond_to(req).unwrap();
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(r.content_type().unwrap(), ContentType::JSON);
    assert_eq!(r.body_mut().to_string().await.unwrap(), "\"hi\"");

    let v: MyResult<(), &[u8], _, _> = MyResult::Err(&[7, 13, 23], ContentType::JPEG, Accept::Text);
    let mut r = v.respond_to(req).unwrap();
    assert_eq!(r.status(), Status::NotFound);
    assert_eq!(r.content_type().unwrap(), ContentType::JPEG);
    assert_eq!(r.body_mut().to_bytes().await.unwrap(), vec![7, 13, 23]);

    let v: MyResult<(), &[u8], ContentType, Accept> = MyResult::Other("beep beep");
    let mut r = v.respond_to(req).unwrap();
    assert_eq!(r.status(), Status::InternalServerError);
    assert_eq!(r.content_type().unwrap(), ContentType::Text);
    assert_eq!(r.body_mut().to_string().await.unwrap(), "beep beep");
}
