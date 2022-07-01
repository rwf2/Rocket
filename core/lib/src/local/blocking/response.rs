use std::io;
use tokio::io::AsyncReadExt;

use crate::{Response, local::asynchronous, http::CookieJar};

use super::Client;

/// A `blocking` response from a dispatched [`LocalRequest`](super::LocalRequest).
///
/// This `LocalResponse` implements [`io::Read`]. As such, if
/// [`into_string()`](LocalResponse::into_string()) and
/// [`into_bytes()`](LocalResponse::into_bytes()) do not suffice, the response's
/// body can be read directly:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use std::io::{self, Read};
///
/// use rocket::local::blocking::Client;
/// use rocket::http::Status;
///
/// #[get("/")]
/// fn hello_world() -> &'static str {
///     "Hello, world!"
/// }
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build().mount("/", routes![hello_world])
///     #    .configure(rocket::Config::debug_default())
/// }
///
/// # fn read_body_manually() -> io::Result<()> {
/// // Dispatch a `GET /` request.
/// let client = Client::tracked(rocket()).expect("valid rocket");
/// let mut response = client.get("/").dispatch();
///
/// // Check metadata validity.
/// assert_eq!(response.status(), Status::Ok);
/// assert_eq!(response.body().preset_size(), Some(13));
///
/// // Read 10 bytes of the body. Note: in reality, we'd use `into_string()`.
/// let mut buffer = [0; 10];
/// response.read(&mut buffer)?;
/// assert_eq!(buffer, "Hello, wor".as_bytes());
/// # Ok(())
/// # }
/// # read_body_manually().expect("read okay");
/// ```
///
/// For more, see [the top-level documentation](../index.html#localresponse).
pub struct LocalResponse<'c> {
    pub(in super) inner: asynchronous::LocalResponse<'c>,
    pub(in super) client: &'c Client,
}

impl LocalResponse<'_> {
    fn _response(&self) -> &Response<'_> {
        &self.inner._response()
    }

    pub(crate) fn _cookies(&self) -> &CookieJar<'_> {
        self.inner._cookies()
    }

    fn _into_string(self) -> io::Result<String> {
        self.client.block_on(self.inner._into_string())
    }

    fn _into_bytes(self) -> io::Result<Vec<u8>> {
        self.client.block_on(self.inner._into_bytes())
    }

    #[cfg(feature = "json")]
    fn _into_json<T: Send + 'static>(self) -> Option<T>
        where T: serde::de::DeserializeOwned
    {
        serde_json::from_reader(self).ok()
    }

    #[cfg(feature = "msgpack")]
    fn _into_msgpack<T: Send + 'static>(self) -> Option<T>
        where T: serde::de::DeserializeOwned
    {
        rmp_serde::from_read(self).ok()
    }

    // Generates the public API methods, which call the private methods above.
    pub_response_impl!("# use rocket::local::blocking::Client;\n\
        use rocket::local::blocking::LocalResponse;");
}

impl io::Read for LocalResponse<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.client.block_on(self.inner.read(buf))
    }
}

impl std::fmt::Debug for LocalResponse<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self._response().fmt(f)
    }
}
