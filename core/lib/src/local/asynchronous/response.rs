use std::io;
use std::future::Future;
use std::{pin::Pin, task::{Context, Poll}};

use tokio::io::{AsyncRead, ReadBuf};

use crate::http::CookieJar;
use crate::{Request, Response};

/// An `async` response from a dispatched [`LocalRequest`](super::LocalRequest).
///
/// This `LocalResponse` implements [`tokio::io::AsyncRead`]. As such, if
/// [`into_string()`](LocalResponse::into_string()) and
/// [`into_bytes()`](LocalResponse::into_bytes()) do not suffice, the response's
/// body can be read directly:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use std::io;
///
/// use rocket::local::asynchronous::Client;
/// use rocket::tokio::io::AsyncReadExt;
/// use rocket::http::Status;
///
/// #[get("/")]
/// fn hello_world() -> &'static str {
///     "Hello, world!"
/// }
///
/// # /*
/// #[launch]
/// # */
/// fn rocket() -> rocket::Rocket {
///     rocket::ignite().mount("/", routes![hello_world])
/// }
///
/// # async fn read_body_manually() -> io::Result<()> {
/// // Dispatch a `GET /` request.
/// let client = Client::tracked(rocket()).await.expect("valid rocket");
/// let mut response = client.get("/").dispatch().await;
///
/// // Check metadata validity.
/// assert_eq!(response.status(), Status::Ok);
/// assert_eq!(response.body().unwrap().known_size(), Some(13));
///
/// // Read 10 bytes of the body. Note: in reality, we'd use `into_string()`.
/// let mut buffer = [0; 10];
/// response.read(&mut buffer).await?;
/// assert_eq!(buffer, "Hello, wor".as_bytes());
/// # Ok(())
/// # }
/// # rocket::async_test(read_body_manually()).expect("read okay");
/// ```
///
/// For more, see [the top-level documentation](../index.html#localresponse).
pub struct LocalResponse<'c> {
    _request: Box<Request<'c>>,
    response: Response<'c>,
    cookies: CookieJar<'c>,
}

impl<'c> LocalResponse<'c> {
    pub(crate) fn new<F, O>(req: Request<'c>, f: F) -> impl Future<Output = LocalResponse<'c>>
        where F: FnOnce(&'c Request<'c>) -> O + Send,
              O: Future<Output = Response<'c>> + Send
    {
        // `LocalResponse` is a self-referential structure. In particular,
        // `inner` can refer to `_request` and its contents. As such, we must
        //   1) Ensure `Request` has a stable address.
        //
        //      This is done by `Box`ing the `Request`, using only the stable
        //      address thereafter.
        //
        //   2) Ensure no refs to `Request` or its contents leak with a lifetime
        //      extending beyond that of `&self`.
        //
        //      We have no methods that return an `&Request`. However, we must
        //      also ensure that `Response` doesn't leak any such references. To
        //      do so, we don't expose the `Response` directly in any way;
        //      otherwise, methods like `.headers()` could, in conjunction with
        //      particular crafted `Responder`s, potentially be used to obtain a
        //      reference to contents of `Request`. All methods, instead, return
        //      references bounded by `self`. This is easily verified by nothing
        //      that 1) `LocalResponse` fields are private, and 2) all `impl`s
        //      of `LocalResponse` aside from this method abstract the lifetime
        //      away as `'_`, ensuring it is not used for any output value.
        let boxed_req = Box::new(req);
        let request: &'c Request<'c> = unsafe { &*(&*boxed_req as *const _) };

        async move {
            let response: Response<'c> = f(request).await;
            let mut cookies = CookieJar::new(&request.state.config.secret_key);
            for cookie in response.cookies() {
                cookies.add_original(cookie.into_owned());
            }

            LocalResponse { cookies, _request: boxed_req, response, }
        }
    }
}

impl LocalResponse<'_> {
    pub(crate) fn _response(&self) -> &Response<'_> {
        &self.response
    }

    pub(crate) fn _cookies(&self) -> &CookieJar<'_> {
        &self.cookies
    }

    pub(crate) async fn _into_string(mut self) -> Option<String> {
        self.response.body_string().await
    }

    pub(crate) async fn _into_bytes(mut self) -> Option<Vec<u8>> {
        self.response.body_bytes().await
    }

    // Generates the public API methods, which call the private methods above.
    pub_response_impl!("# use rocket::local::asynchronous::Client;
        use rocket::local::asynchronous::LocalResponse;" async await);
}

impl AsyncRead for LocalResponse<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let body = match self.response.body_mut() {
            Some(body) => body,
            _ => return Poll::Ready(Ok(()))
        };

        Pin::new(body.as_reader()).poll_read(cx, buf)
    }
}

impl std::fmt::Debug for LocalResponse<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self._response().fmt(f)
    }
}
