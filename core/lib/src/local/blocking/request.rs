use std::fmt;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::borrow::Cow;

use crate::{Request, Response};
use crate::http::{Method, Header, Cookie};
use crate::local::blocking::Client;

// TODO: Explain difference from async LocalRequest
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
/// ## Example
///
/// The following snippet uses the available builder methods to construct a
/// `POST` request to `/` with a JSON body:
///
/// ```rust
/// use rocket::local::blocking::Client;
/// use rocket::http::{ContentType, Cookie};
///
/// let client = Client::new(rocket::ignite()).expect("valid rocket");
/// let req = client.post("/")
///     .header(ContentType::JSON)
///     .remote("127.0.0.1:8000".parse().unwrap())
///     .cookie(Cookie::new("name", "value"))
///     .body(r#"{ "value": 42 }"#);
/// ```
///
/// # Dispatching
///
/// A `LocalRequest` can be dispatched in one of two ways:
///
///   1. [`dispatch`]
///
///      This method should always be preferred. The `LocalRequest` is consumed
///      and a response is returned.
///
///   2. [`mut_dispatch`]
///
///      This method should _only_ be used when either it is known that the
///      application will not modify the request, or it is desired to see
///      modifications to the request. No cloning occurs, and the request is not
///      consumed.
///
/// Additionally, note that `LocalRequest` implements `Clone`. As such, if the
/// same request needs to be dispatched multiple times, the request can first be
/// cloned and then dispatched: `request.clone().dispatch()`.
///
/// [`Client`]: crate::local::blocking::Client
/// [`header`]: #method.header
/// [`add_header`]: #method.add_header
/// [`cookie`]: #method.cookie
/// [`remote`]: #method.remote
/// [`body`]: #method.body
/// [`set_body`]: #method.set_body
/// [`dispatch`]: #method.dispatch
/// [`mut_dispatch`]: #method.mut_dispatch
pub struct LocalRequest<'c> {
    inner: crate::local::LocalRequest<'c>,
    client: &'c Client,
}

impl<'c> LocalRequest<'c> {
    pub(crate) fn new(
        client: &'c Client,
        method: Method,
        uri: Cow<'c, str>
    ) -> LocalRequest<'c> {
        let inner = crate::local::LocalRequest::new(&client.inner, method, uri);
        Self { inner, client }
    }

    /// Retrieves the inner `Request` as seen by Rocket.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.get("/");
    /// let inner_req = req.inner();
    /// ```
    #[inline]
    pub fn inner(&self) -> &Request<'c> {
        self.inner.inner()
    }

    /// Add a header to this request.
    ///
    /// Any type that implements `Into<Header>` can be used here. Among others,
    /// this includes [`ContentType`] and [`Accept`].
    ///
    /// [`ContentType`]: crate::http::ContentType
    /// [`Accept`]: crate::http::Accept
    ///
    /// # Examples
    ///
    /// Add the Content-Type header:
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    /// use rocket::http::ContentType;
    ///
    /// # #[allow(unused_variables)]
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let req = client.get("/").header(ContentType::JSON);
    /// ```
    #[inline]
    pub fn header<H: Into<Header<'static>>>(mut self, header: H) -> Self {
        self.inner = self.inner.header(header);
        self
    }

    /// Adds a header to this request without consuming `self`.
    ///
    /// # Examples
    ///
    /// Add the Content-Type header:
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let mut req = client.get("/");
    /// req.add_header(ContentType::JSON);
    /// ```
    #[inline]
    pub fn add_header<H: Into<Header<'static>>>(&mut self, header: H) {
        self.inner.add_header(header)
    }

    /// Set the remote address of this request.
    ///
    /// # Examples
    ///
    /// Set the remote address to "8.8.8.8:80":
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let address = "8.8.8.8:80".parse().unwrap();
    /// let req = client.get("/").remote(address);
    /// ```
    #[inline]
    pub fn remote(mut self, address: SocketAddr) -> Self {
        self.inner = self.inner.remote(address);
        self
    }

    /// Add a cookie to this request.
    ///
    /// # Examples
    ///
    /// Add `user_id` cookie:
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    /// use rocket::http::Cookie;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// # #[allow(unused_variables)]
    /// let req = client.get("/")
    ///     .cookie(Cookie::new("username", "sb"))
    ///     .cookie(Cookie::new("user_id", "12"));
    /// ```
    #[inline]
    pub fn cookie(mut self, cookie: Cookie<'_>) -> Self {
        self.inner = self.inner.cookie(cookie);
        self
    }

    /// Add all of the cookies in `cookies` to this request.
    ///
    /// # Examples
    ///
    /// Add `user_id` cookie:
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    /// use rocket::http::Cookie;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let cookies = vec![Cookie::new("a", "b"), Cookie::new("c", "d")];
    /// # #[allow(unused_variables)]
    /// let req = client.get("/").cookies(cookies);
    /// ```
    #[inline]
    pub fn cookies(mut self, cookies: Vec<Cookie<'_>>) -> Self {
        self.inner = self.inner.cookies(cookies);
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
    /// use rocket::local::blocking::Client;
    /// use rocket::http::Cookie;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// # #[allow(unused_variables)]
    /// let req = client.get("/").private_cookie(Cookie::new("user_id", "sb"));
    /// ```
    #[inline]
    #[cfg(feature = "private-cookies")]
    pub fn private_cookie(mut self, cookie: Cookie<'static>) -> Self {
        self.inner = self.inner.private_cookie(cookie);
        self
    }

    // TODO: For CGI, we want to be able to set the body to be stdin without
    // actually reading everything into a vector. Can we allow that here while
    // keeping the simplicity? Looks like it would require us to reintroduce a
    // NetStream::Local(Box<Read>) or something like that.

    /// Set the body (data) of the request.
    ///
    /// # Examples
    ///
    /// Set the body to be a JSON structure; also sets the Content-Type.
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// # #[allow(unused_variables)]
    /// let req = client.post("/")
    ///     .header(ContentType::JSON)
    ///     .body(r#"{ "key": "value", "array": [1, 2, 3], }"#);
    /// ```
    #[inline]
    pub fn body<S: AsRef<[u8]>>(mut self, body: S) -> Self {
        self.inner = self.inner.body(body);
        self
    }

    /// Set the body (data) of the request without consuming `self`.
    ///
    /// # Examples
    ///
    /// Set the body to be a JSON structure; also sets the Content-Type.
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let mut req = client.post("/").header(ContentType::JSON);
    /// req.set_body(r#"{ "key": "value", "array": [1, 2, 3], }"#);
    /// ```
    #[inline]
    pub fn set_body<S: AsRef<[u8]>>(&mut self, body: S) {
        self.inner.set_body(body);
    }

    /// Dispatches the request, returning the response.
    ///
    /// This method consumes `self` and is the preferred mechanism for
    /// dispatching.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let response = client.get("/").dispatch();
    /// ```
    #[inline(always)]
    pub fn dispatch(self) -> LocalResponse<'c> {
        let inner = self.client.block_on(self.inner.dispatch());
        LocalResponse { inner, client: self.client }
    }

    /// Dispatches the request, returning the response.
    ///
    /// This method _does not_ consume or clone `self`. Any changes to the
    /// request that occur during handling will be visible after this method is
    /// called. For instance, body data is always consumed after a request is
    /// dispatched. As such, only the first call to `mut_dispatch` for a given
    /// `LocalRequest` will contains the original body data.
    ///
    /// This method should _only_ be used when either it is known that
    /// the application will not modify the request, or it is desired to see
    /// modifications to the request. Prefer to use [`dispatch`] instead.
    ///
    /// [`dispatch`]: #method.dispatch
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::blocking::Client;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    ///
    /// let mut req = client.get("/");
    /// let response_a = req.mut_dispatch();
    /// // TODO.async: Annoying. Is this really a good example to show?
    /// drop(response_a);
    /// let response_b = req.mut_dispatch();
    /// ```
    #[inline(always)]
    pub fn mut_dispatch(&mut self) -> LocalResponse<'c> {
        let inner = self.client.block_on(self.inner.mut_dispatch());
        LocalResponse { inner, client: self.client }
    }

}

impl fmt::Debug for LocalRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.inner.inner(), f)
    }
}

/// A structure representing a response from dispatching a local request.
///
/// This structure is a thin wrapper around [`Response`]. It implements no
/// methods of its own; all functionality is exposed via the [`Deref`] and
/// [`DerefMut`] implementations with a target of `Response`. In other words,
/// when invoking methods, a `LocalResponse` can be treated exactly as if it
/// were a `Response`.
pub struct LocalResponse<'c> {
    inner: crate::local::LocalResponse<'c>,
    client: &'c Client,
}

impl<'c> Deref for LocalResponse<'c> {
    type Target = Response<'c>;

    #[inline(always)]
    fn deref(&self) -> &Response<'c> {
        &*self.inner
    }
}

impl<'c> DerefMut for LocalResponse<'c> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Response<'c> {
        &mut *self.inner
    }
}

impl LocalResponse<'_> {
    pub fn body_string(&mut self) -> Option<String> {
        self.client.block_on(self.inner.body_string())
    }

    pub fn body_bytes(&mut self) -> Option<Vec<u8>> {
        self.client.block_on(self.inner.body_bytes())
    }
}

impl fmt::Debug for LocalResponse<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.inner, f)
    }
}
