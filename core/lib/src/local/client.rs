use {Rocket, Request, Response};
use local::LocalRequest;
use http::{Method, CookieJar, uri::Uri};
use error::LaunchError;
use std::cell::RefCell;

/// A structure to construct requests for local dispatching.
///
/// # Usage
///
/// A `Client` is constructed via the [`new`] or [`untracked`] methods from an
/// already constructed `Rocket` instance. Once a value of `Client` has been
/// constructed, the [`LocalRequest`] constructor methods ([`get`], [`put`],
/// [`post`], and so on) can be used to create a `LocalRequest` for dispatching.
///
/// See the [top-level documentation](/rocket/local/index.html) for more usage
/// information.
///
/// ## Cookie Tracking
///
/// A `Client` constructed using [`new`] propagates cookie changes made by
/// responses to previously dispatched requests. In other words, if a previously
/// dispatched request resulted in a response that adds a cookie, any future
/// requests will contain that cookie. Similarly, cookies removed by a response
/// won't be propagated further.
///
/// This is typically the desired mode of operation for a `Client` as it removes
/// the burden of manually tracking cookies. Under some circumstances, however,
/// disabling this tracking may be desired. In these cases, use the
/// [`untracked`](Client::untracked()) constructor to create a `Client` that
/// _will not_ track cookies.
///
/// ## Example
///
/// The following snippet creates a `Client` from a `Rocket` instance and
/// dispatches a local request to `POST /` with a body of `Hello, world!`.
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
/// [`untracked`]: #method.untracked
/// [`LocalRequest`]: /rocket/local/struct.LocalRequest.html
/// [`get`]: #method.get
/// [`put`]: #method.put
/// [`post`]: #method.post
pub struct Client {
    rocket: Rocket,
    cookies: Option<RefCell<CookieJar>>,
}

impl Client {
    /// Constructs a new `Client`. If `tracked` is `true`, an empty `CookieJar`
    /// is created for cookie tracking. Otherwise, the internal `CookieJar` is
    /// set to `None`.
    fn _new(rocket: Rocket, tracked: bool) -> Result<Client, LaunchError> {
        if let Some(err) = rocket.prelaunch_check() {
            return Err(err);
        }

        let cookies = match tracked {
            true => Some(RefCell::new(CookieJar::new())),
            false => None
        };

        Ok(Client { rocket, cookies })
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
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// ```
    #[inline(always)]
    pub fn new(rocket: Rocket) -> Result<Client, LaunchError> {
        Client::_new(rocket, true)
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
    /// use rocket::local::Client;
    ///
    /// let client = Client::untracked(rocket::ignite()).expect("valid rocket");
    /// ```
    #[inline(always)]
    pub fn untracked(rocket: Rocket) -> Result<Client, LaunchError> {
        Client::_new(rocket, false)
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

    // If `self` is tracking cookies, updates the internal cookie jar with the
    // changes reflected by `response`.
    pub(crate) fn update_cookies(&self, response: &Response) {
        if let Some(ref jar) = self.cookies {
            let mut jar = jar.borrow_mut();
            let current_time = ::time::now();
            for cookie in response.cookies() {
                if let Some(expires) = cookie.expires() {
                    if expires <= current_time {
                        jar.force_remove(cookie);
                        continue;
                    }
                }

                jar.add(cookie.into_owned());
            }
        }
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
    pub fn get<'c, 'u: 'c, U: Into<Uri<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
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
    pub fn put<'c, 'u: 'c, U: Into<Uri<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
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
    pub fn post<'c, 'u: 'c, U: Into<Uri<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
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
        where U: Into<Uri<'u>>
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
        where U: Into<Uri<'u>>
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
        where U: Into<Uri<'u>>
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
        where U: Into<Uri<'u>>
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
        where U: Into<Uri<'u>>
    {
        let request = Request::new(&self.rocket, method, uri);

        if let Some(ref jar) = self.cookies {
            for cookie in jar.borrow().iter() {
                request.cookies().add_original(cookie.clone().into_owned());
            }
        }

        LocalRequest::new(&self, request)
    }
}
