use std::borrow::Cow;
use std::cell::RefCell;

use crate::error::Error;
use crate::local::{asynchronous, blocking::{LocalRequest, LocalResponse}};
use crate::rocket::Rocket;
use crate::http::Method;

/// A `blocking` client to construct and dispatch local requests.
///
/// For details, see [the top-level documentation](../index.html#client). For
/// the `async` version, see [`asynchronous::Client`].
///
/// ## Example
///
/// The following snippet creates a `Client` from a `Rocket` instance and
/// dispatches a local `POST /` request with a body of `Hello, world!`.
///
/// ```rust
/// use rocket::local::blocking::Client;
///
/// let rocket = rocket::ignite();
/// let client = Client::tracked(rocket).expect("valid rocket");
/// let response = client.post("/")
///     .body("Hello, world!")
///     .dispatch();
/// ```
pub struct Client {
    pub(crate) inner: Option<asynchronous::Client>,
    runtime: RefCell<tokio::runtime::Runtime>,
}

impl Client {
    fn _new(rocket: Rocket, tracked: bool) -> Result<Client, Error> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create tokio runtime");

        // Initialize the Rocket instance
        let inner = Some(runtime.block_on(asynchronous::Client::_new(rocket, tracked))?);
        Ok(Self { inner, runtime: RefCell::new(runtime) })
    }

    // WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    pub fn _test<T, F>(f: F) -> T
        where F: FnOnce(&Self, LocalRequest<'_>, LocalResponse<'_>) -> T + Send
    {
        let rocket = crate::ignite();
        let client = Client::untracked(rocket).expect("valid rocket");
        let request = client.get("/");
        let response = request.clone().dispatch();
        f(&client, request, response)
    }

    #[inline(always)]
    pub(crate) fn inner(&self) -> &asynchronous::Client {
        self.inner.as_ref().expect("internal invariant broken: self.inner is Some")
    }

    #[inline(always)]
    pub(crate) fn block_on<F, R>(&self, fut: F) -> R
        where F: std::future::Future<Output=R>,
    {
        self.runtime.borrow_mut().block_on(fut)
    }

    #[inline(always)]
    fn _rocket(&self) -> &Rocket {
        self.inner()._rocket()
    }

    #[inline(always)]
    pub(crate) fn _with_raw_cookies<F, T>(&self, f: F) -> T
        where F: FnOnce(&crate::http::private::cookie::CookieJar) -> T
    {
        self.inner()._with_raw_cookies(f)
    }

    #[inline(always)]
    pub(crate) fn _req<'c, 'u: 'c, U>(
        &'c self,
        method: Method,
        uri: U
    ) -> LocalRequest<'c>
        where U: Into<Cow<'u, str>>
    {
        LocalRequest::new(self, method, uri.into())
    }

    // Generates the public API methods, which call the private methods above.
    pub_client_impl!("use rocket::local::blocking::Client;");
}

impl Drop for Client {
    fn drop(&mut self) {
        let client = self.inner.take();
        self.block_on(async { drop(client) });
    }
}

#[cfg(doctest)]
mod doctest {
    /// ```compile_fail
    /// use rocket::local::blocking::Client;
    ///
    /// fn not_sync<T: Sync>() {};
    /// not_sync::<Client>();
    /// ```
    #[allow(dead_code)]
    fn test_not_sync() {}
}
