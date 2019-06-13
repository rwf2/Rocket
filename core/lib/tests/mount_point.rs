#[test]
#[should_panic]
fn bad_dynamic_mount() {
    rocket::ignite().mount("<name>", vec![]);
}

#[test]
fn good_static_mount() {
    rocket::ignite().mount("/abcdefghijkl_mno", vec![]);
}
