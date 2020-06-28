macro_rules! struct_request {
    ([$(#[$attr:meta])*] $item:item) =>
{
    /// A structure representing a local request as created by [`Client`].
    ///
    /// # Usage
    ///
    /// A `LocalRequest` value is constructed via method constructors on [`Client`].
    /// Headers can be added via the [`header`] builder method and the
    /// [`add_header`] method. Cookies can be added via the [`cookie`] builder
    /// method. The remote IP address can be set via the [`remote`] builder method.
    /// The body of the request can be set via the [`body`] builder method or
    /// [`set_body`] method.
    ///
    $(#[$attr])*
    ///
    /// # Dispatching
    ///
    /// A `LocalRequest` is dispatched by calling [`dispatch`].
    /// The `LocalRequest` is consumed and a response is returned.
    ///
    /// Note that `LocalRequest` implements `Clone`. As such, if the
    /// same request needs to be dispatched multiple times, the request can first be
    /// cloned and then dispatched: `request.clone().dispatch()`.
    ///
    /// [`Client`]: super::Client
    /// [`header`]: #method.header
    /// [`add_header`]: #method.add_header
    /// [`cookie`]: #method.cookie
    /// [`remote`]: #method.remote
    /// [`body`]: #method.body
    /// [`set_body`]: #method.set_body
    /// [`dispatch`]: #method.dispatch
    $item
}}

macro_rules! impl_request {
    ($import:literal $(@$prefix:tt $suffix:tt)? $name:ident) =>
{
    impl<'c> $name<'c> {
        /// Retrieves the inner `Request` as seen by Rocket.
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = $import]
        /// use rocket::Request;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let req = client.get("/");
        /// let inner: &Request = req.inner();
        /// # });
        /// ```
        #[inline(always)]
        pub fn inner(&self) -> &Request<'c> {
            self._request()
        }

        /// Add a header to this request.
        ///
        /// Any type that implements `Into<Header>` can be used here. Among
        /// others, this includes [`ContentType`] and [`Accept`].
        ///
        /// [`ContentType`]: crate::http::ContentType
        /// [`Accept`]: crate::http::Accept
        ///
        /// # Examples
        ///
        /// Add the Content-Type header:
        ///
        /// ```rust
        #[doc = $import]
        /// use rocket::http::ContentType;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let req = client.get("/").header(ContentType::JSON);
        /// # });
        /// ```
        #[inline]
        pub fn header<H>(mut self, header: H) -> Self
            where H: Into<crate::http::Header<'static>>
        {
            self._request_mut().add_header(header.into());
            self
        }

        /// Adds a header to this request without consuming `self`.
        ///
        /// # Examples
        ///
        /// Add the Content-Type header:
        ///
        /// ```rust
        #[doc = $import]
        /// use rocket::http::ContentType;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let mut req = client.get("/");
        /// req.add_header(ContentType::JSON);
        /// # });
        /// ```
        #[inline]
        pub fn add_header<H>(&mut self, header: H)
            where H: Into<crate::http::Header<'static>>
        {
            self._request_mut().add_header(header.into());
        }

        /// Set the remote address of this request.
        ///
        /// # Examples
        ///
        /// Set the remote address to "8.8.8.8:80":
        ///
        /// ```rust
        #[doc = $import]
        ///
        /// # Client::_test(|client| {
        /// let address = "8.8.8.8:80".parse().unwrap();
        ///
        /// let client: Client = client;
        /// let req = client.get("/").remote(address);
        /// # });
        /// ```
        #[inline]
        pub fn remote(mut self, address: std::net::SocketAddr) -> Self {
            self._request_mut().set_remote(address);
            self
        }

        /// Add a cookie to this request.
        ///
        /// # Examples
        ///
        /// Add `user_id` cookie:
        ///
        /// ```rust
        #[doc = $import]
        /// use rocket::http::Cookie;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let req = client.get("/")
        ///     .cookie(Cookie::new("username", "sb"))
        ///     .cookie(Cookie::new("user_id", "12"));
        /// # });
        /// ```
        #[inline]
        pub fn cookie(self, cookie: crate::http::Cookie<'_>) -> Self {
            self._request().cookies().add_original(cookie.into_owned());
            self
        }

        /// Add all of the cookies in `cookies` to this request.
        ///
        /// # Examples
        ///
        /// Add `user_id` cookie:
        ///
        /// ```rust
        #[doc = $import]
        /// use rocket::http::Cookie;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let cookies = vec![Cookie::new("a", "b"), Cookie::new("c", "d")];
        /// let req = client.get("/").cookies(cookies);
        /// # });
        /// ```
        #[inline]
        pub fn cookies(self, cookies: Vec<crate::http::Cookie<'_>>) -> Self {
            for cookie in cookies {
                self._request().cookies().add_original(cookie.into_owned());
            }

            self
        }

        /// Add a [private cookie] to this request.
        ///
        /// This method is only available when the `private-cookies` feature is
        /// enabled.
        ///
        /// [private cookie]: crate::http::Cookies::add_private()
        ///
        /// # Examples
        ///
        /// Add `user_id` as a private cookie:
        ///
        /// ```rust
        #[doc = $import]
        /// use rocket::http::Cookie;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let req = client.get("/").private_cookie(Cookie::new("user_id", "sb"));
        /// # });
        /// ```
        #[inline]
        #[cfg(feature = "private-cookies")]
        pub fn private_cookie(self, cookie: crate::http::Cookie<'static>) -> Self {
            self._request().cookies().add_original_private(cookie);
            self
        }

        /// Set the body (data) of the request.
        ///
        /// # Examples
        ///
        /// Set the body to be a JSON structure; also sets the Content-Type.
        ///
        /// ```rust
        #[doc = $import]
        /// use rocket::http::ContentType;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let req = client.post("/")
        ///     .header(ContentType::JSON)
        ///     .body(r#"{ "key": "value", "array": [1, 2, 3], }"#);
        /// # });
        /// ```
        #[inline]
        pub fn body<S: AsRef<[u8]>>(mut self, body: S) -> Self {
            // TODO: For CGI, we want to be able to set the body to be stdin
            // without actually reading everything into a vector. Can we allow
            // that here while keeping the simplicity? Looks like it would
            // require us to reintroduce a NetStream::Local(Box<Read>) or
            // something like that.
            *self._body_mut() = body.as_ref().into();
            self
        }

        /// Set the body (data) of the request without consuming `self`.
        ///
        /// # Examples
        ///
        /// Set the body to be a JSON structure; also sets the Content-Type.
        ///
        /// ```rust
        #[doc = $import]
        /// use rocket::http::ContentType;
        ///
        /// # Client::_test(|client| {
        /// let client: Client = client;
        /// let mut req = client.post("/").header(ContentType::JSON);
        /// req.set_body(r#"{ "key": "value", "array": [1, 2, 3], }"#);
        /// # });
        /// ```
        #[inline]
        pub fn set_body<S: AsRef<[u8]>>(&mut self, body: S) {
            *self._body_mut() = body.as_ref().into();
        }

        /// Dispatches the request, returning the response.
        ///
        /// This method consumes `self` and is the preferred mechanism for
        /// dispatching.
        ///
        /// # Example
        ///
        /// ```rust
        /// use rocket::local::asynchronous::Client;
        ///
        /// # rocket::async_test(async {
        /// let client = Client::new(rocket::ignite()).await.unwrap();
        /// let response = client.get("/").dispatch();
        /// # });
        /// ```
        #[inline(always)]
        pub $($prefix)? fn dispatch(self) -> LocalResponse<'c> {
            self._dispatch()$(.$suffix)?
        }
    }

    impl std::fmt::Debug for $name<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self._request().fmt(f)
        }
    }

    // TODO: Add test to check that `LocalRequest` is `Clone`.
}}
