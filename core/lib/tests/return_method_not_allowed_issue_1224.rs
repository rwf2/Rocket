#![feature(proc_macro_hygiene)]

#[macro_use]
extern crate rocket;

#[get("/index")]
fn get_index() -> &'static str {
    "GET index :)"
}

#[post("/index")]
fn post_index() -> &'static str {
    "POST index :)"
}

#[post("/hello")]
fn post_hello() -> &'static str {
    "POST Hello, world!"
}

mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn test_http_200_when_same_route_with_diff_meth() {
        let rocket = rocket::ignite()
            .mount("/", routes![get_index, post_index]);

        let client = Client::tracked(rocket).unwrap();

        let response = client.post("/index").dispatch();

        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn test_http_200_when_head_request() {
        let rocket = rocket::ignite().mount("/", routes![get_index]);

        let client = Client::tracked(rocket).unwrap();

        let response = client.head("/index").dispatch();

        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn test_http_200_when_route_is_ok() {
        let rocket = rocket::ignite().mount("/", routes![get_index]);

        let client = Client::tracked(rocket).unwrap();

        let response = client.get("/index").dispatch();

        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn test_http_200_with_params() {
        let rocket = rocket::ignite().mount("/", routes![get_index]);

        let client = Client::tracked(rocket).unwrap();

        let response = client.get("/index?say=hi").dispatch();

        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn test_http_404_when_route_not_match() {
        let rocket = rocket::ignite().mount("/", routes![get_index]);

        let client = Client::tracked(rocket).unwrap();

        let response = client.get("/abc").dispatch();

        assert_eq!(response.status(), Status::NotFound);
    }

    #[test]
    fn test_http_405_when_method_not_match() {
        let rocket = rocket::ignite().mount("/", routes![get_index]);

        let client = Client::tracked(rocket).unwrap();

        let response = client.post("/index").dispatch();

        assert_eq!(response.status(), Status::MethodNotAllowed);
    }

    #[test]
    fn test_http_405_with_params() {
        let rocket = rocket::ignite().mount("/", routes![post_hello]);

        let client = Client::tracked(rocket).unwrap();

        let response = client.get("/hello?say=hi").dispatch();

        assert_eq!(response.status(), Status::MethodNotAllowed);
    }
}
