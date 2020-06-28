//! A structure to construct requests for local dispatching.
//!
//! # Usage
//!
//! A `Client` is constructed via the [`new()`] or [`untracked()`] methods from
//! an already constructed `Rocket` instance. Once a value of `Client` has been
//! constructed, the [`LocalRequest`] constructor methods ([`get()`], [`put()`],
//! [`post()`], and so on) can be used to create a `LocalRequest` for
//! dispatching.
//!
//! See the [top-level documentation](crate::local) for more usage information.
//!
//! ## Cookie Tracking
//!
//! A `Client` constructed using [`new()`] propagates cookie changes made by
//! responses to previously dispatched requests. In other words, if a previously
//! dispatched request resulted in a response that adds a cookie, any future
//! requests will contain that cookie. Similarly, cookies removed by a response
//! won't be propagated further.
//!
//! This is typically the desired mode of operation for a `Client` as it removes
//! the burden of manually tracking cookies. Under some circumstances, however,
//! disabling this tracking may be desired. In these cases, use the
//! [`untracked()`](Client::untracked()) constructor to create a `Client` that
//! _will not_ track cookies.
//!
//! ### Synchronization
//!
//! While `Client` implements `Sync`, using it in a multithreaded environment
//! while tracking cookies can result in surprising, non-deterministic behavior.
//! This is because while cookie modifications are serialized, the exact
//! ordering depends on when requests are dispatched. Specifically, when cookie
//! tracking is enabled, all request dispatches are serialized, which in-turn
//! serializes modifications to the internally tracked cookies.
//!
//! If possible, refrain from sharing a single instance of `Client` across
//! multiple threads. Instead, prefer to create a unique instance of `Client`
//! per thread. If it's not possible, ensure that either you are not depending
//! on cookies, the ordering of their modifications, or both, or have arranged
//! for dispatches to occur in a deterministic ordering.
//!
//! ## Example
//!
//! The following snippet creates a `Client` from a `Rocket` instance and
//! dispatches a local request to `POST /` with a body of `Hello, world!`.
//!
//! ```rust
//! use rocket::local::asynchronous::Client;
//!
//! # rocket::async_test(async {
//! let rocket = rocket::ignite();
//! let client = Client::new(rocket).await.expect("valid rocket");
//! let response = client.post("/")
//!     .body("Hello, world!")
//!     .dispatch().await;
//! # });
//! ```
//!
//! [`new()`]: #method.new
//! [`untracked()`]: #method.untracked
//! [`get()`]: #method.get
//! [`put()`]: #method.put
//! [`post()`]: #method.post

macro_rules! req_method {
    ($import:literal, $NAME:literal, $f:ident, $method:expr) => (
        req_method!(@
            $import,
            $NAME,
            concat!("let req = client.", stringify!($f), r#"("/hello");"#),
            $f,
            $method
        );
    );

    (@$import:literal, $NAME:literal, $use_it:expr, $f:ident, $method:expr) => (
        /// Create a local `
        #[doc = $NAME]
        /// ` request to the URI `uri`.
        ///
        /// When dispatched, the request will be served by the instance of Rocket
        /// within `self`. The request is not dispatched automatically. To actually
        /// dispatch the request, call [`LocalRequest::dispatch()`] on the returned
        /// request.
        ///
        /// # Example
        ///
        /// ```rust,no_run
        #[doc = $import]
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        #[doc = $use_it]
        /// # });
        /// ```
        #[inline(always)]
        pub fn $f<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
            where U: Into<Cow<'u, str>>
        {
            self.req($method, uri)
        }
    )
}

macro_rules! impl_client {
    ($import:literal $(@$prefix:tt $suffix:tt)? $name:ident) =>
{
    impl $name {
        /// Construct a new `Client` from an instance of `Rocket` with cookie
        /// tracking.
        ///
        /// # Cookie Tracking
        ///
        /// By default, a `Client` propagates cookie changes made by responses
        /// to previously dispatched requests. In other words, if a previously
        /// dispatched request resulted in a response that adds a cookie, any
        /// future requests will contain the new cookies. Similarly, cookies
        /// removed by a response won't be propagated further.
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
        /// ```rust,no_run
        #[doc = $import]
        ///
        /// let rocket = rocket::ignite();
        /// let client = Client::new(rocket);
        /// ```
        #[inline(always)]
        pub $($prefix)? fn new(rocket: Rocket) -> Result<Self, LaunchError> {
            Self::_new(rocket, true) $(.$suffix)?
        }

        /// Construct a new `Client` from an instance of `Rocket` _without_
        /// cookie tracking.
        ///
        /// # Cookie Tracking
        ///
        /// Unlike the [`new()`](Client::new()) constructor, a `Client` returned
        /// from this method _does not_ automatically propagate cookie changes.
        ///
        /// # Errors
        ///
        /// If launching the `Rocket` instance would fail, excepting network
        /// errors, the `LaunchError` is returned.
        ///
        /// ```rust,no_run
        #[doc = $import]
        ///
        /// let rocket = rocket::ignite();
        /// let client = Client::untracked(rocket);
        /// ```
        pub $($prefix)? fn untracked(rocket: Rocket) -> Result<Self, LaunchError> {
            Self::_new(rocket, true) $(.$suffix)?
        }

        /// Returns a reference to the `Rocket` this client is creating requests
        /// for.
        ///
        /// # Example
        ///
        /// ```rust,no_run
        #[doc = $import]
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let rocket = client.rocket();
        /// # });
        /// ```
        #[inline(always)]
        pub fn rocket(&self) -> &Rocket {
            &*self._cargo()
        }

        /// Returns a reference to the `Cargo` of the `Rocket` this client is
        /// creating requests for.
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = $import]
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let cargo = client.cargo();
        /// # });
        /// ```
        #[inline(always)]
        pub fn cargo(&self) -> &Cargo {
            self._cargo()
        }

        req_method!($import, "GET", get, Method::Get);
        req_method!($import, "PUT", put, Method::Put);
        req_method!($import, "POST", post, Method::Post);
        req_method!($import, "DELETE", delete, Method::Delete);
        req_method!($import, "OPTIONS", options, Method::Options);
        req_method!($import, "HEAD", head, Method::Head);
        req_method!($import, "PATCH", patch, Method::Patch);

        /// Create a local `GET` request to the URI `uri`.
        ///
        /// When dispatched, the request will be served by the instance of
        /// Rocket within `self`. The request is not dispatched automatically.
        /// To actually dispatch the request, call [`LocalRequest::dispatch()`]
        /// on the returned request.
        ///
        /// # Example
        ///
        /// ```rust,no_run
        #[doc = $import]
        /// use rocket::http::Method;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// client.req(Method::Get, "/hello");
        /// # });
        /// ```
        #[inline(always)]
        pub fn req<'c, 'u: 'c, U>(
            &'c self,
            method: Method,
            uri: U
        ) -> LocalRequest<'c>
            where U: Into<Cow<'u, str>>
        {
            self._req(method, uri)
        }
    }
}}
