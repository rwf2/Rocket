use rocket::http::ProxyProto;

#[macro_use]
extern crate rocket;

#[get("/")]
fn inspect_proto(proto: Option<ProxyProto>) -> String {
    proto
        .map(|proto| match proto {
            ProxyProto::Http => "http".to_owned(),
            ProxyProto::Https => "https".to_owned(),
            ProxyProto::Unknown(s) => s.to_string(),
        })
        .unwrap_or("<none>".to_owned())
}

mod tests {
    use rocket::{Rocket, Build, Route};
    use rocket::http::Header;
    use rocket::local::blocking::Client;
    use rocket::figment::Figment;

    fn routes() -> Vec<Route> {
        routes![super::inspect_proto]
    }

    fn rocket_with_proto_header(header: Option<&'static str>) -> Rocket<Build> {
        let mut config = rocket::Config::debug_default();
        config.proxy_proto_header = header.map(|h| h.into());
        rocket::custom(config).mount("/", routes())
    }

    #[test]
    fn check_proxy_proto_header_works() {
        let rocket = rocket_with_proto_header(Some("X-Url-Scheme"));
        let client = Client::debug(rocket).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Forwarded-Proto", "https"))
            .header(Header::new("X-Url-Scheme", "http"))
            .dispatch();

        assert_eq!(response.into_string().unwrap(), "http");

        let response = client.get("/")
            .header(Header::new("X-Url-Scheme", "https"))
            .dispatch();

        assert_eq!(response.into_string().unwrap(), "https");

        let response = client.get("/").dispatch();
        assert_eq!(response.into_string().unwrap(), "<none>");
    }

    #[test]
    fn check_proxy_proto_header_works_again() {
        let client = Client::debug(rocket_with_proto_header(Some("x-url-scheme"))).unwrap();
        let response = client
            .get("/")
            .header(Header::new("X-Url-Scheme", "https"))
            .dispatch();

        assert_eq!(response.into_string().unwrap(), "https");

        let config = Figment::from(rocket::Config::debug_default())
            .merge(("proxy_proto_header", "x-url-scheme"));

        let client = Client::debug(rocket::custom(config).mount("/", routes())).unwrap();
        let response = client
            .get("/")
            .header(Header::new("X-url-Scheme", "https"))
            .dispatch();

        assert_eq!(response.into_string().unwrap(), "https");
    }

    #[test]
    fn check_default_proxy_proto_header_works() {
        let client = Client::debug_with(routes()).unwrap();
        let response = client
            .get("/")
            .header(Header::new("X-Forwarded-Proto", "https"))
            .dispatch();

        assert_eq!(response.into_string(), Some("<none>".into()));
    }

    #[test]
    fn check_no_proxy_proto_header_works() {
        let client = Client::debug(rocket_with_proto_header(None)).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Forwarded-Proto", "https"))
            .dispatch();

        assert_eq!(response.into_string(), Some("<none>".into()));

        let config =
            Figment::from(rocket::Config::debug_default()).merge(("proxy_proto_header", false));

        let client = Client::debug(rocket::custom(config).mount("/", routes())).unwrap();
        let response = client
            .get("/")
            .header(Header::new("X-Forwarded-Proto", "https"))
            .dispatch();

        assert_eq!(response.into_string(), Some("<none>".into()));

        let config = Figment::from(rocket::Config::debug_default())
            .merge(("proxy_proto_header", "x-forwarded-proto"));

        let client = Client::debug(rocket::custom(config).mount("/", routes())).unwrap();
        let response = client
            .get("/")
            .header(Header::new("x-Forwarded-Proto", "https"))
            .dispatch();

        assert_eq!(response.into_string(), Some("https".into()));
    }
}
