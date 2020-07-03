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

#[get("/test")]
fn test(shutdown: Shutdown) -> Response<'static> {
    shutdown.shutdown();
    Response::build()
    .chunked_body(AsyncReader, 512)
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

        let _ = client.get("/test").dispatch();
    }
}
