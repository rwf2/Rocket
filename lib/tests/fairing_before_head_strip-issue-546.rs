#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::response::content;

#[head("/")]
fn index() -> content::Json<&'static str> {
    content::Json("{ 'test': 'dont strip before fairing' }")
}

#[get("/")]
fn auto() -> content::Json<&'static str> {
    index()
}

mod fairing_before_head_strip {
    use super::*;
    use rocket::fairing::AdHoc;
    use rocket::http::Method;
    use rocket::local::Client;
    use rocket::http::Status;

    #[test]
    fn not_empty_before_fairing() {
        let rocket = rocket::ignite()
            .mount("/", routes![index])
            .attach(AdHoc::on_response(|req, res| {
                if req.method() != Method::Head || res.body_string() != Some("{ 'test': 'dont strip before fairing' }".into()) {
                    res.set_status(Status::InternalServerError);
                }
            }));

        let client = Client::new(rocket).unwrap();
        let response = client.head("/").dispatch();

        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn not_empty_before_fairing_autohandeled() {
        let rocket = rocket::ignite()
            .mount("/", routes![auto])
            .attach(AdHoc::on_response(|req, res| {
                if req.method() != Method::Head || res.body_string() != Some("{ 'test': 'dont strip before fairing' }".into()) {
                    res.set_status(Status::InternalServerError);
                }
            }));

        let client = Client::new(rocket).unwrap();
        let response = client.head("/").dispatch();

        assert_eq!(response.status(), Status::Ok);
    }
}
