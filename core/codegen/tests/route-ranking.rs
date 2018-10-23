#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::local::Client;

// Test that manual/auto ranking works as expected.

#[get("/<_number>")]
fn get0(_number: u8) -> &'static str { "0" }

#[get("/<_number>", rank = 1)]
fn get1(_number: u16) -> &'static str { "1" }

#[get("/<_number>", rank = 2)]
fn get2(_number: u32) -> &'static str { "2" }

#[get("/<_number>", rank = 3)]
fn get3(_number: u64) -> &'static str { "3" }

#[test]
fn test_ranking() {
    let rocket = rocket::ignite().mount("/", routes![get0, get1, get2, get3]);
    let client = Client::new(rocket).unwrap();

    let mut response = client.get("/0").dispatch();
    assert_eq!(response.body_string().unwrap(), "0");

    let mut response = client.get(format!("/{}", 1 << 8)).dispatch();
    assert_eq!(response.body_string().unwrap(), "1");

    let mut response = client.get(format!("/{}", 1 << 16)).dispatch();
    assert_eq!(response.body_string().unwrap(), "2");

    let mut response = client.get(format!("/{}", 1u64 << 32)).dispatch();
    assert_eq!(response.body_string().unwrap(), "3");
}

// Test a collision due to same auto rank.

#[get("/<_n>")]
fn get0b(_n: u8) {  }

#[test]
fn test_rank_collision() {
    use rocket::error::LaunchErrorKind;

    let rocket = rocket::ignite().mount("/", routes![get0, get0b]);
    let client_result = Client::new(rocket);
    match client_result.as_ref().map_err(|e| e.kind()) {
        Err(LaunchErrorKind::Collision(..)) => { /* o.k. */ },
        Ok(_) => panic!("client succeeded unexpectedly"),
        Err(e) => panic!("expected collision, got {}", e)
    }
}
