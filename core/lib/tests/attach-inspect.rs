use rocket::fairing::AdHoc;

#[rocket::async_test]
async fn test_inspectable_attach_state() {
    let rocket = rocket::ignite()
        .attach(AdHoc::on_attach("Add State", |rocket| async {
            Ok(rocket.manage("Hi!"))
        }));

    let state = rocket.state::<&'static str>();
    assert_eq!(state, Some(&"Hi!"));
}

#[rocket::async_test]
async fn test_inspectable_attach_state_in_future_attach() {
    let rocket = rocket::ignite()
        .attach(AdHoc::on_attach("Add State", |rocket| async {
            Ok(rocket.manage("Hi!"))
        }))
        .attach(AdHoc::on_attach("Inspect State", |rocket| async {
            let state = rocket.state::<&'static str>();
            assert_eq!(state, Some(&"Hi!"));
            Ok(rocket)
        }));

    let state = rocket.state::<&'static str>();
    assert_eq!(state, Some(&"Hi!"));
}

#[rocket::async_test]
async fn test_attach_state_is_well_ordered() {
    let rocket = rocket::ignite()
        .attach(AdHoc::on_attach("Inspect State Pre", |rocket| async {
            let state = rocket.state::<&'static str>();
            assert_eq!(state, None);
            Ok(rocket)
        }))
        .attach(AdHoc::on_attach("Add State", |rocket| async {
            Ok(rocket.manage("Hi!"))
        }));

    let state = rocket.state::<&'static str>();
    assert_eq!(state, Some(&"Hi!"));
}
