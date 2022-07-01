#[macro_use] extern crate rocket;

use rocket::{Request, Rocket, Route, Catcher, Build, route, catcher};
use rocket::data::Data;
use rocket::http::{Method, Status};
use rocket::local::blocking::Client;

#[get("/panic")]
fn panic_route() -> &'static str {
    panic!("Panic in route")
}

#[catch(404)]
fn panic_catcher() -> &'static str {
    panic!("Panic in catcher")
}

#[catch(500)]
fn ise() -> &'static str {
    "Hey, sorry! :("
}

fn pre_future_route<'r>(_: &'r Request<'_>, _: Data<'r>) -> route::BoxFuture<'r> {
    panic!("hey now...");
}

fn rocket() -> Rocket<Build> {
    rocket::build()
        .mount("/", routes![panic_route])
        .mount("/", vec![Route::new(Method::Get, "/pre", pre_future_route)])
}

#[test]
fn catches_route_panic() {
    let rocket = rocket().register("/", catchers![panic_catcher, ise]);
    let client = Client::debug(rocket).unwrap();
    let response = client.get("/panic").dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.into_string().unwrap(), "Hey, sorry! :(");
}

#[test]
fn catches_catcher_panic() {
    let rocket = rocket().register("/", catchers![panic_catcher, ise]);
    let client = Client::debug(rocket).unwrap();
    let response = client.get("/noroute").dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.into_string().unwrap(), "Hey, sorry! :(");
}

#[test]
fn catches_double_panic() {
    #[catch(500)]
    fn double_panic() {
        panic!("so, so sorry...")
    }

    let rocket = rocket().register("/", catchers![panic_catcher, double_panic]);
    let client = Client::debug(rocket).unwrap();
    let response = client.get("/noroute").dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
    assert!(response.into_string().unwrap().contains("Rocket"));
}

#[test]
fn catches_early_route_panic() {
    let rocket = rocket().register("/", catchers![panic_catcher, ise]);
    let client = Client::debug(rocket).unwrap();
    let response = client.get("/pre").dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.into_string().unwrap(), "Hey, sorry! :(");
}

#[test]
fn catches_early_catcher_panic() {
    fn pre_future_catcher<'r>(_: Status, _: &'r Request) -> catcher::BoxFuture<'r> {
        panic!("a panicking pre-future catcher")
    }

    let rocket = rocket()
        .register("/", vec![Catcher::new(404, pre_future_catcher)])
        .register("/", catchers![ise]);

    let client = Client::debug(rocket).unwrap();
    let response = client.get("/idontexist").dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.into_string().unwrap(), "Hey, sorry! :(");
}
