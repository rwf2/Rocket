#[macro_use] extern crate rocket;

use rocket::Shutdown;
use rocket::response::Response;
use tokio::io::AsyncRead;

use std::pin::Pin;
use std::task::{Poll, Context};
use std::time::Duration;

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
fn test(shutdown: Shutdown) -> Response<'static> {
    shutdown.shutdown();
    Response::build()
    .chunked_body(AsyncReader(false), 512)
    .wait_on_shutdown(Duration::from_millis(u64::MAX))
    .finalize()
}

mod tests {
    use super::*;
    use rocket::local::blocking::Client;

    #[test]
    fn graceful_shutdown_works() {
        let rocket = rocket::ignite()
            .mount("/", routes![test]);
        let client = Client::new(rocket).unwrap();

        let response = client.get("/test").dispatch();
        assert_eq!(response.into_string().unwrap(), String::from("a"));
    }
}
