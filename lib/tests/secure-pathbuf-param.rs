#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use std::path::PathBuf;

#[get("/<path..>")]
fn none(path: PathBuf) -> String {
    path.to_string_lossy().into()
}

#[cfg(feature = "testing")]
mod tests {
    use super::*;
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;

    #[test]
    fn secure_segments() {
        let rocket = rocket::ignite()
            .mount("/", routes![none]);

        let mut req = MockRequest::new(Get, "hello%2f..%2f..%2f..%2fetc%2fpasswd");
        let mut response = req.dispatch_with(&rocket);
        let body_str = response.body().and_then(|b| b.into_string());
        assert_eq!(body_str, Some("etc/passwd".into()));
    }
}
