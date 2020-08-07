#[macro_use] extern crate rocket;
use rocket::Route;

pub fn prepend(prefix: &str, route: Route) -> Route {
    route.map_base(|base| format!("{}{}", prefix, base)).unwrap()
}

pub fn extend_routes(prefix: &str, routes: Vec<Route>) -> Vec<Route> {
    routes.into_iter()
        .map(|route| prepend(prefix, route))
        .collect()
}

mod a {
    #[get("/b/<id>")]
    fn b(id: u8) -> String { id.to_string() }

    pub fn routes() -> Vec<rocket::Route> {
        super::extend_routes("/a", routes![b])
    }
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", a::routes()).mount("/foo", a::routes())
}

mod mapped_base_tests {
    use rocket::local::blocking::Client;
    use rocket::http::Status;

    #[test]
    fn only_prefix() {
        let client = Client::new(super::rocket()).unwrap();

        let response = client.get("/a/b/3").dispatch();
        assert_eq!(response.into_string().unwrap(), "3");

        let response = client.get("/a/b/239").dispatch();
        assert_eq!(response.into_string().unwrap(), "239");

        let response = client.get("/b/239").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }

    #[test]
    fn prefix_and_base() {
        let client = Client::new(super::rocket()).unwrap();

        let response = client.get("/foo/a/b/23").dispatch();
        assert_eq!(response.into_string().unwrap(), "23");

        let response = client.get("/foo/a/b/99").dispatch();
        assert_eq!(response.into_string().unwrap(), "99");

        let response = client.get("/foo/b/239").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}
