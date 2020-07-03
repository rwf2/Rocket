#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::shutdown::ShutdownHandle;
use rocket::response::Response;
use tokio::io::AsyncRead;

use std::pin::Pin;
use std::task::{Poll, Context};

struct AsyncReader(bool);

impl AsyncRead for AsyncReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        buf: &mut [u8]
    ) -> Poll<tokio::io::Result<usize>> {
        if self.0 {
            Poll::Ready(Ok(0))
        } else {
            buf[0] = b'a';
            Pin::<&mut AsyncReader>::into_inner(self).0 = true;
            Poll::Ready(Ok(1))
        }
    }
}

#[get("/test")]
fn test(shutdown: ShutdownHandle) -> Response<'static> {
    shutdown.shutdown();
    Response::build()
    .chunked_body(AsyncReader(false), 512)
    .finish_on_shutdown(true)
    .finalize()
}

mod tests {
    use super::*;
    use rocket::local::Client;

    #[rocket::async_test]
    async fn graceful_shutdown_works() {
        let rocket = rocket::ignite()
            .mount("/", routes![test]);
        let client = Client::new(rocket).await.unwrap();

        let mut response = client.get("/test").dispatch().await;
        assert_eq!(response.body_string().await.unwrap_or(String::new()), String::from("a"));
    }
}
