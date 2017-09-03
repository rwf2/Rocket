#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[derive(FromForm)]
struct Query {
    field: String
}

#[get("/?<query>")]
fn first(query: Query) -> String {
    query.field
}

#[get("/")]
fn second() -> &'static str {
    "no query"
}

mod tests {
    use super::*;
    use rocket::Rocket;
    use rocket::local::Client;

    fn assert_no_collision(rocket: Rocket) {
        let client = Client::new(rocket).unwrap();
        let mut response = client.get("/?field=query").dispatch();
        assert_eq!(response.body_string(), Some("query".into()));

        let mut response = client.get("/").dispatch();
        assert_eq!(response.body_string(), Some("no query".into()));
    }

    #[test]
    fn check_query_collisions() {
        let rocket = rocket::ignite().mount("/", routes![first, second]);
        assert_no_collision(rocket);

        let rocket = rocket::ignite().mount("/", routes![second, first]);
        assert_no_collision(rocket);
    }
}
