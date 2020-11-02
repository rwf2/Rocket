#[macro_use] extern crate rocket;

use std::path::{Path, PathBuf};
use rocket::http::ext::Normalize;
use rocket::Route;

#[get("/<path..>")]
fn files(route: &Route, path: PathBuf) -> String {
    Path::new(route.base()).join(path).normalized_str().to_string()
}

mod route_guard_tests {
    use super::*;
    use rocket::local::blocking::Client;

    fn assert_path(client: &Client, path: &str) {
        let res = client.get(path).dispatch();
        assert_eq!(res.into_string(), Some(path.into()));
    }

    #[test]
    fn check_mount_path() {
        let rocket = rocket::ignite()
            .mount("/first", routes![files])
            .mount("/second", routes![files]);

        let client = Client::tracked(rocket).unwrap();
        assert_path(&client, "/first/some/path");
        assert_path(&client, "/second/some/path");
        assert_path(&client, "/first/second/b/c");
        assert_path(&client, "/second/a/b/c");
    }
}
