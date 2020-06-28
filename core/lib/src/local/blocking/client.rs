use std::borrow::Cow;
use std::cell::RefCell;
use crate::error::LaunchError;
use crate::http::Method;
use crate::local::{asynchronous, blocking::LocalRequest};
use crate::rocket::{Rocket, Cargo};

struct_client! { [
///
/// ## Example
///
/// The following snippet creates a `Client` from a `Rocket` instance and
/// dispatches a local request to `POST /` with a body of `Hello, world!`.
///
/// ```rust
/// use rocket::local::blocking::Client;
///
/// let rocket = rocket::ignite();
/// let client = Client::new(rocket).expect("valid rocket");
/// let response = client.post("/")
///     .body("Hello, world!")
///     .dispatch();
/// ```
]
pub struct Client {
    pub(crate) inner: asynchronous::Client,
    runtime: RefCell<tokio::runtime::Runtime>,
}
}

impl Client {
    fn _new(rocket: Rocket, tracked: bool) -> Result<Client, LaunchError> {
        let mut runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .expect("create tokio runtime");

        // Initialize the Rocket instance
        let inner = runtime.block_on(asynchronous::Client::_new(rocket, tracked))?;
        Ok(Self { inner, runtime: RefCell::new(runtime) })
    }

    #[doc(hidden)]
    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    pub fn _test<T, F: FnOnce(Self) -> T + Send>(f: F) -> T {
        let rocket = crate::ignite();
        let client = Client::new(rocket).expect("valid rocket");
        f(client)
    }

    #[inline(always)]
    pub(crate) fn block_on<F, R>(&self, fut: F) -> R
        where F: std::future::Future<Output=R>,
    {
        self.runtime.borrow_mut().block_on(fut)
    }

    #[inline(always)]
    fn _cargo(&self) -> &Cargo {
        self.inner._cargo()
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
}

impl_client!("use rocket::local::blocking::Client;" Client);

#[cfg(doctest)]
mod doctest {
    /// ```no_run
    /// // Just to ensure we get the path/form right in the following tests.
    /// use rocket::local::blocking::Client;
    ///
    /// fn test<T>() {};
    /// test::<Client>();
    ///
    /// fn is_send<T: Send>() {};
    /// is_send::<Client>();
    /// ```
    ///
    /// ```compile_fail
    /// use rocket::local::blocking::Client;
    ///
    /// fn not_sync<T: Sync>() {};
    /// not_sync::<Client>();
    /// ```
    #[allow(dead_code)]
    fn test_not_sync_or_send() {}
}
