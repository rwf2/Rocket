macro_rules! getter_method {
    ($doc_prelude:literal, $desc:literal, $f:ident -> $r:ty) => (
        getter_method!(@$doc_prelude, $f, $desc, $r,
            concat!("let ", stringify!($f), " = response.", stringify!($f), "();"));
    );
    (@$doc_prelude:literal, $f:ident, $desc:expr, $r:ty, $use_it:expr) => (
        /// Returns the
        #[doc = $desc]
        /// of `self`.
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = $doc_prelude]
        ///
        /// # Client::_test(|_, _, response| {
        /// let response: LocalResponse = response;
        #[doc = $use_it]
        /// # });
        /// ```
        #[inline(always)]
        pub fn $f(&self) -> $r {
            self._response().$f()
        }
    )
}

macro_rules! pub_response_impl {
    ($doc_prelude:literal $($prefix:tt $suffix:tt)?) =>
{
    getter_method!($doc_prelude, "HTTP status",
        status -> crate::http::Status);

    getter_method!($doc_prelude, "Content-Type, if a valid one is set,",
        content_type -> Option<crate::http::ContentType>);

    getter_method!($doc_prelude, "HTTP headers",
        headers -> &crate::http::HeaderMap<'_>);

    /// Return a cookie jar containing the HTTP cookies in the response.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $doc_prelude]
    ///
    /// # Client::_test(|_, _, response| {
    /// let response: LocalResponse = response;
    /// let string = response.cookies();
    /// # });
    /// ```
    #[inline(always)]
    pub fn cookies(&self) -> &crate::http::CookieJar<'_> {
        self._cookies()
    }

    getter_method!($doc_prelude, "response body, if there is one,",
        body -> &crate::response::Body<'_>);

    /// Consumes `self` and reads the entirety of its body into a string.
    ///
    /// If reading fails, the body contains invalid UTF-8 characters, or the
    /// body is unset in the response, returns `None`. Otherwise, returns
    /// `Some`. The string may be empty if the body is empty.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $doc_prelude]
    ///
    /// # Client::_test(|_, _, response| {
    /// let response: LocalResponse = response;
    /// let string = response.into_string();
    /// # });
    /// ```
    #[inline(always)]
    pub $($prefix)? fn into_string(self) -> Option<String> {
        if self._response().body().is_none() {
            return None;
        }

        self._into_string() $(.$suffix)? .ok()
    }

    /// Consumes `self` and reads the entirety of its body into a `Vec` of
    /// bytes.
    ///
    /// If reading fails or the body is unset in the response, returns `None`.
    /// Otherwise, returns `Some`. The returned vector may be empty if the body
    /// is empty.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $doc_prelude]
    ///
    /// # Client::_test(|_, _, response| {
    /// let response: LocalResponse = response;
    /// let bytes = response.into_bytes();
    /// # });
    /// ```
    #[inline(always)]
    pub $($prefix)? fn into_bytes(self) -> Option<Vec<u8>> {
        if self._response().body().is_none() {
            return None;
        }

        self._into_bytes() $(.$suffix)? .ok()
    }

    /// Consumes `self` and deserializes its body as JSON without buffering in
    /// memory.
    ///
    /// If deserialization fails or the body is unset in the response, returns
    /// `None`. Otherwise, returns `Some`.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $doc_prelude]
    /// use rocket::serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Task {
    ///     id: usize,
    ///     complete: bool,
    ///     text: String,
    /// }
    ///
    /// # Client::_test(|_, _, response| {
    /// let response: LocalResponse = response;
    /// let task = response.into_json::<Task>();
    /// # });
    /// ```
    #[cfg(feature = "json")]
    #[cfg_attr(nightly, doc(cfg(feature = "json")))]
    pub $($prefix)? fn into_json<T>(self) -> Option<T>
        where T: Send + serde::de::DeserializeOwned + 'static
    {
        if self._response().body().is_none() {
            return None;
        }

        self._into_json() $(.$suffix)?
    }

    /// Consumes `self` and deserializes its body as MessagePack without
    /// buffering in memory.
    ///
    /// If deserialization fails or the body is unset in the response, returns
    /// `None`. Otherwise, returns `Some`.
    ///
    /// # Example
    ///
    /// ```rust
    #[doc = $doc_prelude]
    /// use rocket::serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Task {
    ///     id: usize,
    ///     complete: bool,
    ///     text: String,
    /// }
    ///
    /// # Client::_test(|_, _, response| {
    /// let response: LocalResponse = response;
    /// let task = response.into_msgpack::<Task>();
    /// # });
    /// ```
    #[cfg(feature = "msgpack")]
    #[cfg_attr(nightly, doc(cfg(feature = "msgpack")))]
    pub $($prefix)? fn into_msgpack<T>(self) -> Option<T>
        where T: Send + serde::de::DeserializeOwned + 'static
    {
        if self._response().body().is_none() {
            return None;
        }

        self._into_msgpack() $(.$suffix)?
    }

    /// Checks if a response was generted by a specific route type. This only returns true if the route
    /// actually generated the response, and a catcher was _not_ run. See [`was_attempted_by`] to
    /// check if a route was attempted, but may not have generated the response
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{get, routes};
    /// #[get("/")]
    /// fn index() -> &'static str { "Hello World" }
    #[doc = $doc_prelude]
    /// # Client::_test_with(|r| r.mount("/", routes![index]), |_, _, response| {
    /// let response: LocalResponse = response;
    /// assert!(response.was_routed_by::<index>());
    /// # });
    /// ```
    ///
    /// # Other Route types
    ///
    /// [`FileServer`](crate::fs::FileServer) implementes `RouteType`, so a route that should
    /// return a static file can be checked against it. Libraries which provide custom Routes should
    /// implement `RouteType`, see [`RouteType`](crate::route::RouteType) for more information.
    pub fn was_routed_by<T: crate::route::RouteType>(&self) -> bool {
        // If this request was caught, the route in `.route()` did NOT generate this response.
        if self._request().catcher().is_some() {
            false
        } else if let Some(route_type) = self._request().route().map(|r| r.route_type).flatten() {
            route_type == std::any::TypeId::of::<T>()
        } else {
            false
        }
    }

    /// Checks if a request was routed to a specific route type. This will return true for routes
    /// that were attempted, _but not actually called_. This enables a test to verify that a route
    /// was attempted, even if another route actually generated the response, e.g. an
    /// authenticated route will typically defer to an error catcher if the request does not have
    /// the proper authentication. This makes it possible to verify that a request was routed to
    /// the authentication route, even if the response was eventaully generated by another route or
    /// a catcher.
    ///
    /// # Example
    ///
    // WARNING: this doc-test is NOT run, because cargo test --doc does not run doc-tests for items
    // only available during tests.
    /// ```rust
    /// # use rocket::{get, routes, async_trait, request::{Request, Outcome, FromRequest}};
    /// # struct WillFail {}
    /// # #[async_trait]
    /// # impl<'r> FromRequest<'r> for WillFail {
    /// #     type Error = ();
    /// #     async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
    /// #         Outcome::Forward(())
    /// #     }
    /// # }
    /// #[get("/", rank = 2)]
    /// fn index1(guard: WillFail) -> &'static str { "Hello World" }
    /// #[get("/")]
    /// fn index2() -> &'static str { "Hello World" }
    #[doc = $doc_prelude]
    /// # Client::_test_with(|r| r.mount("/", routes![index1, index2]), |_, _, response| {
    /// let response: LocalResponse = response;
    /// assert!(response.was_attempted_by::<index1>());
    /// assert!(response.was_attempted_by::<index2>());
    /// assert!(response.was_routed_by::<index2>());
    /// # });
    /// ```
    ///
    /// # Other Route types
    ///
    /// [`FileServer`](crate::fs::FileServer) implementes `RouteType`, so a route that should
    /// return a static file can be checked against it. Libraries which provide custom Routes should
    /// implement `RouteType`, see [`RouteType`](crate::route::RouteType) for more information.
    ///
    /// # Note
    ///
    /// This method is marked as `cfg(test)`, and is therefore only available in unit and
    /// integration tests. This is because the list of routes attempted is only collected in these
    /// testing environments, to minimize performance impacts during normal operation.
    #[cfg(test)]
    pub fn was_attempted_by<T: crate::route::RouteType>(&self) -> bool {
        self._request().route_path(|path| path.iter().any(|r|
                r.route_type == Some(std::any::TypeId::of::<T>())
            ))
    }

    /// Checks if a route was caught by a specific route type
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{catch, catchers};
    /// #[catch(404)]
    /// fn default_404() -> &'static str { "Hello World" }
    #[doc = $doc_prelude]
    /// # Client::_test_with(|r| r.register("/", catchers![default_404]), |_, _, response| {
    /// let response: LocalResponse = response;
    /// assert!(response.was_caught_by::<default_404>());
    /// # });
    /// ```
    ///
    /// # Rocket's default catcher
    ///
    /// The default catcher has a `CatcherType` of [`DefaultCatcher`](crate::catcher::DefaultCatcher)
    pub fn was_caught_by<T: crate::catcher::CatcherType>(&self) -> bool {
        if let Some(catcher_type) = self._request().catcher().map(|r| r.catcher_type).flatten() {
            catcher_type == std::any::TypeId::of::<T>()
        } else {
            false
        }
    }

    /// Checks if a route was caught by a catcher
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::get;
    #[doc = $doc_prelude]
    /// # Client::_test(|_, _, response| {
    /// let response: LocalResponse = response;
    /// assert!(response.was_caught())
    /// # });
    /// ```
    pub fn was_caught(&self) -> bool {
        self._request().catcher().is_some()
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn _ensure_impls_exist() {
        fn is_debug<T: std::fmt::Debug>() {}
        is_debug::<Self>();
    }
}}
