#[macro_use] extern crate rocket;

use rocket::Rocket;
use rocket::http::Status;
use rocket::local::blocking::Client;

#[get("/panic")]
fn panic_route() -> &'static str {
    panic!("Panic in route")
}

#[catch(404)]
fn panic_catcher() -> &'static str {
    panic!("Panic in catcher")
}

fn rocket() -> Rocket {
    rocket::ignite()
        .mount("/", routes![panic_route])
        .register(catchers![panic_catcher])
}

#[test]
fn catches_route_panic() {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get("/panic").dispatch();
    assert_eq!(response.status(), Status::InternalServerError);

}

#[test]
fn catches_catcher_panic() {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get("/noroute").dispatch();
    assert_eq!(response.status(), Status::InternalServerError);
}
