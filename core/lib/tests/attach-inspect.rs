use rocket::fairing::AdHoc;

#[rocket::async_test]
async fn test_inspectable_attach_state() {
    let mut rocket = rocket::ignite()
        .attach(AdHoc::on_attach("Add State", |rocket| async {
            Ok(rocket.manage("Hi!"))
        }));

    let state = rocket.inspect().await;
    assert_eq!(state.state::<&'static str>(), Some(&"Hi!"));
}

#[rocket::async_test]
async fn test_inspectable_attach_state_in_future_attach() {
    let mut rocket = rocket::ignite()
        .attach(AdHoc::on_attach("Add State", |rocket| async {
            Ok(rocket.manage("Hi!"))
        }))
        .attach(AdHoc::on_attach("Inspect State", |mut rocket| async {
            let state = rocket.inspect().await;
            assert_eq!(state.state::<&'static str>(), Some(&"Hi!"));
            Ok(rocket)
        }));

    let _ = rocket.inspect().await;
}

#[rocket::async_test]
async fn test_attach_state_is_well_ordered() {
    let mut rocket = rocket::ignite()
        .attach(AdHoc::on_attach("Inspect State Pre", |mut rocket| async {
            let state = rocket.inspect().await;
            assert_eq!(state.state::<&'static str>(), None);
            Ok(rocket)
        }))
        .attach(AdHoc::on_attach("Add State", |rocket| async {
            Ok(rocket.manage("Hi!"))
        }));

    let state = rocket.inspect().await;
    assert_eq!(state.state::<&'static str>(), Some(&"Hi!"));
}
