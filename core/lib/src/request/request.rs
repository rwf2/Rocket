use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::net::{IpAddr, SocketAddr};
use std::fmt;
use std::str;

use yansi::Paint;
use state::{Container, Storage};

use crate::request::{FromParam, FromSegments, FromRequest, Outcome};
use crate::request::{FromFormValue, FormItems, FormItem};

use crate::rocket::Rocket;
use crate::router::Route;
use crate::config::{Config, Limits};
use crate::http::{hyper, uri::{Origin, Segments}};
use crate::http::{Method, Header, HeaderMap, Cookies};
use crate::http::{RawStr, ContentType, Accept, MediaType};
use crate::http::private::{Indexed, SmallVec, CookieJar};

type Indices = (usize, usize);

/// The type of an incoming web request.
///
/// This should be used sparingly in Rocket applications. In particular, it
/// should likely only be used when writing [`FromRequest`] implementations. It
/// contains all of the information for a given web request except for the body
/// data. This includes the HTTP method, URI, cookies, headers, and more.
#[derive(Clone)]
pub struct Request<'r> {
    method: Cell<Method>,
    uri: Origin<'r>,
    headers: HeaderMap<'r>,
    remote: Option<SocketAddr>,
    crate state: RequestState<'r>,
}

#[derive(Clone)]
crate struct RequestState<'r> {
    crate config: &'r Config,
    crate managed: &'r Container,
    crate path_segments: SmallVec<[Indices; 12]>,
    crate query_items: Option<SmallVec<[IndexedFormItem; 6]>>,
    crate route: Cell<Option<&'r Route>>,
    crate cookies: RefCell<CookieJar>,
    crate accept: Storage<Option<Accept>>,
    crate content_type: Storage<Option<ContentType>>,
    crate cache: Rc<Container>,
}

#[derive(Clone)]
crate struct IndexedFormItem {
    raw: Indices,
    key: Indices,
    value: Indices
}

impl<'r> Request<'r> {
    /// Create a new `Request` with the given `method` and `uri`.
    #[inline(always)]
    crate fn new<'s: 'r>(
        rocket: &'r Rocket,
        method: Method,
        uri: Origin<'s>
    ) -> Request<'r> {
        let mut request = Request {
            method: Cell::new(method),
            uri: uri,
            headers: HeaderMap::new(),
            remote: None,
            state: RequestState {
                path_segments: SmallVec::new(),
                query_items: None,
                config: &rocket.config,
                managed: &rocket.state,
                route: Cell::new(None),
                cookies: RefCell::new(CookieJar::new()),
                accept: Storage::new(),
                content_type: Storage::new(),
                cache: Rc::new(Container::new()),
            }
        };

        request.update_cached_uri_info();
        request
    }

    /// Retrieve the method from `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// use rocket::http::Method;
    ///
    /// # Request::example(Method::Get, "/uri", |request| {
    /// request.set_method(Method::Get);
    /// assert_eq!(request.method(), Method::Get);
    /// # });
    /// ```
    #[inline(always)]
    pub fn method(&self) -> Method {
        self.method.get()
    }

    /// Set the method of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// use rocket::http::Method;
    ///
    /// # Request::example(Method::Get, "/uri", |request| {
    /// assert_eq!(request.method(), Method::Get);
    ///
    /// request.set_method(Method::Post);
    /// assert_eq!(request.method(), Method::Post);
    /// # });
    /// ```
    #[inline(always)]
    pub fn set_method(&mut self, method: Method) {
        self._set_method(method);
    }

    /// Borrow the [`Origin`] URI from `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// # Request::example(Method::Get, "/uri", |request| {
    /// assert_eq!(request.uri().path(), "/uri");
    /// # });
    /// ```
    #[inline(always)]
    pub fn uri(&self) -> &Origin<'_> {
        &self.uri
    }

    /// Set the URI in `self` to `uri`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::uri::Origin;
    ///
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// let uri = Origin::parse("/hello/Sergio?type=greeting").unwrap();
    /// request.set_uri(uri);
    /// assert_eq!(request.uri().path(), "/hello/Sergio");
    /// assert_eq!(request.uri().query(), Some("type=greeting"));
    /// # });
    /// ```
    pub fn set_uri<'u: 'r>(&mut self, uri: Origin<'u>) {
        self.uri = uri;
        self.update_cached_uri_info();
    }

    /// Returns the address of the remote connection that initiated this
    /// request if the address is known. If the address is not known, `None` is
    /// returned.
    ///
    /// Because it is common for proxies to forward connections for clients, the
    /// remote address may contain information about the proxy instead of the
    /// client. For this reason, proxies typically set the "X-Real-IP" header
    /// with the client's true IP. To extract this IP from the request, use the
    /// [`real_ip()`] or [`client_ip()`] methods.
    ///
    /// [`real_ip()`]: #method.real_ip
    /// [`client_ip()`]: #method.client_ip
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// # Request::example(Method::Get, "/uri", |request| {
    /// assert!(request.remote().is_none());
    /// # });
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
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// use std::net::{SocketAddr, IpAddr, Ipv4Addr};
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// let (ip, port) = (IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
    /// let localhost = SocketAddr::new(ip, port);
    /// request.set_remote(localhost);
    ///
    /// assert_eq!(request.remote(), Some(localhost));
    /// # });
    /// ```
    #[inline(always)]
    pub fn set_remote(&mut self, address: SocketAddr) {
        self.remote = Some(address);
    }

    /// Returns the IP address in the "X-Real-IP" header of the request if such
    /// a header exists and contains a valid IP address.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::{Header, Method};
    /// # use std::net::{SocketAddr, IpAddr, Ipv4Addr};
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// request.add_header(Header::new("X-Real-IP", "8.8.8.8"));
    /// assert_eq!(request.real_ip(), Some("8.8.8.8".parse().unwrap()));
    /// # });
    /// ```
    pub fn real_ip(&self) -> Option<IpAddr> {
        self.headers()
            .get_one("X-Real-IP")
            .and_then(|ip| {
                ip.parse()
                    .map_err(|_| warn_!("'X-Real-IP' header is malformed: {}", ip))
                    .ok()
            })
    }

    /// Attempts to return the client's IP address by first inspecting the
    /// "X-Real-IP" header and then using the remote connection's IP address.
    ///
    /// If the "X-Real-IP" header exists and contains a valid IP address, that
    /// address is returned. Otherwise, if the address of the remote connection
    /// is known, that address is returned. Otherwise, `None` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::{Header, Method};
    /// # use std::net::{SocketAddr, IpAddr, Ipv4Addr};
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// // starting without an "X-Real-IP" header or remote addresss
    /// assert!(request.client_ip().is_none());
    ///
    /// // add a remote address; this is done by Rocket automatically
    /// request.set_remote("127.0.0.1:8000".parse().unwrap());
    /// assert_eq!(request.client_ip(), Some("127.0.0.1".parse().unwrap()));
    ///
    /// // now with an X-Real-IP header
    /// request.add_header(Header::new("X-Real-IP", "8.8.8.8"));
    /// assert_eq!(request.client_ip(), Some("8.8.8.8".parse().unwrap()));
    /// # });
    /// ```
    #[inline]
    pub fn client_ip(&self) -> Option<IpAddr> {
        self.real_ip().or_else(|| self.remote().map(|r| r.ip()))
    }

    /// Returns a wrapped borrow to the cookies in `self`.
    ///
    /// [`Cookies`] implements internal mutability, so this method allows you to
    /// get _and_ add/remove cookies in `self`.
    ///
    /// # Example
    ///
    /// Add a new cookie to a request's cookies:
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// use rocket::http::Cookie;
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// request.cookies().add(Cookie::new("key", "val"));
    /// request.cookies().add(Cookie::new("ans", format!("life: {}", 38 + 4)));
    /// # });
    /// ```
    pub fn cookies(&self) -> Cookies<'_> {
        // FIXME: Can we do better? This is disappointing.
        match self.state.cookies.try_borrow_mut() {
            Ok(jar) => Cookies::new(jar, self.state.config.secret_key()),
            Err(_) => {
                error_!("Multiple `Cookies` instances are active at once.");
                info_!("An instance of `Cookies` must be dropped before another \
                       can be retrieved.");
                warn_!("The retrieved `Cookies` instance will be empty.");
                Cookies::empty()
            }
        }
    }

    /// Returns a [`HeaderMap`] of all of the headers in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// # Request::example(Method::Get, "/uri", |request| {
    /// let header_map = request.headers();
    /// assert!(header_map.is_empty());
    /// # });
    /// ```
    #[inline(always)]
    pub fn headers(&self) -> &HeaderMap<'r> {
        &self.headers
    }

    /// Add `header` to `self`'s headers. The type of `header` can be any type
    /// that implements the `Into<Header>` trait. This includes common types
    /// such as [`ContentType`] and [`Accept`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// use rocket::http::ContentType;
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// assert!(request.headers().is_empty());
    ///
    /// request.add_header(ContentType::HTML);
    /// assert!(request.headers().contains("Content-Type"));
    /// assert_eq!(request.headers().len(), 1);
    /// # });
    /// ```
    #[inline(always)]
    pub fn add_header<'h: 'r, H: Into<Header<'h>>>(&mut self, header: H) {
        self.headers.add(header.into());
    }

    /// Replaces the value of the header with name `header.name` with
    /// `header.value`. If no such header exists, `header` is added as a header
    /// to `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// use rocket::http::ContentType;
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// assert!(request.headers().is_empty());
    ///
    /// request.add_header(ContentType::Any);
    /// assert_eq!(request.headers().get_one("Content-Type"), Some("*/*"));
    ///
    /// request.replace_header(ContentType::PNG);
    /// assert_eq!(request.headers().get_one("Content-Type"), Some("image/png"));
    /// # });
    /// ```
    #[inline(always)]
    pub fn replace_header<'h: 'r, H: Into<Header<'h>>>(&mut self, header: H) {
        self.headers.replace(header.into());
    }

    /// Returns the Content-Type header of `self`. If the header is not present,
    /// returns `None`. The Content-Type header is cached after the first call
    /// to this function. As a result, subsequent calls will always return the
    /// same value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// use rocket::http::ContentType;
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// request.add_header(ContentType::JSON);
    /// assert_eq!(request.content_type(), Some(&ContentType::JSON));
    ///
    /// // The header is cached; it cannot be replaced after first access.
    /// request.replace_header(ContentType::HTML);
    /// assert_eq!(request.content_type(), Some(&ContentType::JSON));
    /// # });
    /// ```
    #[inline(always)]
    pub fn content_type(&self) -> Option<&ContentType> {
        self.state.content_type.get_or_set(|| {
            self.headers().get_one("Content-Type").and_then(|v| v.parse().ok())
        }).as_ref()
    }

    /// Returns the Accept header of `self`. If the header is not present,
    /// returns `None`. The Accept header is cached after the first call to this
    /// function. As a result, subsequent calls will always return the same
    /// value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// use rocket::http::Accept;
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// request.add_header(Accept::JSON);
    /// assert_eq!(request.accept(), Some(&Accept::JSON));
    ///
    /// // The header is cached; it cannot be replaced after first access.
    /// request.replace_header(Accept::HTML);
    /// assert_eq!(request.accept(), Some(&Accept::JSON));
    /// # });
    /// ```
    #[inline(always)]
    pub fn accept(&self) -> Option<&Accept> {
        self.state.accept.get_or_set(|| {
            self.headers().get_one("Accept").and_then(|v| v.parse().ok())
        }).as_ref()
    }

    /// Returns the media type "format" of the request.
    ///
    /// The "format" of a request is either the Content-Type, if the request
    /// methods indicates support for a payload, or the preferred media type in
    /// the Accept header otherwise. If the method indicates no payload and no
    /// Accept header is specified, a media type of `Any` is returned.
    ///
    /// The media type returned from this method is used to match against the
    /// `format` route attribute.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// use rocket::http::{Method, Accept, ContentType, MediaType};
    ///
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// request.add_header(ContentType::JSON);
    /// request.add_header(Accept::HTML);
    ///
    /// request.set_method(Method::Get);
    /// assert_eq!(request.format(), Some(&MediaType::HTML));
    ///
    /// request.set_method(Method::Post);
    /// assert_eq!(request.format(), Some(&MediaType::JSON));
    /// # });
    /// ```
    pub fn format(&self) -> Option<&MediaType> {
        static ANY: MediaType = MediaType::Any;
        if self.method().supports_payload() {
            self.content_type().map(|ct| ct.media_type())
        } else {
            // FIXME: Should we be using `accept_first` or `preferred`? Or
            // should we be checking neither and instead pass things through
            // where the client accepts the thing at all?
            self.accept()
                .map(|accept| accept.preferred().media_type())
                .or(Some(&ANY))
        }
    }

    /// Returns the configured application receive limits.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// let json_limit = request.limits().get("json");
    /// # });
    /// ```
    pub fn limits(&self) -> &'r Limits {
        &self.state.config.limits
    }

    /// Get the presently matched route, if any.
    ///
    /// This method returns `Some` any time a handler or its guards are being
    /// invoked. This method returns `None` _before_ routing has commenced; this
    /// includes during request fairing callbacks.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// # Request::example(Method::Get, "/uri", |mut request| {
    /// let route = request.route();
    /// # });
    /// ```
    pub fn route(&self) -> Option<&'r Route> {
        self.state.route.get()
    }

    /// Invokes the request guard implementation for `T`, returning its outcome.
    ///
    /// # Example
    ///
    /// Assuming a `User` request guard exists, invoke it:
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// # type User = Method;
    /// # Request::example(Method::Get, "/uri", |request| {
    /// let outcome = request.guard::<User>();
    /// # });
    /// ```
    ///
    /// Retrieve managed state inside of a guard implementation:
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # use rocket::http::Method;
    /// use rocket::State;
    ///
    /// # type Pool = usize;
    /// # Request::example(Method::Get, "/uri", |request| {
    /// let pool = request.guard::<State<Pool>>();
    /// # });
    /// ```
    #[inline(always)]
    pub fn guard<'a, T: FromRequest<'a, 'r>>(&'a self) -> Outcome<T, T::Error> {
        T::from_request(self)
    }

    /// Retrieves the cached value for type `T` from the request-local cached
    /// state of `self`. If no such value has previously been cached for this
    /// request, `f` is called to produce the value which is subsequently
    /// returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::http::Method;
    /// # use rocket::Request;
    /// # type User = ();
    /// fn current_user(request: &Request) -> User {
    ///     // Validate request for a given user, load from database, etc.
    /// }
    ///
    /// # Request::example(Method::Get, "/uri", |request| {
    /// let user = request.local_cache(|| current_user(request));
    /// # });
    /// ```
    pub fn local_cache<T, F>(&self, f: F) -> &T
        where F: FnOnce() -> T,
              T: Send + Sync + 'static
    {
        self.state.cache.try_get()
            .unwrap_or_else(|| {
                self.state.cache.set(f());
                self.state.cache.get()
            })
    }

    /// Retrieves and parses into `T` the 0-indexed `n`th segment from the
    /// request. Returns `None` if `n` is greater than the number of segments.
    /// Returns `Some(Err(T::Error))` if the parameter type `T` failed to be
    /// parsed from the `n`th dynamic parameter.
    ///
    /// This method exists only to be used by manual routing. To retrieve
    /// parameters from a request, use Rocket's code generation facilities.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{Request, http::Method};
    /// use rocket::http::{RawStr, uri::Origin};
    ///
    /// # Request::example(Method::Get, "/", |req| {
    /// fn string<'s>(req: &'s mut Request, uri: &'static str, n: usize) -> &'s RawStr {
    ///     req.set_uri(Origin::parse(uri).unwrap());
    ///
    ///     req.get_param(n)
    ///         .and_then(|r| r.ok())
    ///         .unwrap_or("unnamed".into())
    /// }
    ///
    /// assert_eq!(string(req, "/", 0).as_str(), "unnamed");
    /// assert_eq!(string(req, "/a/b/this_one", 0).as_str(), "a");
    /// assert_eq!(string(req, "/a/b/this_one", 1).as_str(), "b");
    /// assert_eq!(string(req, "/a/b/this_one", 2).as_str(), "this_one");
    /// assert_eq!(string(req, "/a/b/this_one", 3).as_str(), "unnamed");
    /// assert_eq!(string(req, "/a/b/c/d/e/f/g/h", 7).as_str(), "h");
    /// # });
    /// ```
    #[inline]
    pub fn get_param<'a, T>(&'a self, n: usize) -> Option<Result<T, T::Error>>
        where T: FromParam<'a>
    {
        Some(T::from_param(self.raw_segment_str(n)?))
    }

    /// Retrieves and parses into `T` all of the path segments in the request
    /// URI beginning and including the 0-indexed `n`th non-empty segment. `T`
    /// must implement [`FromSegments`], which is used to parse the segments.
    ///
    /// This method exists only to be used by manual routing. To retrieve
    /// segments from a request, use Rocket's code generation facilities.
    ///
    /// # Error
    ///
    /// If there are fewer than `n` non-empty segments, returns `None`. If
    /// parsing the segments failed, returns `Some(Err(T:Error))`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{Request, http::Method};
    /// use std::path::PathBuf;
    ///
    /// use rocket::http::uri::Origin;
    ///
    /// # Request::example(Method::Get, "/", |req| {
    /// fn path<'s>(req: &'s mut Request, uri: &'static str, n: usize) -> PathBuf {
    ///     req.set_uri(Origin::parse(uri).unwrap());
    ///
    ///     req.get_segments(n)
    ///         .and_then(|r| r.ok())
    ///         .unwrap_or_else(|| "whoops".into())
    /// }
    ///
    /// assert_eq!(path(req, "/", 0), PathBuf::from("whoops"));
    /// assert_eq!(path(req, "/a/", 0), PathBuf::from("a"));
    /// assert_eq!(path(req, "/a/b/c", 0), PathBuf::from("a/b/c"));
    /// assert_eq!(path(req, "/a/b/c", 1), PathBuf::from("b/c"));
    /// assert_eq!(path(req, "/a/b/c", 2), PathBuf::from("c"));
    /// assert_eq!(path(req, "/a/b/c", 6), PathBuf::from("whoops"));
    /// # });
    /// ```
    #[inline]
    pub fn get_segments<'a, T>(&'a self, n: usize) -> Option<Result<T, T::Error>>
        where T: FromSegments<'a>
    {
        Some(T::from_segments(self.raw_segments(n)?))
    }

    /// Retrieves and parses into `T` the query value with key `key`. `T` must
    /// implement [`FromFormValue`], which is used to parse the query's value.
    /// Key matching is performed case-sensitively. If there are multiple pairs
    /// with key `key`, the _last_ one is returned.
    ///
    /// This method exists only to be used by manual routing. To retrieve
    /// query values from a request, use Rocket's code generation facilities.
    ///
    /// # Error
    ///
    /// If a query segment with key `key` isn't present, returns `None`. If
    /// parsing the value fails, returns `Some(Err(T:Error))`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{Request, http::Method};
    /// use std::path::PathBuf;
    /// use rocket::http::{RawStr, uri::Origin};
    ///
    /// # Request::example(Method::Get, "/", |req| {
    /// fn value<'s>(req: &'s mut Request, uri: &'static str, key: &str) -> &'s RawStr {
    ///     req.set_uri(Origin::parse(uri).unwrap());
    ///
    ///     req.get_query_value(key)
    ///         .and_then(|r| r.ok())
    ///         .unwrap_or("n/a".into())
    /// }
    ///
    /// assert_eq!(value(req, "/?a=apple&z=zebra", "a").as_str(), "apple");
    /// assert_eq!(value(req, "/?a=apple&z=zebra", "z").as_str(), "zebra");
    /// assert_eq!(value(req, "/?a=apple&z=zebra", "A").as_str(), "n/a");
    /// assert_eq!(value(req, "/?a=apple&z=zebra&a=argon", "a").as_str(), "argon");
    /// assert_eq!(value(req, "/?a=1&a=2&a=3&b=4", "a").as_str(), "3");
    /// assert_eq!(value(req, "/?a=apple&z=zebra", "apple").as_str(), "n/a");
    /// # });
    /// ```
    #[inline]
    pub fn get_query_value<'a, T>(&'a self, key: &str) -> Option<Result<T, T::Error>>
        where T: FromFormValue<'a>
    {
        self.raw_query_items()?
            .rev()
            .find(|item| item.key.as_str() == key)
            .map(|item| T::from_form_value(item.value))
    }
}

// All of these methods only exist for internal, including codegen, purposes.
// They _are not_ part of the stable API.
#[doc(hidden)]
impl<'r> Request<'r> {
    // Only used by doc-tests! Needs to be `pub` because doc-test are external.
    pub fn example<F: Fn(&mut Request<'_>)>(method: Method, uri: &str, f: F) {
        let rocket = Rocket::custom(Config::development());
        let uri = Origin::parse(uri).expect("invalid URI in example");
        let mut request = Request::new(&rocket, method, uri);
        f(&mut request);
    }

    // Updates the cached `path_segments` and `query_items` in `self.state`.
    // MUST be called whenever a new URI is set or updated.
    #[inline]
    fn update_cached_uri_info(&mut self) {
        let path_segments = Segments(self.uri.path())
            .map(|s| indices(s, self.uri.path()))
            .collect();

        let query_items = self.uri.query()
            .map(|query_str| FormItems::from(query_str)
                 .map(|item| IndexedFormItem::from(query_str, item))
                 .collect()
            );

        self.state.path_segments = path_segments;
        self.state.query_items = query_items;
    }

    /// Get the `n`th path segment, 0-indexed, after the mount point for the
    /// currently matched route, as a string, if it exists. Used by codegen.
    #[inline]
    pub fn raw_segment_str(&self, n: usize) -> Option<&RawStr> {
        self.routed_path_segment(n)
            .map(|(i, j)| self.uri.path()[i..j].into())
    }

    /// Get the segments beginning at the `n`th, 0-indexed, after the mount
    /// point for the currently matched route, if they exist. Used by codegen.
    #[inline]
    pub fn raw_segments(&self, n: usize) -> Option<Segments<'_>> {
        self.routed_path_segment(n)
            .map(|(i, _)| Segments(&self.uri.path()[i..]) )
    }

    // Returns an iterator over the raw segments of the path URI. Does not take
    // into account the current route. This is used during routing.
    #[inline]
    crate fn raw_path_segments(&self) -> impl Iterator<Item = &RawStr> {
        let path = self.uri.path();
        self.state.path_segments.iter().cloned()
            .map(move |(i, j)| path[i..j].into())
    }

    #[inline]
    fn routed_path_segment(&self, n: usize) -> Option<(usize, usize)> {
        let mount_segments = self.route()
            .map(|r| r.base.segment_count())
            .unwrap_or(0);

        self.state.path_segments.get(mount_segments + n).map(|(i, j)| (*i, *j))
    }

    // Retrieves the pre-parsed query items. Used by matching and codegen.
    #[inline]
    pub fn raw_query_items(
        &self
    ) -> Option<impl Iterator<Item = FormItem<'_>> + DoubleEndedIterator + Clone> {
        let query = self.uri.query()?;
        self.state.query_items.as_ref().map(move |items| {
            items.iter().map(move |item| item.convert(query))
        })
    }

    /// Set `self`'s parameters given that the route used to reach this request
    /// was `route`. Use during routing when attempting a given route.
    #[inline(always)]
    crate fn set_route(&self, route: &'r Route) {
        self.state.route.set(Some(route));
    }

    /// Set the method of `self`, even when `self` is a shared reference. Used
    /// during routing to override methods for re-routing.
    #[inline(always)]
    crate fn _set_method(&self, method: Method) {
        self.method.set(method);
    }

    /// Convert from Hyper types into a Rocket Request.
    crate fn from_hyp(
        rocket: &'r Rocket,
        h_method: hyper::Method,
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

        // We need to re-parse the URI since we don't trust Hyper... :(
        let uri = Origin::parse_owned(uri).map_err(|e| e.to_string())?;

        // Construct the request object.
        let mut request = Request::new(rocket, method, uri);
        request.set_remote(h_addr);

        // Set the request cookies, if they exist.
        if let Some(cookie_headers) = h_headers.get_raw("Cookie") {
            let mut cookie_jar = CookieJar::new();
            for header in cookie_headers {
                let raw_str = match std::str::from_utf8(header) {
                    Ok(string) => string,
                    Err(_) => continue
                };

                for cookie_str in raw_str.split(';').map(|s| s.trim()) {
                    if let Some(cookie) = Cookies::parse_cookie(cookie_str) {
                        cookie_jar.add_original(cookie);
                    }
                }
            }

            request.state.cookies = RefCell::new(cookie_jar);
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

impl fmt::Debug for Request<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Request")
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("headers", &self.headers())
            .field("remote", &self.remote())
            .finish()
    }
}

impl fmt::Display for Request<'_> {
    /// Pretty prints a Request. This is primarily used by Rocket's logging
    /// infrastructure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", Paint::green(self.method()), Paint::blue(&self.uri))?;

        // Print the requests media type when the route specifies a format.
        if let Some(media_type) = self.format() {
            if !media_type.is_any() {
                write!(f, " {}", Paint::yellow(media_type))?;
            }
        }

        Ok(())
    }
}

impl IndexedFormItem {
    #[inline(always)]
    fn from(s: &str, i: FormItem<'_>) -> Self {
        let (r, k, v) = (indices(i.raw, s), indices(i.key, s), indices(i.value, s));
        IndexedFormItem { raw: r, key: k, value: v }
    }

    #[inline(always)]
    fn convert<'s>(&self, source: &'s str) -> FormItem<'s> {
        FormItem {
            raw: source[self.raw.0..self.raw.1].into(),
            key: source[self.key.0..self.key.1].into(),
            value: source[self.value.0..self.value.1].into(),
        }
    }
}

fn indices(needle: &str, haystack: &str) -> (usize, usize) {
    Indexed::checked_from(needle, haystack)
        .expect("segments inside of path/query")
        .indices()
}
