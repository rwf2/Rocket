use std::cell::RefCell;
use std::net::SocketAddr;
use std::fmt;
use std::str;

use term_painter::Color::*;
use term_painter::ToStyle;

use state::Container;

use error::Error;
use super::{FromParam, FromSegments};

use router::Route;
use http::uri::{URI, Segments};
use http::{Method, Header, HeaderMap, Cookies, Session, CookieJar, Key};
use http::{RawStr, ContentType, Accept, MediaType};
use http::parse::media_type;
use http::hyper;

struct PresetState<'r> {
    key: &'r Key,
    managed_state: &'r Container,
}

struct RequestState<'r> {
    preset: Option<PresetState<'r>>,
    params: RefCell<Vec<(usize, usize)>>,
    cookies: RefCell<CookieJar>,
    session: RefCell<CookieJar>,
}

/// The type of an incoming web request.
///
/// This should be used sparingly in Rocket applications. In particular, it
/// should likely only be used when writing
/// [FromRequest](/rocket/request/trait.FromRequest.html) implementations. It
/// contains all of the information for a given web request except for the body
/// data. This includes the HTTP method, URI, cookies, headers, and more.
pub struct Request<'r> {
    method: Method,
    uri: URI<'r>,
    headers: HeaderMap<'r>,
    remote: Option<SocketAddr>,
    extra: RequestState<'r>
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
    /// # #[allow(unused_variables)]
    /// let request = Request::new(Method::Get, "/uri");
    /// ```
    #[inline(always)]
    pub fn new<U: Into<URI<'r>>>(method: Method, uri: U) -> Request<'r> {
        Request {
            method: method,
            uri: uri.into(),
            headers: HeaderMap::new(),
            remote: None,
            extra: RequestState {
                preset: None,
                params: RefCell::new(Vec::new()),
                cookies: RefCell::new(CookieJar::new()),
                session: RefCell::new(CookieJar::new()),
            }
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
    /// let request = Request::new(Method::Get, "/uri");
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
        *self.extra.params.borrow_mut() = Vec::new();
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
    /// use rocket::http::Method;
    ///
    /// let request = Request::new(Method::Get, "/uri");
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
    /// request.add_header(ContentType::HTML);
    /// assert!(request.headers().contains("Content-Type"));
    /// assert_eq!(request.headers().len(), 1);
    /// ```
    #[inline(always)]
    pub fn add_header<H: Into<Header<'r>>>(&mut self, header: H) {
        self.headers.add(header.into());
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
    /// request.add_header(ContentType::HTML);
    /// assert_eq!(request.content_type(), Some(ContentType::HTML));
    ///
    /// request.replace_header(ContentType::JSON);
    /// assert_eq!(request.content_type(), Some(ContentType::JSON));
    /// ```
    #[inline(always)]
    pub fn replace_header<H: Into<Header<'r>>>(&mut self, header: H) {
        self.headers.replace(header.into());
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
    /// use rocket::http::{Cookie, Method};
    ///
    /// let request = Request::new(Method::Get, "/uri");
    /// request.cookies().add(Cookie::new("key", "val"));
    /// request.cookies().add(Cookie::new("ans", format!("life: {}", 38 + 4)));
    /// ```
    #[inline]
    pub fn cookies(&self) -> Cookies {
        match self.extra.cookies.try_borrow_mut() {
            Ok(jar) => Cookies::new(jar),
            Err(_) => {
                error_!("Multiple `Cookies` instances are active at once.");
                info_!("An instance of `Cookies` must be dropped before another \
                       can be retrieved.");
                warn_!("The retrieved `Cookies` instance will be empty.");
                Cookies::empty()
            }
        }
    }

    #[inline]
    pub fn session(&self) -> Session {
        match self.extra.session.try_borrow_mut() {
            Ok(jar) => Session::new(jar, self.preset().key),
            Err(_) => {
                error_!("Multiple `Session` instances are active at once.");
                info_!("An instance of `Session` must be dropped before another \
                       can be retrieved.");
                warn_!("The retrieved `Session` instance will be empty.");
                Session::empty(self.preset().key)
            }
        }
    }

    /// Replace all of the cookies in `self` with those in `jar`.
    #[inline]
    pub(crate) fn set_cookies(&mut self, jar: CookieJar) {
        self.extra.cookies = RefCell::new(jar);
    }

    /// Replace all of the session cookie in `self` with those in `jar`.
    #[inline]
    pub(crate) fn set_session(&mut self, jar: CookieJar) {
        self.extra.session = RefCell::new(jar);
    }

    /// Returns `Some` of the Content-Type header of `self`. If the header is
    /// not present, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Request;
    /// use rocket::http::{Method, ContentType};
    ///
    /// let mut request = Request::new(Method::Get, "/uri");
    /// assert_eq!(request.content_type(), None);
    ///
    /// request.replace_header(ContentType::JSON);
    /// assert_eq!(request.content_type(), Some(ContentType::JSON));
    /// ```
    #[inline(always)]
    pub fn content_type(&self) -> Option<ContentType> {
        // FIXME: Don't reparse each time! Use RC? Smarter than that?
        // Use state::Storage!
        self.headers().get_one("Content-Type").and_then(|value| value.parse().ok())
    }

    #[inline(always)]
    pub fn accept(&self) -> Option<Accept> {
        self.headers().get_one("Accept").and_then(|v| v.parse().ok())
    }

    #[inline(always)]
    pub fn accept_first(&self) -> Option<MediaType> {
        self.headers().get_one("Accept").and_then(|mut v| media_type(&mut v).ok())
    }

    #[inline(always)]
    pub fn format(&self) -> Option<MediaType> {
        if self.method.supports_payload() {
            self.content_type().map(|ct| ct.into_media_type())
        } else {
            // FIXME: Should we be using `accept_first` or `preferred`? Or
            // should we be checking neither and instead pass things through
            // where the client accepts the thing at all?
            self.accept()
                .map(|accept| accept.preferred().media_type().clone())
                .or(Some(MediaType::Any))
        }
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
    /// Retrieve parameter `0`, which is expected to be a `String`, in a manual
    /// route:
    ///
    /// ```rust
    /// use rocket::{Request, Data};
    /// use rocket::handler::Outcome;
    ///
    /// # #[allow(dead_code)]
    /// fn name<'a>(req: &'a Request, _: Data) -> Outcome<'a> {
    ///     Outcome::of(req.get_param::<String>(0).unwrap_or("unnamed".into()))
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
    #[inline]
    pub(crate) fn set_params(&self, route: &Route) {
        *self.extra.params.borrow_mut() = route.get_param_indexes(self.uri());
    }

    /// Get the `n`th path parameter as a string, if it exists. This is used by
    /// codegen.
    #[doc(hidden)]
    pub fn get_param_str(&self, n: usize) -> Option<&RawStr> {
        let params = self.extra.params.borrow();
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

        Some(path[i..j].into())
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
    /// exist. Used by codegen.
    #[doc(hidden)]
    pub fn get_raw_segments(&self, n: usize) -> Option<Segments> {
        let params = self.extra.params.borrow();
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
    #[inline(always)]
    pub fn get_state<T: Send + Sync + 'static>(&self) -> Option<&'r T> {
        self.preset().managed_state.try_get()
    }

    #[inline(always)]
    fn preset(&self) -> &PresetState<'r> {
        match self.extra.preset {
            Some(ref state) => state,
            None => {
                error_!("Internal Rocket error: preset state is unset!");
                panic!("Please report this error to the GitHub issue tracker.");
            }
        }
    }

    /// Set the precomputed state. For internal use only!
    #[inline(always)]
    pub(crate) fn set_preset_state(&mut self, key: &'r Key, state: &'r Container) {
        self.extra.preset = Some(PresetState { key, managed_state: state });
    }

    /// Convert from Hyper types into a Rocket Request.
    pub(crate) fn from_hyp(h_method: hyper::Method,
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
        request.set_remote(h_addr);

        // Set the request cookies, if they exist.
        if let Some(cookie_headers) = h_headers.get_raw("Cookie") {
            let mut cookie_jar = CookieJar::new();
            let mut session_jar = CookieJar::new();
            for header in cookie_headers {
                let raw_str = match ::std::str::from_utf8(header) {
                    Ok(string) => string,
                    Err(_) => continue
                };

                for cookie_str in raw_str.split(";").map(|s| s.trim()) {
                    if let Some(cookie) = Session::parse_cookie(cookie_str) {
                        session_jar.add_original(cookie);
                    } else if let Some(cookie) = Cookies::parse_cookie(cookie_str) {
                        cookie_jar.add_original(cookie);
                    }
                }
            }

            request.set_cookies(cookie_jar);
            request.set_session(session_jar);
        }

        // Set the rest of the headers.
        for hyp in h_headers.iter() {
            if let Some(header_values) = h_headers.get_raw(hyp.name()) {
                for value in header_values {
                    // This is not totally correct since values needn't be UTF8.
                    let value_str = String::from_utf8_lossy(value).into_owned();
                    let header = Header::new(hyp.name().to_string(), value_str);
                    request.add_header(header);
                }
            }
        }

        Ok(request)
    }
}

impl<'r> fmt::Display for Request<'r> {
    /// Pretty prints a Request. This is primarily used by Rocket's logging
    /// infrastructure.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", Green.paint(&self.method), Blue.paint(&self.uri))?;
        if let Some(content_type) = self.content_type() {
            if self.method.supports_payload() {
                write!(f, " {}", Yellow.paint(content_type))?;
            }
        }

        Ok(())
    }
}
