use std::borrow::Cow;
use std::cell::RefCell;

use crate::error::LaunchError;
use crate::http::Method;
use crate::local::blocking::LocalRequest;
use crate::rocket::{Rocket, Manifest};

pub struct Client {
    pub(crate) inner: crate::local::Client,
    runtime: RefCell<tokio::runtime::Runtime>,
}

impl Client {
    fn _new(rocket: Rocket, tracked: bool) -> Result<Client, LaunchError> {
        let mut runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .expect("create tokio runtime");

        // Initialize the Rocket instance
        let inner = runtime.block_on(crate::local::Client::_new(rocket, tracked))?;

        Ok(Self { inner, runtime: RefCell::new(runtime) })
    }

    pub(crate) fn block_on<F, R>(&self, fut: F) -> R
    where
        F: std::future::Future<Output=R>,
    {
        self.runtime.borrow_mut().block_on(fut)
    }

    /// Construct a new `Client` from an instance of `Rocket` with cookie
    /// tracking.
    ///
    /// # Cookie Tracking
    ///
    /// By default, a `Client` propagates cookie changes made by responses to
    /// previously dispatched requests. In other words, if a previously
    /// dispatched request resulted in a response that adds a cookie, any future
    /// requests will contain the new cookies. Similarly, cookies removed by a
    /// response won't be propagated further.
    ///
    /// This is typically the desired mode of operation for a `Client` as it
    /// removes the burden of manually tracking cookies. Under some
    /// circumstances, however, disabling this tracking may be desired. The
    /// [`untracked()`](Client::untracked()) method creates a `Client` that
    /// _will not_ track cookies.
    ///
    /// # Errors
    ///
    /// If launching the `Rocket` instance would fail, excepting network errors,
    /// the `LaunchError` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// ```
    #[inline(always)]
    pub fn new(rocket: Rocket) -> Result<Client, LaunchError> {
        Self::_new(rocket, true)
    }

    /// Construct a new `Client` from an instance of `Rocket` _without_ cookie
    /// tracking.
    ///
    /// # Cookie Tracking
    ///
    /// Unlike the [`new()`](Client::new()) constructor, a `Client` returned
    /// from this method _does not_ automatically propagate cookie changes.
    ///
    /// # Errors
    ///
    /// If launching the `Rocket` instance would fail, excepting network errors,
    /// the `LaunchError` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::untracked(rocket::ignite()).expect("valid rocket");
    /// ```
    #[inline(always)]
    pub fn untracked(rocket: Rocket) -> Result<Client, LaunchError> {
        Self::_new(rocket, false)
    }

    /// Returns a reference to the `Manifest` of the `Rocket` this client is
    /// creating requests for.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let my_rocket = rocket::ignite();
    /// let client = Client::new(my_rocket).expect("valid rocket");
    ///
    /// // get access to the manifest within `client`
    /// let manifest = client.manifest();
    /// ```
    #[inline(always)]
    pub fn manifest(&self) -> &Manifest {
        self.inner.manifest()
    }

    /// Create a local `GET` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
    /// request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.get("/hello");
    /// ```
    #[inline(always)]
    pub fn get<'c, 'u: 'c, U: Into<Cow<'u, str>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Get, uri)
    }

    /// Create a local `PUT` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
    /// request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.put("/hello");
    /// ```
    #[inline(always)]
    pub fn put<'c, 'u: 'c, U: Into<Cow<'u, str>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Put, uri)
    }

    /// Create a local `POST` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
    /// request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    ///
    /// let req = client.post("/hello")
    ///     .body("field=value&otherField=123")
    ///     .header(ContentType::Form);
    /// ```
    #[inline(always)]
    pub fn post<'c, 'u: 'c, U: Into<Cow<'u, str>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Post, uri)
    }

    /// Create a local `DELETE` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
    /// request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.delete("/hello");
    /// ```
    #[inline(always)]
    pub fn delete<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<Cow<'u, str>>
    {
        self.req(Method::Delete, uri)
    }

    /// Create a local `OPTIONS` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
    /// request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.options("/hello");
    /// ```
    #[inline(always)]
    pub fn options<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<Cow<'u, str>>
    {
        self.req(Method::Options, uri)
    }

    /// Create a local `HEAD` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
    /// request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.head("/hello");
    /// ```
    #[inline(always)]
    pub fn head<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<Cow<'u, str>>
    {
        self.req(Method::Head, uri)
    }

    /// Create a local `PATCH` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
    /// request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.patch("/hello");
    /// ```
    #[inline(always)]
    pub fn patch<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<Cow<'u, str>>
    {
        self.req(Method::Patch, uri)
    }

    /// Create a local request with method `method` to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
    /// request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    /// use rocket::http::Method;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.req(Method::Get, "/hello");
    /// ```
    #[inline(always)]
    pub fn req<'c, 'u: 'c, U>(&'c self, method: Method, uri: U) -> LocalRequest<'c>
        where U: Into<Cow<'u, str>>
    {
        LocalRequest::new(self, method, uri.into())
    }
}

#[cfg(test)]
mod test {
    // TODO: assert that client is !Sync - e.g. with static_assertions or another tool
}
