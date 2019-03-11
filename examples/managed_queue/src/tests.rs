use rocket::local::Client;
use rocket::http::Status;

#[test]
fn test_push_pop() {
    let client = Client::new(super::rocket()).unwrap();

    let response = client.put("/push?event=test1").dispatch();
    assert_eq!(response.status(), Status::Ok);

    let mut response = client.get("/pop").dispatch();
    assert_eq!(response.body_string(), Some("test1".to_string()));
}
