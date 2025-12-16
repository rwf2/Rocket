extern crate rocket_community as rocket;

struct A;

#[test]
#[should_panic]
fn twice_managed_state() {
    let _ = rocket::build().manage(A).manage(A);
}
