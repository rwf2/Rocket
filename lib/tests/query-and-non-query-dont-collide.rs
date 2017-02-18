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

#[cfg(feature = "testing")]
mod tests {
    use super::*;

    use rocket::Rocket;
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;

    fn assert_no_collision(rocket: &Rocket) {
        let mut req = MockRequest::new(Get, "/?field=query");
        let mut response = req.dispatch_with(&rocket);
        let body_str = response.body().and_then(|b| b.into_string());
        assert_eq!(body_str, Some("query".to_string()));

        let mut req = MockRequest::new(Get, "/");
        let mut response = req.dispatch_with(&rocket);
        let body_str = response.body().and_then(|b| b.into_string());
        assert_eq!(body_str, Some("no query".to_string()));
    }

    #[test]
    fn check_query_collisions() {
        let rocket = rocket::ignite().mount("/", routes![first, second]);
        assert_no_collision(&rocket);

        let rocket = rocket::ignite().mount("/", routes![second, first]);
        assert_no_collision(&rocket);
    }
}
