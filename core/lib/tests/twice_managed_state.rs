struct A;

#[test]
#[should_panic]
fn twice_managed_state() {
    let _ = rocket::ignite().manage(A).manage(A);
}
