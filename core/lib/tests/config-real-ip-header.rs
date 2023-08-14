#[macro_use] extern crate rocket;

#[get("/")]
fn inspect_ip(ip: Option<std::net::IpAddr>) -> String {
    ip.map(|ip| ip.to_string()).unwrap_or("<none>".into())
}

mod tests {
    use rocket::{Rocket, Build, Route};
    use rocket::local::blocking::Client;
    use rocket::figment::Figment;
    use rocket::http::Header;

    fn routes() -> Vec<Route> {
        routes![super::inspect_ip]
    }

    fn rocket_with_custom_ip_header(header: Option<&'static str>) -> Rocket<Build> {
        let mut config = rocket::Config::debug_default();
        config.ip_header = header.map(|h| h.into());
        rocket::custom(config).mount("/", routes())
    }

    #[test]
    fn check_real_ip_header_works() {
        let client = Client::debug(rocket_with_custom_ip_header(Some("IP"))).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Real-IP", "1.2.3.4"))
            .header(Header::new("IP", "8.8.8.8"))
            .dispatch();

        assert_eq!(response.into_string(), Some("8.8.8.8".into()));

        let response = client.get("/")
            .header(Header::new("IP", "1.1.1.1"))
            .dispatch();

        assert_eq!(response.into_string(), Some("1.1.1.1".into()));

        let response = client.get("/").dispatch();
        assert_eq!(response.into_string(), Some("<none>".into()));
    }

    #[test]
    fn check_real_ip_header_works_again() {
        let client = Client::debug(rocket_with_custom_ip_header(Some("x-forward-ip"))).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Forward-IP", "1.2.3.4"))
            .dispatch();

        assert_eq!(response.into_string(), Some("1.2.3.4".into()));

        let config = Figment::from(rocket::Config::debug_default())
            .merge(("ip_header", "x-forward-ip"));

        let client = Client::debug(rocket::custom(config).mount("/", routes())).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Forward-IP", "1.2.3.4"))
            .dispatch();

        assert_eq!(response.into_string(), Some("1.2.3.4".into()));
    }

    #[test]
    fn check_default_real_ip_header_works() {
        let client = Client::debug_with(routes()).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Real-IP", "1.2.3.4"))
            .dispatch();

        assert_eq!(response.into_string(), Some("1.2.3.4".into()));
    }

    #[test]
    fn check_no_ip_header_works() {
        let client = Client::debug(rocket_with_custom_ip_header(None)).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Real-IP", "1.2.3.4"))
            .dispatch();

        assert_eq!(response.into_string(), Some("<none>".into()));

        let config = Figment::from(rocket::Config::debug_default())
            .merge(("ip_header", false));

        let client = Client::debug(rocket::custom(config).mount("/", routes())).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Real-IP", "1.2.3.4"))
            .dispatch();

        assert_eq!(response.into_string(), Some("<none>".into()));

        let config = Figment::from(rocket::Config::debug_default());
        let client = Client::debug(rocket::custom(config).mount("/", routes())).unwrap();
        let response = client.get("/")
            .header(Header::new("X-Real-IP", "1.2.3.4"))
            .dispatch();

        assert_eq!(response.into_string(), Some("1.2.3.4".into()));
    }
}
