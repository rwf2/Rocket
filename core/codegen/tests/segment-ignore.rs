#[macro_use]
extern crate rocket;

use rocket::local::blocking::Client;

#[get("/<_>")]
fn ignored_parameter() {}

#[get("/<_>/<_>")]
fn ignored_parameters2() {}

#[get("/<_>/<_>/<_>")]
fn ignored_parameters3() {}

#[get("/a/b/c/<_>/<_>")]
fn ignored_parameters4() {}

#[get("/<_>/a/a/a")]
fn ignored_parameters5() {}

#[get("/f/<_>/c/d")]
fn ignored_parameters6() {}

#[get("/<_>/a/b/c/<_>")]
fn ignored_parameters7() {}

#[get("/<a>/<_>/<_c>/e")]
fn ignored_parameters8(a: String, _c: String) -> String {
    a
}

#[test]
fn test_data() {
    let rocket = rocket::ignite().mount(
        "/",
        routes![
            ignored_parameter,
            ignored_parameters2,
            ignored_parameters3,
            ignored_parameters4,
            ignored_parameters5,
            ignored_parameters6,
            ignored_parameters7,
            ignored_parameters8,
        ],
    );
    let client = Client::new(rocket).unwrap();

    let response = client.get("/asdf/hjkl/aaaa/e").dispatch();

    assert_eq!(response.into_string().unwrap(), "asdf");
}
