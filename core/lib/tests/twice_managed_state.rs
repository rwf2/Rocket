struct A;

#[rocket::async_test]
#[should_panic]
async fn twice_managed_state() {
    let _ = rocket::ignite().manage(A).manage(A).inspect().await;
}
