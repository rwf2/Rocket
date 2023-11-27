use rocket::fairing::AdHoc;

#[rocket::async_test]
async fn test_inspectable_launch_state() -> Result<(), rocket::Error> {
    let rocket = rocket::custom(rocket::Config::debug_default())
        .attach(AdHoc::on_ignite("Add State", |rocket| async {
            rocket.manage("Hi!")
        }))
        .ignite()
        .await?;

    let state = rocket.state::<&'static str>();
    assert_eq!(state, Some(&"Hi!"));
    Ok(())
}

#[rocket::async_test]
async fn test_inspectable_launch_state_in_liftoff() -> Result<(), rocket::Error> {
    let rocket = rocket::custom(rocket::Config::debug_default())
        .attach(AdHoc::on_ignite("Add State", |rocket| async {
            rocket.manage("Hi!")
        }))
        .attach(AdHoc::on_ignite("Inspect State", |rocket| async {
            let state = rocket.state::<&'static str>();
            assert_eq!(state, Some(&"Hi!"));
            rocket
        }))
        .attach(AdHoc::on_liftoff("Inspect State", |rocket| {
            Box::pin(async move {
                let state = rocket.state::<&'static str>();
                assert_eq!(state, Some(&"Hi!"));
            })
        }))
        .ignite()
        .await?;

    let state = rocket.state::<&'static str>();
    assert_eq!(state, Some(&"Hi!"));
    Ok(())
}

#[rocket::async_test]
async fn test_launch_state_is_well_ordered() -> Result<(), rocket::Error> {
    let rocket = rocket::custom(rocket::Config::debug_default())
        .attach(AdHoc::on_ignite("Inspect State Pre", |rocket| async {
            let state = rocket.state::<&'static str>();
            assert_eq!(state, None);
            rocket
        }))
        .attach(AdHoc::on_ignite("Add State", |rocket| async {
            rocket.manage("Hi!")
        }))
        .attach(AdHoc::on_ignite("Inspect State", |rocket| async {
            let state = rocket.state::<&'static str>();
            assert_eq!(state, Some(&"Hi!"));
            rocket
        }))
        .ignite()
        .await?;

    let state = rocket.state::<&'static str>();
    assert_eq!(state, Some(&"Hi!"));
    Ok(())
}

#[should_panic]
#[rocket::async_test]
async fn negative_test_launch_state() {
    let _ = rocket::custom(rocket::Config::debug_default())
        .attach(AdHoc::on_ignite("Add State", |rocket| async {
            rocket.manage("Hi!")
        }))
        .attach(AdHoc::on_ignite("Inspect State", |rocket| async {
            let state = rocket.state::<&'static str>();
            assert_ne!(state, Some(&"Hi!"));
            rocket
        }))
        .ignite()
        .await;
}
