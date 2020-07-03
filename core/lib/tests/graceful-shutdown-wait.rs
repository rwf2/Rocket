#[macro_use] extern crate rocket;

use rocket::Shutdown;
use rocket::response::Response;
use tokio::io::AsyncRead;

use std::pin::Pin;
use std::task::{Poll, Context};

struct AsyncReader;

impl AsyncRead for AsyncReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        _buf: &mut [u8]
    ) -> Poll<tokio::io::Result<usize>> {
        Poll::Pending
    }
}

#[get("/test-shutdown")]
async fn test(shutdown: Shutdown) -> Response<'static> {
    shutdown.shutdown();
    Response::build()
    .chunked_body(AsyncReader, 512)
    .finalize()
}

#[get("/test-wait")]
async fn test2(shutdown: Shutdown) -> Response<'static> {
    shutdown.wait().await;
    Response::build()
    .chunked_body(AsyncReader, 512)
    .finalize()
}

mod tests {
    use super::*;
    use rocket::local::asynchronous::Client;
    use futures::join;

    #[rocket::async_test]
    async fn graceful_shutdown_works() {
        let rocket = rocket::ignite()
            .mount("/", routes![test, test2]);
        let client = Client::new(rocket).await.unwrap();

        let shutdown_response = client.get("/test-shutdown").dispatch();
        let wait_response = client.get("/test-wait").dispatch();
        let _ = join!(shutdown_response, wait_response);
    }
}
