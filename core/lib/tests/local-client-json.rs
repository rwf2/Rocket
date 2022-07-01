#![cfg(feature = "json")]

#[macro_use] extern crate rocket;

use rocket::serde::json::Json;

#[get("/int")] fn int() -> Json<i32> { Json(5) }
#[get("/nil")] fn nil() -> Json<()> { Json(()) }

#[async_test]
async fn async_json_works() {
    use rocket::local::asynchronous::Client;

    let client = Client::debug_with(routes![int, nil]).await.unwrap();

    let int0 = client.get("/int").dispatch().await.into_json::<u32>().await;
    let int1 = client.get("/int").dispatch().await.into_json::<i32>().await;

    assert_eq!(int0, Some(5));
    assert_eq!(int1, Some(5));

    let nil0 = client.get("/nil").dispatch().await.into_json::<()>().await;
    assert_eq!(nil0, Some(()));
}
