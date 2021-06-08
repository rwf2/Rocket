use rocket::{Rocket, Build};
use rocket::{fairing::AdHoc, http::ContentType, local::blocking::Client};

#[rocket::post("/", data = "<_data>", format = "json")]
fn index(_data: rocket::Data<'_>) -> &'static str { "json" }

#[rocket::post("/", data = "<_data>", rank = 2)]
fn other_index(_data: rocket::Data<'_>) -> &'static str { "other" }

fn rocket() -> Rocket<Build> {
    rocket::build()
        .mount("/", rocket::routes![index, other_index])
        .attach(AdHoc::on_request("Change CT", |req, _| Box::pin(async move {
            let need_ct = req.content_type().is_none();
            if req.uri().path().starts_with("/add") {
                req.set_uri(rocket::uri!(index));
                if need_ct { req.add_header(ContentType::JSON); }
            } else if need_ct {
                req.replace_header(ContentType::JSON);
            }
        })))
}

#[test]
fn check_fairing_changes_content_type() {
    let client = Client::debug(rocket()).unwrap();
    let response = client.post("/").header(ContentType::PNG).dispatch();
    assert_eq!(response.into_string().unwrap(), "other");

    let response = client.post("/").dispatch();
    assert_eq!(response.into_string().unwrap(), "json");

    let response = client.post("/add").dispatch();
    assert_eq!(response.into_string().unwrap(), "json");

    let response = client.post("/add").header(ContentType::HTML).dispatch();
    assert_eq!(response.into_string().unwrap(), "other");
}
