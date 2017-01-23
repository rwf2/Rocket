use std::cell::RefCell;
use std::net::SocketAddr;
use std::fmt;

use term_painter::Color::*;
use term_painter::ToStyle;

use state::Container;

use error::Error;
use super::{FromParam, FromSegments};

use router::Route;
use http::uri::{URI, Segments};
use http::{Method, ContentType, Header, HeaderMap, Cookies};

use http::hyper;

/// The type of an incoming web request.
///
/// This should be used sparingly in Rocket applications. In particular, it
/// should likely only be used when writing
/// [FromRequest](/rocket/request/struct.Request.html) implementations. It
/// contains all of the information for a given web request except for the body
/// data. This includes the HTTP method, URI, cookies, headers, and more.
pub struct Request<'r> {
    method: Method,
    uri: URI<'r>,
    headers: HeaderMap<'r>,
    remote: Option<SocketAddr>,
    params: RefCell<Vec<(usize, usize)>>,
    cookies: Cookies,
    state: Option<&'r Container>,
}

impl<'r> Request<'r> {
    /// Create a new `Request` with the given `method` and `uri`. The `uri`
    /// parameter can be of any type that implements `Into<URI>` including
    /// `&str` and `String`; it must be a valid absolute URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::Method;
    ///
    /// let request = Request::new(Method::Get, "/uri");
    /// ```
    pub fn new<U: Into<URI<'r>>>(method: Method, uri: U) -> Request<'r> {
        Request {
            method: method,
            uri: uri.into(),
            headers: HeaderMap::new(),
            remote: None,
            params: RefCell::new(Vec::new()),
            cookies: Cookies::new(&[]),
            state: None
        }
    }

    /// Retrieve the method from `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::Method;
    ///
    /// let request = Request::new(Method::Get, "/uri");
    /// assert_eq!(request.method(), Method::Get);
    /// ```
    #[inline(always)]
    pub fn method(&self) -> Method {
        self.method
    }

    /// Set the method of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::Method;
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// assert_eq!(request.method(), Method::Get);
    ///
    /// request.set_method(Method::Post);
    /// assert_eq!(request.method(), Method::Post);
    /// ```
    #[inline(always)]
    pub fn set_method(&mut self, method: Method) {
        self.method = method;
    }

    /// Borrow the URI from `self`, which must be an absolute URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::Method;
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// assert_eq!(request.uri().as_str(), "/uri");
    /// ```
    #[inline(always)]
    pub fn uri(&self) -> &URI {
        &self.uri
    }

    /// Set the URI in `self`. The `uri` parameter can be of any type that
    /// implements `Into<URI>` including `&str` and `String`; it must be a valid
    /// absolute URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::Method;
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    ///
    /// request.set_uri("/hello/Sergio?type=greeting");
    /// assert_eq!(request.uri().as_str(), "/hello/Sergio?type=greeting");
    /// ```
    #[inline(always)]
    pub fn set_uri<'u: 'r, U: Into<URI<'u>>>(&mut self, uri: U) {
        self.uri = uri.into();
        self.params = RefCell::new(Vec::new());
    }

    /// Returns the address of the remote connection that initiated this
    /// request if the address is known. If the address is not known, `None` is
    /// returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::Method;
    ///
    /// let request = Request::new(Method::Get, "/uri");
    /// assert!(request.remote().is_none());
    /// ```
    #[inline(always)]
    pub fn remote(&self) -> Option<SocketAddr> {
        self.remote
    }

    /// Sets the remote address of `self` to `address`.
    ///
    /// # Example
    ///
    /// Set the remote address to be 127.0.0.1:8000:
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::Method;
    /// use std::net::{SocketAddr, IpAddr, Ipv4Addr};
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    ///
    /// let (ip, port) = (IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
    /// let localhost = SocketAddr::new(ip, port);
    /// request.set_remote(localhost);
    ///
    /// assert_eq!(request.remote(), Some(localhost));
    /// ```
    #[doc(hidden)]
    #[inline(always)]
    pub fn set_remote(&mut self, address: SocketAddr) {
        self.remote = Some(address);
    }

    /// Returns a `HeaderMap` of all of the headers in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::{Method, ContentType};
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// let header_map = request.headers();
    /// assert!(header_map.is_empty());
    /// ```
    #[inline(always)]
    pub fn headers(&self) -> &HeaderMap<'r> {
        &self.headers
    }

    /// Add the `header` to `self`'s headers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::{Method, ContentType};
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// assert!(request.headers().is_empty());
    ///
    /// request.add_header(ContentType::HTML.into());
    /// assert!(request.headers().contains("Content-Type"));
    /// assert_eq!(request.headers().len(), 1);
    /// ```
    #[inline(always)]
    pub fn add_header(&mut self, header: Header<'r>) {
        self.headers.add(header);
    }

    /// Replaces the value of the header with `header.name` with `header.value`.
    /// If no such header existed, `header` is added.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::{Method, ContentType};
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// assert!(request.headers().is_empty());
    ///
    /// request.add_header(ContentType::HTML.into());
    /// assert_eq!(request.content_type(), ContentType::HTML);
    ///
    /// request.replace_header(ContentType::JSON.into());
    /// assert_eq!(request.content_type(), ContentType::JSON);
    /// ```
    #[inline(always)]
    pub fn replace_header(&mut self, header: Header<'r>) {
        self.headers.replace(header);
    }

    /// Returns a borrow to the cookies in `self`.
    ///
    /// Note that `Cookies` implements internal mutability, so this method
    /// allows you to get _and_ set cookies in `self`.
    ///
    /// # Example
    ///
    /// Add a new cookie to a request's cookies:
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::{Cookie, Method, ContentType};
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// request.cookies().add(Cookie::new("key".into(), "val".into()))
    /// ```
    #[inline(always)]
    pub fn cookies(&self) -> &Cookies {
        &self.cookies
    }

    /// Replace all of the cookies in `self` with `cookies`.
    ///
    /// # Example
    ///
    /// Add a new cookie to a request's cookies:
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::{Cookie, Method, ContentType};
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// request.cookies().add(Cookie::new("key".into(), "val".into()))
    /// ```
    #[doc(hidden)]
    #[inline(always)]
    pub fn set_cookies(&mut self, cookies: Cookies) {
        self.cookies = cookies;
    }

    /// Returns the Content-Type header of `self`. If the header is not present,
    /// returns `ContentType::Any`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::{Method, ContentType};
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// assert_eq!(request.content_type(), ContentType::Any);
    ///
    /// request.replace_header(ContentType::JSON.into());
    /// assert_eq!(request.content_type(), ContentType::JSON);
    /// ```
    #[inline(always)]
    pub fn content_type(&self) -> ContentType {
        self.headers().get_one("Content-Type")
            .and_then(|value| value.parse().ok())
            .unwrap_or(ContentType::Any)
    }

    /// Retrieves and parses into `T` the 0-indexed `n`th dynamic parameter from
    /// the request. Returns `Error::NoKey` if `n` is greater than the number of
    /// params. Returns `Error::BadParse` if the parameter type `T` can't be
    /// parsed from the parameter.
    ///
    /// This method exists only to be used by manual routing. To retrieve
    /// parameters from a request, use Rocket's code generation facilities.
    ///
    /// # Example
    ///
    /// Retrieve parameter `0`, which is expected to be an `&str`, in a manual
    /// route:
    ///
    /// ```rust
    /// use rocket::{Request, Data};
    /// use rocket::handler::Outcome;
    ///
    /// fn name<'a>(req: &'a Request, _: Data) -> Outcome<'a> {
    ///     Outcome::of(req.get_param(0).unwrap_or("unnamed"))
    /// }
    /// ```
    pub fn get_param<'a, T: FromParam<'a>>(&'a self, n: usize) -> Result<T, Error> {
        let param = self.get_param_str(n).ok_or(Error::NoKey)?;
        T::from_param(param).map_err(|_| Error::BadParse)
    }

    /// Set `self`'s parameters given that the route used to reach this request
    /// was `route`. This should only be used internally by `Rocket` as improper
    /// use may result in out of bounds indexing.
    /// TODO: Figure out the mount path from here.
    #[doc(hidden)]
    #[inline(always)]
    pub fn set_params(&self, route: &Route) {
        *self.params.borrow_mut() = route.get_param_indexes(self.uri());
    }

    /// Get the `n`th path parameter as a string, if it exists.
    #[doc(hidden)]
    pub fn get_param_str(&self, n: usize) -> Option<&str> {
        let params = self.params.borrow();
        if n >= params.len() {
            debug!("{} is >= param count {}", n, params.len());
            return None;
        }

        let (i, j) = params[n];
        let path = self.uri.path();
        if j > path.len() {
            error!("Couldn't retrieve parameter: internal count incorrect.");
            return None;
        }

        Some(&path[i..j])
    }

    /// Retrieves and parses into `T` all of the path segments in the request
    /// URI beginning at the 0-indexed `n`th dynamic parameter. `T` must
    /// implement [FromSegments](/rocket/request/trait.FromSegments.html), which
    /// is used to parse the segments.
    ///
    /// This method exists only to be used by manual routing. To retrieve
    /// segments from a request, use Rocket's code generation facilities.
    ///
    /// # Error
    ///
    /// If there are less than `n` segments, returns an `Err` of `NoKey`. If
    /// parsing the segments failed, returns an `Err` of `BadParse`.
    ///
    /// # Example
    ///
    /// If the request URI is `"/hello/there/i/am/here"`, and the matched route
    /// path for this request is `"/hello/<name>/i/<segs..>"`, then
    /// `request.get_segments::<T>(1)` will attempt to parse the segments
    /// `"am/here"` as type `T`.
    pub fn get_segments<'a, T: FromSegments<'a>>(&'a self, n: usize)
            -> Result<T, Error> {
        let segments = self.get_raw_segments(n).ok_or(Error::NoKey)?;
        T::from_segments(segments).map_err(|_| Error::BadParse)
    }

    /// Get the segments beginning at the `n`th dynamic parameter, if they
    /// exist.
    #[doc(hidden)]
    pub fn get_raw_segments(&self, n: usize) -> Option<Segments> {
        let params = self.params.borrow();
        if n >= params.len() {
            debug!("{} is >= param (segments) count {}", n, params.len());
            return None;
        }

        let (i, j) = params[n];
        let path = self.uri.path();
        if j > path.len() {
            error!("Couldn't retrieve segments: internal count incorrect.");
            return None;
        }

        Some(Segments(&path[i..j]))
    }

    /// Get the managed state container, if it exists. For internal use only!
    #[doc(hidden)]
    pub fn get_state(&self) -> Option<&'r Container> {
        self.state
    }

    /// Set the state. For internal use only!
    #[doc(hidden)]
    pub fn set_state(&mut self, state: &'r Container) {
        self.state = Some(state);
    }

    /// Convert from Hyper types into a Rocket Request.
    #[doc(hidden)]
    pub fn from_hyp(h_method: hyper::Method,
                    h_headers: hyper::header::Headers,
                    h_uri: hyper::RequestUri,
                    h_addr: SocketAddr,
                    ) -> Result<Request<'r>, String> {
        // Get a copy of the URI for later use.
        let uri = match h_uri {
            hyper::RequestUri::AbsolutePath(s) => s,
            _ => return Err(format!("Bad URI: {}", h_uri)),
        };

        // Ensure that the method is known. TODO: Allow made-up methods?
        let method = match Method::from_hyp(&h_method) {
            Some(method) => method,
            None => return Err(format!("Invalid method: {}", h_method))
        };

        // Construct the request object.
        let mut request = Request::new(method, uri);

        // Set the request cookies, if they exist. TODO: Use session key.
        if let Some(cookies) = h_headers.get::<hyper::header::Cookie>() {
            request.set_cookies(cookies.to_cookie_jar(&[]));
        }

        // Set the rest of the headers.
        for hyp in h_headers.iter() {
            let header = Header::new(hyp.name().to_string(), hyp.value_string());
            request.add_header(header);
        }

        // Set the remote address.
        request.set_remote(h_addr);

        Ok(request)
    }
}

impl<'r> fmt::Display for Request<'r> {
    /// Pretty prints a Request. This is primarily used by Rocket's logging
    /// infrastructure.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", Green.paint(&self.method), Blue.paint(&self.uri))?;
        if self.method.supports_payload() && !self.content_type().is_any() {
            write!(f, " {}", Yellow.paint(self.content_type()))?;
        }

        Ok(())
    }
}
