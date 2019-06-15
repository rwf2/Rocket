use std::fmt;
use std::rc::Rc;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::borrow::Cow;

use crate::{Request, Response, Data};
use crate::http::{Status, Method, Header, Cookie, uri::Origin, ext::IntoOwned};
use crate::local::Client;

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
/// use rocket::local::Client;
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
/// [`Client`]: crate::local::Client
/// [`header`]: #method.header
/// [`add_header`]: #method.add_header
/// [`cookie`]: #method.cookie
/// [`remote`]: #method.remote
/// [`body`]: #method.body
/// [`set_body`]: #method.set_body
/// [`dispatch`]: #method.dispatch
/// [`mut_dispatch`]: #method.mut_dispatch
pub struct LocalRequest<'c> {
    client: &'c Client,
    // This pointer exists to access the `Rc<Request>` mutably inside of
    // `LocalRequest`. This is the only place that a `Request` can be accessed
    // mutably. This is accomplished via the private `request_mut()` method.
    ptr: *mut Request<'c>,
    // This `Rc` exists so that we can transfer ownership to the `LocalResponse`
    // selectively on dispatch. This is necessary because responses may point
    // into the request, and thus the request and all of its data needs to be
    // alive while the response is accessible.
    //
    // Because both a `LocalRequest` and a `LocalResponse` can hold an `Rc` to
    // the same `Request`, _and_ the `LocalRequest` can mutate the request, we
    // must ensure that 1) neither `LocalRequest` not `LocalResponse` are `Sync`
    // or `Send` and 2) mutations carried out in `LocalRequest` are _stable_:
    // they never _remove_ data, and any reallocations (say, for vectors or
    // hashmaps) result in object pointers remaining the same. This means that
    // even if the `Request` is mutated by a `LocalRequest`, those mutations are
    // not observable by `LocalResponse`.
    //
    // The first is ensured by the embedding of the `Rc` type which is neither
    // `Send` nor `Sync`. The second is more difficult to argue. First, observe
    // that any methods of `LocalRequest` that _remove_ values from `Request`
    // only remove _Copy_ values, in particular, `SocketAddr`. Second, the
    // lifetime of the `Request` object is tied to the lifetime of the
    // `LocalResponse`, so references from `Request` cannot be dangling in
    // `Response`. And finally, observe how all of the data stored in `Request`
    // is converted into its owned counterpart before insertion, ensuring stable
    // addresses. Together, these properties guarantee the second condition.
    request: Rc<Request<'c>>,
    data: Vec<u8>,
    uri: Cow<'c, str>,
}

impl<'c> LocalRequest<'c> {
    #[inline(always)]
    crate fn new(
        client: &'c Client,
        method: Method,
        uri: Cow<'c, str>
    ) -> LocalRequest<'c> {
        // We set a dummy string for now and check the user's URI on dispatch.
        let request = Request::new(client.rocket(), method, Origin::dummy());

        // Set up any cookies we know about.
        if let Some(ref jar) = client.cookies {
            let cookies = jar.read().expect("LocalRequest::new() read lock");
            for cookie in cookies.iter() {
                request.cookies().add_original(cookie.clone().into_owned());
            }
        }

        // See the comments on the structure for what's going on here.
        let mut request = Rc::new(request);
        let ptr = Rc::get_mut(&mut request).unwrap() as *mut Request<'_>;
        LocalRequest { client, ptr, request, uri, data: vec![] }
    }

    /// Retrieves the inner `Request` as seen by Rocket.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).expect("valid rocket");
    /// let req = client.get("/");
    /// let inner_req = req.inner();
    /// ```
    #[inline]
    pub fn inner(&self) -> &Request<'c> {
        &*self.request
    }

    #[inline(always)]
    fn request_mut(&mut self) -> &mut Request<'c> {
        // See the comments in the structure for the argument of correctness.
        unsafe { &mut *self.ptr }
    }

    // This method should _never_ be publicly exposed!
    #[inline(always)]
    fn long_lived_request<'a>(&mut self) -> &'a mut Request<'c> {
        // See the comments in the structure for the argument of correctness.
        // Additionally, the caller must ensure that the owned instance of
        // `Rc<Request>` remains valid as long as the returned reference can be
        // accessed.
        unsafe { &mut *self.ptr }
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
    /// use rocket::local::Client;
    /// use rocket::http::ContentType;
    ///
    /// # #[allow(unused_variables)]
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let req = client.get("/").header(ContentType::JSON);
    /// ```
    #[inline]
    pub fn header<H: Into<Header<'static>>>(mut self, header: H) -> Self {
        self.request_mut().add_header(header.into());
        self
    }

    /// Adds a header to this request without consuming `self`.
    ///
    /// # Examples
    ///
    /// Add the Content-Type header:
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let mut req = client.get("/");
    /// req.add_header(ContentType::JSON);
    /// ```
    #[inline]
    pub fn add_header<H: Into<Header<'static>>>(&mut self, header: H) {
        self.request_mut().add_header(header.into());
    }

    /// Set the remote address of this request.
    ///
    /// # Examples
    ///
    /// Set the remote address to "8.8.8.8:80":
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let address = "8.8.8.8:80".parse().unwrap();
    /// let req = client.get("/").remote(address);
    /// ```
    #[inline]
    pub fn remote(mut self, address: SocketAddr) -> Self {
        self.request_mut().set_remote(address);
        self
    }

    /// Add a cookie to this request.
    ///
    /// # Examples
    ///
    /// Add `user_id` cookie:
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::Cookie;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// # #[allow(unused_variables)]
    /// let req = client.get("/")
    ///     .cookie(Cookie::new("username", "sb"))
    ///     .cookie(Cookie::new("user_id", "12"));
    /// ```
    #[inline]
    pub fn cookie(self, cookie: Cookie<'_>) -> Self {
        self.request.cookies().add_original(cookie.into_owned());
        self
    }

    /// Add all of the cookies in `cookies` to this request.
    ///
    /// # Examples
    ///
    /// Add `user_id` cookie:
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::Cookie;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let cookies = vec![Cookie::new("a", "b"), Cookie::new("c", "d")];
    /// # #[allow(unused_variables)]
    /// let req = client.get("/").cookies(cookies);
    /// ```
    #[inline]
    pub fn cookies(self, cookies: Vec<Cookie<'_>>) -> Self {
        for cookie in cookies {
            self.request.cookies().add_original(cookie.into_owned());
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
    /// use rocket::local::Client;
    /// use rocket::http::Cookie;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// # #[allow(unused_variables)]
    /// let req = client.get("/").private_cookie(Cookie::new("user_id", "sb"));
    /// ```
    #[inline]
    #[cfg(feature = "private-cookies")]
    pub fn private_cookie(self, cookie: Cookie<'static>) -> Self {
        self.request.cookies().add_original_private(cookie);
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
    /// use rocket::local::Client;
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
        self.data = body.as_ref().into();
        self
    }

    /// Set the body (data) of the request without consuming `self`.
    ///
    /// # Examples
    ///
    /// Set the body to be a JSON structure; also sets the Content-Type.
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let mut req = client.post("/").header(ContentType::JSON);
    /// req.set_body(r#"{ "key": "value", "array": [1, 2, 3], }"#);
    /// ```
    #[inline]
    pub fn set_body<S: AsRef<[u8]>>(&mut self, body: S) {
        self.data = body.as_ref().into();
    }

    /// Dispatches the request, returning the response.
    ///
    /// This method consumes `self` and is the preferred mechanism for
    /// dispatching.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let response = client.get("/").dispatch();
    /// ```
    #[inline(always)]
    pub fn dispatch(mut self) -> LocalResponse<'c> {
        let r = self.long_lived_request();
        LocalRequest::_dispatch(self.client, r, self.request, &self.uri, self.data)
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
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    ///
    /// let mut req = client.get("/");
    /// let response_a = req.mut_dispatch();
    /// let response_b = req.mut_dispatch();
    /// ```
    #[inline(always)]
    pub fn mut_dispatch(&mut self) -> LocalResponse<'c> {
        let req = self.long_lived_request();
        let data = std::mem::replace(&mut self.data, vec![]);
        let rc_req = self.request.clone();
        LocalRequest::_dispatch(self.client, req, rc_req, &self.uri, data)
    }

    // Performs the actual dispatch.
    fn _dispatch(
        client: &'c Client,
        request: &'c mut Request<'c>,
        owned_request: Rc<Request<'c>>,
        uri: &str,
        data: Vec<u8>
    ) -> LocalResponse<'c> {
        // First, validate the URI, returning an error response (generated from
        // an error catcher) immediately if it's invalid.
        if let Ok(uri) = Origin::parse(uri) {
            request.set_uri(uri.into_owned());
        } else {
            error!("Malformed request URI: {}", uri);
            let res = client.rocket().handle_error(Status::BadRequest, request);
            return LocalResponse { _request: owned_request, response: res };
        }

        // Actually dispatch the request.
        let response = client.rocket().dispatch(request, Data::local(data));

        // If the client is tracking cookies, updates the internal cookie jar
        // with the changes reflected by `response`.
        if let Some(ref jar) = client.cookies {
            let mut jar = jar.write().expect("LocalRequest::_dispatch() write lock");
            let current_time = time::now();
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

        LocalResponse {
            _request: owned_request,
            response: response
        }
    }
}

impl fmt::Debug for LocalRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.request, f)
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
    _request: Rc<Request<'c>>,
    response: Response<'c>,
}

impl<'c> Deref for LocalResponse<'c> {
    type Target = Response<'c>;

    #[inline(always)]
    fn deref(&self) -> &Response<'c> {
        &self.response
    }
}

impl<'c> DerefMut for LocalResponse<'c> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Response<'c> {
        &mut self.response
    }
}

impl fmt::Debug for LocalResponse<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.response, f)
    }
}

impl<'c> Clone for LocalRequest<'c> {
    fn clone(&self) -> LocalRequest<'c> {
        LocalRequest {
            client: self.client,
            ptr: self.ptr,
            request: self.request.clone(),
            data: self.data.clone(),
            uri: self.uri.clone()
        }
    }
}

// #[cfg(test)]
mod tests {
    // Someday...

    // #[test]
    // #[compile_fail]
    // fn local_req_not_sync() {
    //     fn is_sync<T: Sync>() {  }
    //     is_sync::<::local::LocalRequest>();
    // }

    // #[test]
    // #[compile_fail]
    // fn local_req_not_send() {
    //     fn is_send<T: Send>() {  }
    //     is_send::<::local::LocalRequest>();
    // }

    // #[test]
    // #[compile_fail]
    // fn local_req_not_sync() {
    //     fn is_sync<T: Sync>() {  }
    //     is_sync::<::local::LocalResponse>();
    // }

    // #[test]
    // #[compile_fail]
    // fn local_req_not_send() {
    //     fn is_send<T: Send>() {  }
    //     is_send::<::local::LocalResponse>();
    // }

    // This checks that a response can't outlive the `Client`.
    // #[compile_fail]
    // fn test() {
    //     use {Rocket, local::Client};

    //     let rocket = Rocket::ignite();
    //     let res = {
    //         let mut client = Client::new(rocket).unwrap();
    //         client.get("/").dispatch()
    //     };

    //     // let client = Client::new(rocket).unwrap();
    //     // let res1 = client.get("/").dispatch();
    //     // let res2 = client.get("/").dispatch();
    // }

    // This checks that a response can't outlive the `Client`.
    // #[compile_fail]
    // fn test() {
    //     use {Rocket, local::Client};

    //     let rocket = Rocket::ignite();
    //     let res = {
    //         Client::new(rocket).unwrap()
    //             .get("/").dispatch();
    //     };

    //     // let client = Client::new(rocket).unwrap();
    //     // let res1 = client.get("/").dispatch();
    //     // let res2 = client.get("/").dispatch();
    // }

    // This checks that a response can't outlive the `Client`, in this case, by
    // moving `client` while it is borrowed.
    // #[compile_fail]
    // fn test() {
    //     use {Rocket, local::Client};

    //     let rocket = Rocket::ignite();
    //     let client = Client::new(rocket).unwrap();

    //     let res = {
    //         let x = client.get("/").dispatch();
    //         let y = client.get("/").dispatch();
    //         (x, y)
    //     };

    //     let x = client;
    // }

    // #[compile_fail]
    // fn test() {
    //     use {Rocket, local::Client};

    //     let rocket1 = Rocket::ignite();
    //     let rocket2 = Rocket::ignite();

    //     let client1 = Client::new(rocket1).unwrap();
    //     let client2 = Client::new(rocket2).unwrap();

    //     let res = {
    //         let mut res1 = client1.get("/");
    //         res1.client = &client2;
    //         res1
    //     };

    //     drop(client1);
    // }
}
