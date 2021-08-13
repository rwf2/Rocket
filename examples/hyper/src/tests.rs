use hyper::service::Service;

#[tokio::test]
async fn index() {
    let mut svc = crate::make().await;

    let request = hyper::Request::builder()
        .uri("/")
        .body(hyper::Body::empty())
        .expect("invalid request");

    let response = svc.call(request).await.expect("server failed");
    assert_eq!(response.status(), 200);
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("failed to receive body");
    let expected_body: hyper::body::Bytes = (b"Hello, service user!" as &'static [u8]).into();
    assert_eq!(body, expected_body);
}
