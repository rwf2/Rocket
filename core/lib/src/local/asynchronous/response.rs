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
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build().mount("/", routes![hello_world])
///     #    .configure(rocket::Config::debug_default())
/// }
///
/// # async fn read_body_manually() -> io::Result<()> {
/// // Dispatch a `GET /` request.
/// let client = Client::tracked(rocket()).await.expect("valid rocket");
/// let mut response = client.get("/").dispatch().await;
///
/// // Check metadata validity.
/// assert_eq!(response.status(), Status::Ok);
/// assert_eq!(response.body().preset_size(), Some(13));
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
        //      references bounded by `self`. This is easily verified by noting
        //      that 1) `LocalResponse` fields are private, and 2) all `impl`s
        //      of `LocalResponse` aside from this method abstract the lifetime
        //      away as `'_`, ensuring it is not used for any output value.
        let boxed_req = Box::new(req);
        let request: &'c Request<'c> = unsafe { &*(&*boxed_req as *const _) };

        async move {
            let response: Response<'c> = f(request).await;
            let mut cookies = CookieJar::new(request.rocket().config());
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

    pub(crate) async fn _into_string(mut self) -> io::Result<String> {
        self.response.body_mut().to_string().await
    }

    pub(crate) async fn _into_bytes(mut self) -> io::Result<Vec<u8>> {
        self.response.body_mut().to_bytes().await
    }

    #[cfg(feature = "json")]
    async fn _into_json<T: Send + 'static>(self) -> Option<T>
        where T: serde::de::DeserializeOwned
    {
        self.blocking_read(|r| serde_json::from_reader(r)).await?.ok()
    }

    #[cfg(feature = "msgpack")]
    async fn _into_msgpack<T: Send + 'static>(self) -> Option<T>
        where T: serde::de::DeserializeOwned
    {
        self.blocking_read(|r| rmp_serde::from_read(r)).await?.ok()
    }

    #[cfg(any(feature = "json", feature = "msgpack"))]
    async fn blocking_read<T, F>(mut self, f: F) -> Option<T>
        where T: Send + 'static,
              F: FnOnce(&mut dyn io::Read) -> T + Send + 'static
    {
        use tokio::sync::mpsc;
        use tokio::io::AsyncReadExt;

        struct ChanReader {
            last: Option<io::Cursor<Vec<u8>>>,
            rx: mpsc::Receiver<io::Result<Vec<u8>>>,
        }

        impl std::io::Read for ChanReader {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                loop {
                    if let Some(ref mut cursor) = self.last {
                        if cursor.position() < cursor.get_ref().len() as u64 {
                            return std::io::Read::read(cursor, buf);
                        }
                    }

                    if let Some(buf) = self.rx.blocking_recv() {
                        self.last = Some(io::Cursor::new(buf?));
                    } else {
                        return Ok(0);
                    }
                }
            }
        }

        let (tx, rx) = mpsc::channel(2);
        let reader = tokio::task::spawn_blocking(move || {
            let mut reader = ChanReader { last: None, rx };
            f(&mut reader)
        });

        loop {
            // TODO: Try to fill as much as the buffer before send it off?
            let mut buf = Vec::with_capacity(1024);
            match self.read_buf(&mut buf).await {
                Ok(n) if n == 0 => break,
                Ok(_) => tx.send(Ok(buf)).await.ok()?,
                Err(e) => {
                    tx.send(Err(e)).await.ok()?;
                    break;
                }
            }
        }

        // NOTE: We _must_ drop tx now to prevent a deadlock!
        drop(tx);

        reader.await.ok()
    }

    // Generates the public API methods, which call the private methods above.
    pub_response_impl!("# use rocket::local::asynchronous::Client;\n\
        use rocket::local::asynchronous::LocalResponse;" async await);
}

impl AsyncRead for LocalResponse<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(self.response.body_mut()).poll_read(cx, buf)
    }
}

impl std::fmt::Debug for LocalResponse<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self._response().fmt(f)
    }
}
