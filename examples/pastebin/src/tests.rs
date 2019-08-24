use super::{rocket, index};
use rocket::local::Client;
use rocket::http::{Status, ContentType};

fn extract_id(from: &str) -> Option<String> {
    from.rfind('/').map(|i| &from[(i + 1)..]).map(|s| s.trim_end().to_string())
}

#[rocket::async_test]
async fn check_index() {
    let client = Client::new(rocket()).unwrap();

    // Ensure the index returns what we expect.
    let mut response = client.get("/").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.body_string().await, Some(index().into()))
}

async fn upload_paste(client: &Client, body: &str) -> String {
    let mut response = client.post("/").body(body).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    extract_id(&response.body_string().await.unwrap()).unwrap()
}

async fn download_paste(client: &Client, id: &str) -> String {
    let mut response = client.get(format!("/{}", id)).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    response.body_string().await.unwrap()
}

#[rocket::async_test]
async fn pasting() {
    let client = Client::new(rocket()).unwrap();

    // Do a trivial upload, just to make sure it works.
    let body_1 = "Hello, world!";
    let id_1 = upload_paste(&client, body_1).await;
    assert_eq!(download_paste(&client, &id_1).await, body_1);

    // Make sure we can keep getting that paste.
    assert_eq!(download_paste(&client, &id_1).await, body_1);
    assert_eq!(download_paste(&client, &id_1).await, body_1);
    assert_eq!(download_paste(&client, &id_1).await, body_1);

    // Upload some unicode.
    let body_2 = "こんにちは";
    let id_2 = upload_paste(&client, body_2).await;
    assert_eq!(download_paste(&client, &id_2).await, body_2);

    // Make sure we can get both pastes.
    assert_eq!(download_paste(&client, &id_1).await, body_1);
    assert_eq!(download_paste(&client, &id_2).await, body_2);
    assert_eq!(download_paste(&client, &id_1).await, body_1);
    assert_eq!(download_paste(&client, &id_2).await, body_2);

    // Now a longer upload.
    let body_3 = "Lorem ipsum dolor sit amet, consectetur adipisicing elit, sed
        do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim
        ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut
        aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit
        in voluptate velit esse cillum dolore eu fugiat nulla pariatur.
        Excepteur sint occaecat cupidatat non proident, sunt in culpa qui
        officia deserunt mollit anim id est laborum.";
    let id_3 = upload_paste(&client, body_3).await;
    assert_eq!(download_paste(&client, &id_3).await, body_3);
    assert_eq!(download_paste(&client, &id_1).await, body_1);
    assert_eq!(download_paste(&client, &id_2).await, body_2);
}
