#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

#[post("/", format = "application/json")]
fn specified() -> &'static str {
    "specified"
}

#[post("/", rank = 2)]
fn unspecified() -> &'static str {
    "unspecified"
}

#[post("/", format = "application/json")]
fn specified_json() -> &'static str {
    "specified_json"
}

#[post("/", format = "text/html")]
fn specified_html() -> &'static str {
    "specified_html"
}

mod tests {
    use super::*;

    use rocket::Rocket;
    use rocket::local::Client;
    use rocket::http::{Status, ContentType};

    fn rocket() -> Rocket {
        rocket::ignite()
            .mount("/first", routes![specified, unspecified])
            .mount("/second", routes![specified_json, specified_html])
    }

    macro_rules! check_dispatch {
        ($mount:expr, $ct:expr, $body:expr) => (
            let client = Client::new(rocket()).unwrap();
            let mut req = client.post($mount);
            let ct: Option<ContentType> = $ct;
            if let Some(ct) = ct {
                req.add_header(ct);
            }

            let mut response = req.dispatch();
            let body_str = response.body_string();
            let body: Option<&'static str> = $body;
            match body {
                Some(string) => assert_eq!(body_str, Some(string.to_string())),
                None => assert_eq!(response.status(), Status::NotFound)
            }
        )
    }

    #[test]
    fn exact_match_or_forward() {
        check_dispatch!("/first", Some(ContentType::JSON), Some("specified"));
        check_dispatch!("/first", None, Some("unspecified"));
        check_dispatch!("/first", Some(ContentType::HTML), Some("unspecified"));
    }

    #[test]
    fn exact_match_or_none() {
        check_dispatch!("/second", Some(ContentType::JSON), Some("specified_json"));
        check_dispatch!("/second", Some(ContentType::HTML), Some("specified_html"));
        check_dispatch!("/second", Some(ContentType::CSV), None);
        check_dispatch!("/second", None, None);
    }
}
