use rocket;

use rocket::{get, routes};
use rocket::response::Responder;
use rocket::http::hyper::header::{ContentLanguage, qitem};
use language_tags::langtag;

#[derive(Responder)]
struct DerivedResponder {
    text: &'static str,
    lang_header: ContentLanguage,
}

#[get("/")]
fn index() -> DerivedResponder {
    DerivedResponder {
        text: "Fyr raketten af!",
        lang_header: ContentLanguage(vec![
            qitem(langtag!(da))
        ])
    }
}

#[test]
fn test_derive_reexports() {
    use rocket::local::blocking::Client;

    let rocket = rocket::ignite().mount("/", routes![index]);
    let client = Client::tracked(rocket).unwrap();

    let response = client.get("/").dispatch();
    let content_languages = response.headers().get_one("content-language");
    assert_eq!(content_languages, Some("da"));
    assert_eq!(response.into_string().unwrap(), "Fyr raketten af!");
}
