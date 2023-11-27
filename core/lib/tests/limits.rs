#[macro_use]
extern crate rocket;

use rocket::form::Form;

#[post("/", data = "<form>")]
fn index(form: Form<String>) -> String {
    form.into_inner()
}

mod limits_tests {
    use rocket::data::Limits;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;
    use rocket::{Build, Rocket};

    fn rocket_with_forms_limit(limit: u64) -> Rocket<Build> {
        let mut config = rocket::Config::debug_default();
        config.limits = Limits::default().limit("form", limit.into());
        rocket::custom(config).mount("/", routes![super::index])
    }

    #[test]
    fn large_enough() {
        let client = Client::debug(rocket_with_forms_limit(128)).unwrap();
        let response = client
            .post("/")
            .body("value=Hello+world")
            .header(ContentType::Form)
            .dispatch();

        assert_eq!(response.into_string(), Some("Hello world".into()));
    }

    #[test]
    fn just_large_enough() {
        let client = Client::debug(rocket_with_forms_limit(17)).unwrap();
        let response = client
            .post("/")
            .body("value=Hello+world")
            .header(ContentType::Form)
            .dispatch();

        assert_eq!(response.into_string(), Some("Hello world".into()));
    }

    #[test]
    fn much_too_small() {
        let client = Client::debug(rocket_with_forms_limit(4)).unwrap();
        let response = client
            .post("/")
            .body("value=Hello+world")
            .header(ContentType::Form)
            .dispatch();

        assert_eq!(response.status(), Status::PayloadTooLarge);
    }

    #[test]
    fn contracted() {
        let client = Client::debug(rocket_with_forms_limit(10)).unwrap();
        let response = client
            .post("/")
            .body("value=Hello+world")
            .header(ContentType::Form)
            .dispatch();

        assert_eq!(response.status(), Status::PayloadTooLarge);
    }
}
