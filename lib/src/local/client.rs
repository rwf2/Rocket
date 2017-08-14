use {Rocket, Request};
use local::LocalRequest;
use http::Method;
use http::uri::URI;
use error::LaunchError;

/// A structure to construct requests for local dispatching.
///
/// # Usage
///
/// A `Client` is constructed via the [`new`] method from an already constructed
/// `Rocket` instance. Once a value of `Client` has been constructed, the
/// [`LocalRequest`] constructor methods ([`get`], [`put`], [`post`], and so on)
/// can be used to create a `LocalRequest` for dispaching.
///
/// See the [top-level documentation](/rocket/local/index.html) for more usage
/// information.
///
/// ## Example
///
/// The following snippet creates a `Client` from a `Rocket` instance and
/// dispathes a local request to `POST /` with a body of `Hello, world!`.
///
/// ```rust
/// use rocket::local::Client;
///
/// let rocket = rocket::ignite();
/// let client = Client::new(rocket).expect("valid rocket");
/// let response = client.post("/")
///     .body("Hello, world!")
///     .dispatch();
/// ```
///
/// [`new`]: #method.new
/// [`LocalRequest`]: /rocket/local/struct.LocalRequest.html
/// [`get`]: #method.get
/// [`put`]: #method.put
/// [`post`]: #method.post
pub struct Client {
    rocket: Rocket,
}

impl Client {
    /// Construct a new `Client` from an instance of `Rocket`.
    ///
    /// # Errors
    ///
    /// If launching the `Rocket` instance would fail, excepting network errors,
    /// the `LaunchError` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// ```
    #[inline]
    pub fn new(rocket: Rocket) -> Result<Client, LaunchError> {
        if let Some(err) = rocket.prelaunch_check() {
            return Err(err);
        }

        Ok(Client { rocket })
    }

    /// Returns the instance of `Rocket` this client is creating requests for.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let my_rocket = rocket::ignite();
    /// let client = Client::new(my_rocket).expect("valid rocket");
    ///
    /// // get the instance of `my_rocket` within `client`
    /// let my_rocket = client.rocket();
    /// ```
    #[inline(always)]
    pub fn rocket(&self) -> &Rocket {
        &self.rocket
    }

    /// Create a local `GET` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`dispatch`] on the returned request.
    ///
    /// [`dispatch`]: /rocket/local/struct.LocalRequest.html#method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.get("/hello");
    /// ```
    #[inline(always)]
    pub fn get<'c, 'u: 'c, U: Into<URI<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Get, uri)
    }

    /// Create a local `PUT` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`dispatch`] on the returned request.
    ///
    /// [`dispatch`]: /rocket/local/struct.LocalRequest.html#method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.put("/hello");
    /// ```
    #[inline(always)]
    pub fn put<'c, 'u: 'c, U: Into<URI<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Put, uri)
    }

    /// Create a local `POST` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`dispatch`] on the returned request.
    ///
    /// [`dispatch`]: /rocket/local/struct.LocalRequest.html#method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    ///
    /// let req = client.post("/hello")
    ///     .body("field=value&otherField=123")
    ///     .header(ContentType::Form);
    /// ```
    #[inline(always)]
    pub fn post<'c, 'u: 'c, U: Into<URI<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Post, uri)
    }

    /// Create a local `DELETE` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`dispatch`] on the returned request.
    ///
    /// [`dispatch`]: /rocket/local/struct.LocalRequest.html#method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.delete("/hello");
    /// ```
    #[inline(always)]
    pub fn delete<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        self.req(Method::Delete, uri)
    }

    /// Create a local `OPTIONS` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`dispatch`] on the returned request.
    ///
    /// [`dispatch`]: /rocket/local/struct.LocalRequest.html#method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.options("/hello");
    /// ```
    #[inline(always)]
    pub fn options<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        self.req(Method::Options, uri)
    }

    /// Create a local `HEAD` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`dispatch`] on the returned request.
    ///
    /// [`dispatch`]: /rocket/local/struct.LocalRequest.html#method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.head("/hello");
    /// ```
    #[inline(always)]
    pub fn head<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        self.req(Method::Head, uri)
    }

    /// Create a local `PATCH` request to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`dispatch`] on the returned request.
    ///
    /// [`dispatch`]: /rocket/local/struct.LocalRequest.html#method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.patch("/hello");
    /// ```
    #[inline(always)]
    pub fn patch<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        self.req(Method::Patch, uri)
    }

    /// Create a local request with method `method` to the URI `uri`.
    ///
    /// When dispatched, the request will be served by the instance of Rocket
    /// within `self`. The request is not dispatched automatically. To actually
    /// dispatch the request, call [`dispatch`] on the returned request.
    ///
    /// [`dispatch`]: /rocket/local/struct.LocalRequest.html#method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::Method;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.req(Method::Get, "/hello");
    /// ```
    #[inline(always)]
    pub fn req<'c, 'u: 'c, U>(&'c self, method: Method, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        let request = Request::new(&self.rocket, method, uri);
        LocalRequest::new(&self.rocket, request)
    }
}
