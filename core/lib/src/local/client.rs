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
        /// # Client::_test(|client, _, _| {
        /// let client: &Client = client;
        #[doc = $use_it]
        /// # });
        /// ```
        #[inline(always)]
        pub fn $f<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
            where U: TryInto<Origin<'u>> + fmt::Display
        {
            self.req($method, uri)
        }
    )
}

macro_rules! pub_client_impl {
    ($import:literal $(@$prefix:tt $suffix:tt)?) =>
{
    /// Construct a new `Client` from an instance of `Rocket` _with_ cookie
    /// tracking. This is typically the desired mode of operation for testing.
    ///
    /// # Cookie Tracking
    ///
    /// With cookie tracking enabled, a `Client` propagates cookie changes made
    /// by responses to previously dispatched requests. In other words,
    /// succeeding requests reflect changes (additions and removals) made by any
    /// prior responses.
    ///
    /// Cookie tracking requires synchronization between dispatches. **As such,
    /// cookie tracking _should not_ be enabled if a local client is being used
    /// to serve requests on multiple threads.**
    ///
    /// # Errors
    ///
    /// If launching the `Rocket` instance would fail, excepting network errors,
    /// the `Error` is returned.
    ///
    /// ```rust,no_run
    #[doc = $import]
    ///
    /// let rocket = rocket::build();
    /// let client = Client::tracked(rocket);
    /// ```
    #[inline(always)]
    pub $($prefix)? fn tracked<P: Phase>(rocket: Rocket<P>) -> Result<Self, Error> {
        Self::_new(rocket, true) $(.$suffix)?
    }

    /// Construct a new `Client` from an instance of `Rocket` _without_
    /// cookie tracking.
    ///
    /// # Cookie Tracking
    ///
    /// Unlike the [`tracked()`](Client::tracked()) constructor, a `Client`
    /// returned from this method _does not_ automatically propagate cookie
    /// changes and thus requires no synchronization between dispatches.
    ///
    /// # Errors
    ///
    /// If launching the `Rocket` instance would fail, excepting network
    /// errors, the `Error` is returned.
    ///
    /// ```rust,no_run
    #[doc = $import]
    ///
    /// let rocket = rocket::build();
    /// let client = Client::untracked(rocket);
    /// ```
    pub $($prefix)? fn untracked<P: Phase>(rocket: Rocket<P>) -> Result<Self, Error> {
        Self::_new(rocket, false) $(.$suffix)?
    }

    #[doc(hidden)]
    pub $($prefix)? fn debug_with(routes: Vec<crate::Route>) -> Result<Self, Error> {
        let rocket = crate::custom(crate::Config::debug_default());
        Self::debug(rocket.mount("/", routes)) $(.$suffix)?
    }

    #[doc(hidden)]
    pub $($prefix)? fn debug(rocket: Rocket<crate::Build>) -> Result<Self, Error> {
        use crate::config;

        let figment = rocket.figment().clone()
            .merge((config::Config::LOG_LEVEL, config::LogLevel::Debug))
            .select(config::Config::DEBUG_PROFILE);

        Self::tracked(rocket.configure(figment)) $(.$suffix)?
    }

    /// Deprecated alias to [`Client::tracked()`].
    #[deprecated(
        since = "0.5.0",
        note = "choose between `Client::untracked()` and `Client::tracked()`"
    )]
    pub $($prefix)? fn new<P: Phase>(rocket: Rocket<P>) -> Result<Self, Error> {
        Self::tracked(rocket) $(.$suffix)?
    }

    /// Returns a reference to the `Rocket` this client is creating requests
    /// for.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $import]
    ///
    /// # Client::_test(|client, _, _| {
    /// let client: &Client = client;
    /// let rocket = client.rocket();
    /// # });
    /// ```
    #[inline(always)]
    pub fn rocket(&self) -> &Rocket<Orbit> {
        &*self._rocket()
    }

    /// Returns a cookie jar containing all of the cookies this client is
    /// currently tracking.
    ///
    /// If cookie tracking is disabled, the returned jar will always be empty.
    /// Otherwise, it will contains all of the cookies collected from responses
    /// to requests dispatched by this client that have not expired.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $import]
    ///
    /// # Client::_test(|client, _, _| {
    /// let client: &Client = client;
    /// let cookie = client.cookies();
    /// # });
    /// ```
    #[inline(always)]
    pub fn cookies(&self) -> crate::http::CookieJar<'_> {
        let config = &self.rocket().config();
        let jar = self._with_raw_cookies(|jar| jar.clone());
        crate::http::CookieJar::from(jar, config)
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
    /// # Client::_test(|client, _, _| {
    /// let client: &Client = client;
    /// client.req(Method::Get, "/hello");
    /// # });
    /// ```
    #[inline(always)]
    pub fn req<'c, 'u: 'c, U>(
        &'c self,
        method: Method,
        uri: U
    ) -> LocalRequest<'c>
        where U: TryInto<Origin<'u>> + fmt::Display
    {
        self._req(method, uri)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn _ensure_impls_exist() {
        fn is_send<T: Send>() {}
        is_send::<Self>();

        fn is_debug<T: std::fmt::Debug>() {}
        is_debug::<Self>();
    }
}}
