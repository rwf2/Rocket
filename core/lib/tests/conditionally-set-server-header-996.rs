#[macro_use] extern crate rocket;

use rocket::http::Header;

#[derive(Responder)]
struct HeaderOnly((), Header<'static>);

#[get("/do_not_overwrite")]
fn do_not_overwrite() -> HeaderOnly {
    HeaderOnly((), Header::new("Server", "Test"))
}

#[get("/use_default")]
fn use_default() { }

mod conditionally_set_server_header {
    use super::*;
    use rocket::local::blocking::Client;

    #[test]
    fn do_not_overwrite_server_header() {
        let client = Client::debug_with(routes![do_not_overwrite, use_default]).unwrap();

        let response = client.get("/do_not_overwrite").dispatch();
        let server = response.headers().get_one("Server");
        assert_eq!(server, Some("Test"));

        let response = client.get("/use_default").dispatch();
        let server = response.headers().get_one("Server");
        assert_eq!(server, Some("Rocket"));

        // Now with a special `Ident`.

        let config = rocket::Config {
            ident: rocket::config::Ident::try_new("My Special Server").unwrap(),
            ..rocket::Config::debug_default()
        };

        let rocket = rocket::custom(config)
            .mount("/", routes![do_not_overwrite, use_default]);

        let client = Client::debug(rocket).unwrap();

        let response = client.get("/do_not_overwrite").dispatch();
        let server = response.headers().get_one("Server");
        assert_eq!(server, Some("Test"));

        let response = client.get("/use_default").dispatch();
        let server = response.headers().get_one("Server");
        assert_eq!(server, Some("My Special Server"));
    }
}
