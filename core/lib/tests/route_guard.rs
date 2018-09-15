#![feature(plugin, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;

use std::path::PathBuf;
use rocket::Route;

#[get("/<path..>")]
fn files(route: &Route, path: PathBuf) -> String {
    format!("{}/{}", route.base(), path.to_string_lossy())
}

mod route_guard_tests {
    use super::*;
    use rocket::local::Client;

    fn assert_path(client: &Client, path: &str) {
        let mut res = client.get(path).dispatch();
        assert_eq!(res.body_string(), Some(path.into()));
    }

    #[test]
    fn check_mount_path() {
        let rocket = rocket::ignite()
            .mount("/first", routes![files])
            .mount("/second", routes![files]);

        let client = Client::new(rocket).unwrap();
        assert_path(&client, "/first/some/path");
        assert_path(&client, "/second/some/path");
        assert_path(&client, "/first/second/b/c");
        assert_path(&client, "/second/a/b/c");
    }
}
