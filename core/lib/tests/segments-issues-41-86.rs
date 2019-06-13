#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::http::uri::Segments;

#[get("/test/<path..>")]
fn test(path: Segments<'_>) -> String {
    path.collect::<Vec<_>>().join("/")
}

#[get("/two/<path..>")]
fn two(path: Segments<'_>) -> String {
    path.collect::<Vec<_>>().join("/")
}

#[get("/one/two/<path..>")]
fn one_two(path: Segments<'_>) -> String {
    path.collect::<Vec<_>>().join("/")
}

#[get("/<path..>", rank = 2)]
fn none(path: Segments<'_>) -> String {
    path.collect::<Vec<_>>().join("/")
}

#[get("/static/<user>/is/<path..>")]
fn dual(user: String, path: Segments<'_>) -> String {
    user + "/is/" + &path.collect::<Vec<_>>().join("/")
}

mod tests {
    use super::*;
    use rocket::local::Client;

    #[test]
    fn segments_works() {
        let rocket = rocket::ignite()
            .mount("/", routes![test, two, one_two, none, dual])
            .mount("/point", routes![test, two, one_two, dual]);
        let client = Client::new(rocket).unwrap();

        // We construct a path that matches each of the routes above. We ensure the
        // prefix is stripped, confirming that dynamic segments are working.
        for prefix in &["", "/test", "/two", "/one/two",
                        "/point/test", "/point/two", "/point/one/two",
                        "/static", "/point/static"]
        {
            let path = "this/is/the/path/we/want";
            let mut response = client.get(format!("{}/{}", prefix, path)).dispatch();
            assert_eq!(response.body_string(), Some(path.into()));
        }
    }
}
