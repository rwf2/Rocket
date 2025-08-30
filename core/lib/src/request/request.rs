use std::{io, fmt};
use std::ops::RangeFrom;
use std::sync::{Arc, atomic::Ordering};
use std::borrow::Cow;
use std::str::FromStr;
use std::future::Future;
use std::net::IpAddr;

use http::Version;
use rocket_http::HttpVersion;
use state::{TypeMap, InitCell};
use futures::future::BoxFuture;
use ref_swap::OptionRefSwap;

use crate::{Rocket, Route, Orbit};
use crate::request::{FromParam, FromSegments, FromRequest, Outcome, AtomicMethod};
use crate::form::{self, ValueField, FromForm};
use crate::data::Limits;

use crate::http::ProxyProto;
use crate::http::{Method, Header, HeaderMap, ContentType, Accept, MediaType, CookieJar, Cookie};
use crate::http::uri::{fmt::Path, Origin, Segments, Host, Authority};
use crate::listener::{Certificates, Endpoint};

/// The type of an incoming web request.
///
/// This should be used sparingly in Rocket applications. In particular, it
/// should likely only be used when writing [`FromRequest`] implementations. It
/// contains all of the information for a given web request except for the body
/// data. This includes the HTTP method, URI, cookies, headers, and more.
#[derive(Clone)]
pub struct Request<'r> {
    method: AtomicMethod,
    uri: Origin<'r>,
    headers: HeaderMap<'r>,
    pub(crate) version: Option<HttpVersion>,
    pub(crate) errors: Vec<RequestError>,
    pub(crate) connection: ConnectionMeta,
    pub(crate) state: RequestState<'r>,
}

/// Information derived from an incoming connection, if any.
#[derive(Clone, Default)]
pub(crate) struct ConnectionMeta {
    pub peer_endpoint: Option<Endpoint>,
    #[cfg_attr(not(feature = "mtls"), allow(dead_code))]
    pub peer_certs: Option<Arc<Certificates<'static>>>,
    #[cfg_attr(feature = "tls", allow(dead_code))]
    pub server_name: Option<String>,
}

impl ConnectionMeta {
    pub fn new(
        endpoint: io::Result<Endpoint>,
        certs: Option<Certificates<'_>>,
        server_name: Option<&str>) -> Self {
        ConnectionMeta {
            peer_endpoint: endpoint.ok(),
            peer_certs: certs.map(|c| c.into_owned()).map(Arc::new),
            server_name: server_name.map(|s| s.to_string()),
        }
    }
}

/// Information derived from the request.
pub(crate) struct RequestState<'r> {
    pub rocket: &'r Rocket<Orbit>,
    pub route: OptionRefSwap<'r, Route>,
    pub cookies: CookieJar<'r>,
    pub accept: InitCell<Option<Accept>>,
    pub content_type: InitCell<Option<ContentType>>,
    pub cache: Arc<TypeMap![Send + Sync]>,
    pub host: Option<Host<'r>>,
}

impl Clone for RequestState<'_> {
    fn clone(&self) -> Self {
        RequestState {
            rocket: self.rocket,
            route: OptionRefSwap::new(self.route.load(Ordering::Acquire)),
            cookies: self.cookies.clone(),
            accept: self.accept.clone(),
            content_type: self.content_type.clone(),
            cache: self.cache.clone(),
            host: self.host.clone(),
        }
    }
}

impl<'r> Request<'r> {
    /// Create a new `Request` with the given `method` and `uri`.
    #[inline(always)]
    pub(crate) fn new<'s: 'r>(
        rocket: &'r Rocket<Orbit>,
        method: Method,
        uri: Origin<'s>,
        version: Option<HttpVersion>,
    ) -> Request<'r> {
        Request {
            uri,
            method: AtomicMethod::new(method),
            headers: HeaderMap::new(),
            version,
            errors: Vec::new(),
            connection: ConnectionMeta::default(),
            state: RequestState {
                rocket,
                route: OptionRefSwap::new(None),
                cookies: CookieJar::new(None, rocket),
                accept: InitCell::new(),
                content_type: InitCell::new(),
                cache: Arc::new(<TypeMap![Send + Sync]>::new()),
                host: None,
            }
        }
    }

    /// Retrieve http protocol version, when applicable.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::HttpVersion;
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # req.override_version(HttpVersion::Http11);
    /// assert_eq!(req.version(), Some(HttpVersion::Http11));
    /// ```
    pub fn version(&self) -> Option<HttpVersion> {
        self.version
    }

    /// Retrieve the method from `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::Method;
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// # let post = |uri| c.post(uri);
    /// assert_eq!(get("/").method(), Method::Get);
    /// assert_eq!(post("/").method(), Method::Post);
    /// ```
    #[inline(always)]
    pub fn method(&self) -> Method {
        self.method.load()
    }

    /// Set the method of `self` to `method`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::Method;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    ///
    /// assert_eq!(request.method(), Method::Get);
    ///
    /// request.set_method(Method::Post);
    /// assert_eq!(request.method(), Method::Post);
    /// ```
    #[inline(always)]
    pub fn set_method(&mut self, method: Method) {
        self.method.set(method);
    }

    /// Borrow the [`Origin`] URI from `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// assert_eq!(get("/hello/rocketeer").uri().path(), "/hello/rocketeer");
    /// assert_eq!(get("/hello").uri().query(), None);
    /// ```
    #[inline(always)]
    pub fn uri(&self) -> &Origin<'r> {
        &self.uri
    }

    /// Set the URI in `self` to `uri`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::uri::Origin;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    ///
    /// let uri = Origin::parse("/hello/Sergio?type=greeting").unwrap();
    /// request.set_uri(uri);
    /// assert_eq!(request.uri().path(), "/hello/Sergio");
    /// assert_eq!(request.uri().query().unwrap(), "type=greeting");
    ///
    /// let new_uri = request.uri().map_path(|p| format!("/foo{}", p)).unwrap();
    /// request.set_uri(new_uri);
    /// assert_eq!(request.uri().path(), "/foo/hello/Sergio");
    /// assert_eq!(request.uri().query().unwrap(), "type=greeting");
    /// ```
    #[inline(always)]
    pub fn set_uri(&mut self, uri: Origin<'r>) {
        self.uri = uri;
    }

    /// Returns the [`Host`] identified in the request, if any.
    ///
    /// If the request is made via HTTP/1.1 (or earlier), this method returns
    /// the value in the `HOST` header without the deprecated `user_info`
    /// component. Otherwise, this method returns the contents of the
    /// `:authority` pseudo-header request field.
    ///
    /// Note that this method _only_ reflects the `HOST` header in the _initial_
    /// request and not any changes made thereafter. To change the value
    /// returned by this method, use [`Request::set_host()`].
    ///
    /// # ⚠️ DANGER ⚠️
    ///
    /// Using the user-controlled `host` to construct URLs is a security hazard!
    /// _Never_ do so without first validating the host against a whitelist. For
    /// this reason, Rocket disallows constructing host-prefixed URIs with
    /// [`uri!`]. _Always_ use [`uri!`] to construct URIs.
    ///
    /// [`uri!`]: crate::uri!
    ///
    /// # Example
    ///
    /// Retrieve the raw host, unusable to construct safe URIs:
    ///
    /// ```rust
    /// use rocket::http::uri::Host;
    /// # use rocket::uri;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    ///
    /// assert_eq!(request.host(), None);
    ///
    /// request.set_host(Host::from(uri!("rocket.rs")));
    /// let host = request.host().unwrap();
    /// assert_eq!(host.domain(), "rocket.rs");
    /// assert_eq!(host.port(), None);
    ///
    /// request.set_host(Host::from(uri!("rocket.rs:2392")));
    /// let host = request.host().unwrap();
    /// assert_eq!(host.domain(), "rocket.rs");
    /// assert_eq!(host.port(), Some(2392));
    /// ```
    ///
    /// Retrieve the raw host, check it against a whitelist, and construct a
    /// URI:
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// # type Token = String;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    /// use rocket::http::uri::Host;
    ///
    /// // A sensitive URI we want to prefix with safe hosts.
    /// #[get("/token?<secret>")]
    /// fn token(secret: Token) { /* .. */ }
    ///
    /// // Whitelist of known hosts. In a real setting, you might retrieve this
    /// // list from config at ignite-time using tools like `AdHoc::config()`.
    /// const WHITELIST: [Host<'static>; 3] = [
    ///     Host::new(uri!("rocket.rs")),
    ///     Host::new(uri!("rocket.rs:443")),
    ///     Host::new(uri!("guide.rocket.rs:443")),
    /// ];
    ///
    /// // A request with a host of "rocket.rs". Note the case-insensitivity.
    /// request.set_host(Host::from(uri!("ROCKET.rs")));
    /// let prefix = request.host().and_then(|h| h.to_absolute("https", &WHITELIST));
    ///
    /// // `rocket.rs` is in the whitelist, so we'll get back a `Some`.
    /// assert!(prefix.is_some());
    /// if let Some(prefix) = prefix {
    ///     // We can use this prefix to safely construct URIs.
    ///     let uri = uri!(prefix, token("some-secret-token"));
    ///     assert_eq!(uri, "https://ROCKET.rs/token?secret=some-secret-token");
    /// }
    ///
    /// // A request with a host of "attacker-controlled.com".
    /// request.set_host(Host::from(uri!("attacker-controlled.com")));
    /// let prefix = request.host().and_then(|h| h.to_absolute("https", &WHITELIST));
    ///
    /// // `attacker-controlled.come` is _not_ on the whitelist.
    /// assert!(prefix.is_none());
    /// assert!(request.host().is_some());
    /// ```
    #[inline(always)]
    pub fn host(&self) -> Option<&Host<'r>> {
        self.state.host.as_ref()
    }

    /// Returns the resolved SNI server name requested in the TLS handshake, if
    /// any.
    ///
    /// Ideally, this will match the `Host` header in the request.
    #[cfg(feature = "tls")]
    #[inline(always)]
    pub fn sni(&mut self) -> Option<&str> {
        self.connection.server_name.as_deref()
    }

    /// Sets the host of `self` to `host`.
    ///
    /// # Example
    ///
    /// Set the host to `rocket.rs:443`.
    ///
    /// ```rust
    /// use rocket::http::uri::Host;
    /// # use rocket::uri;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    ///
    /// assert_eq!(request.host(), None);
    ///
    /// request.set_host(Host::from(uri!("rocket.rs:443")));
    /// let host = request.host().unwrap();
    /// assert_eq!(host.domain(), "rocket.rs");
    /// assert_eq!(host.port(), Some(443));
    /// ```
    #[inline(always)]
    pub fn set_host(&mut self, host: Host<'r>) {
        self.state.host = Some(host);
    }

    /// Returns the raw address of the remote connection that initiated this
    /// request if the address is known. If the address is not known, `None` is
    /// returned.
    ///
    /// Because it is common for proxies to forward connections for clients, the
    /// remote address may contain information about the proxy instead of the
    /// client. For this reason, proxies typically set a "X-Real-IP" header
    /// [`ip_header`](crate::Config::ip_header) with the client's true IP. To
    /// extract this IP from the request, use the [`real_ip()`] or
    /// [`client_ip()`] methods.
    ///
    /// [`real_ip()`]: #method.real_ip
    /// [`client_ip()`]: #method.client_ip
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// use rocket::listener::Endpoint;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    ///
    /// assert_eq!(request.remote(), None);
    ///
    /// let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8111);
    /// request.set_remote(Endpoint::Tcp(localhost));
    /// assert_eq!(request.remote().unwrap().tcp().unwrap(), localhost);
    /// ```
    #[inline(always)]
    pub fn remote(&self) -> Option<&Endpoint> {
        self.connection.peer_endpoint.as_ref()
    }

    /// Sets the remote address of `self` to `address`.
    ///
    /// # Example
    ///
    /// Set the remote address to be 127.0.0.1:8111:
    ///
    /// ```rust
    /// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// use rocket::listener::Endpoint;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    ///
    /// assert_eq!(request.remote(), None);
    ///
    /// let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8111);
    /// request.set_remote(Endpoint::Tcp(localhost));
    /// assert_eq!(request.remote().unwrap().tcp().unwrap(), localhost);
    /// ```
    #[inline(always)]
    pub fn set_remote(&mut self, endpoint: Endpoint) {
        self.connection.peer_endpoint = Some(endpoint);
    }

    /// Returns the IP address of the configured
    /// [`ip_header`](crate::Config::ip_header) of the request if such a header
    /// is configured, exists and contains a valid IP address.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::net::Ipv4Addr;
    /// use rocket::http::Header;
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let req = c.get("/");
    /// assert_eq!(req.real_ip(), None);
    ///
    /// // `ip_header` defaults to `X-Real-IP`.
    /// let req = req.header(Header::new("X-Real-IP", "127.0.0.1"));
    /// assert_eq!(req.real_ip(), Some(Ipv4Addr::LOCALHOST.into()));
    /// ```
    pub fn real_ip(&self) -> Option<IpAddr> {
        let ip_header = self.rocket().config.ip_header.as_ref()?.as_str();
        self.headers()
            .get_one(ip_header)
            .and_then(|ip| {
                ip.parse()
                    .map_err(|_| warn!(value = ip, "'{ip_header}' header is malformed"))
                    .ok()
            })
    }

    /// Returns the [`ProxyProto`] associated with the current request.
    ///
    /// The value is determined by inspecting the header named
    /// [`proxy_proto_header`](crate::Config::proxy_proto_header), if
    /// configured, and parsing it case-insensitivity. If the parameter isn't
    /// configured or the request doesn't contain a header named as indicated,
    /// this method returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::{Header, ProxyProto};
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let req = c.get("/");
    /// // By default, no `proxy_proto_header` is configured.
    /// let req = req.header(Header::new("x-forwarded-proto", "https"));
    /// assert_eq!(req.proxy_proto(), None);
    ///
    /// // We can configure one by setting the `proxy_proto_header` parameter.
    /// // Here we set it to `x-forwarded-proto`, considered de-facto standard.
    /// # let figment = rocket::figment::Figment::from(rocket::Config::debug_default());
    /// let figment = figment.merge(("proxy_proto_header", "x-forwarded-proto"));
    /// # let c = rocket::local::blocking::Client::debug(rocket::custom(figment)).unwrap();
    /// # let req = c.get("/");
    /// let req = req.header(Header::new("x-forwarded-proto", "https"));
    /// assert_eq!(req.proxy_proto(), Some(ProxyProto::Https));
    ///
    /// # let req = c.get("/");
    /// let req = req.header(Header::new("x-forwarded-proto", "HTTP"));
    /// assert_eq!(req.proxy_proto(), Some(ProxyProto::Http));
    ///
    /// # let req = c.get("/");
    /// let req = req.header(Header::new("x-forwarded-proto", "xproto"));
    /// assert_eq!(req.proxy_proto(), Some(ProxyProto::Unknown("xproto".into())));
    /// ```
    pub fn proxy_proto(&self) -> Option<ProxyProto<'_>> {
        self.rocket()
            .config
            .proxy_proto_header
            .as_ref()
            .and_then(|header| self.headers().get_one(header.as_str()))
            .map(ProxyProto::from)
    }

    /// Returns whether we are *likely* in a secure context.
    ///
    /// A request is in a "secure context" if it was initially sent over a
    /// secure (TLS, via HTTPS) connection. If TLS is configured and enabled,
    /// then the request is guaranteed to be in a secure context. Otherwise, if
    /// [`Request::proxy_proto()`] evaluates to `Https`, then we are _likely_ to
    /// be in a secure context. We say _likely_ because it is entirely possible
    /// for the header to indicate that the connection is being proxied via
    /// HTTPS while reality differs. As such, this value should not be trusted
    /// when 100% confidence is a necessity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::{Header, ProxyProto};
    ///
    /// # let client = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let req = client.get("/");
    /// // If TLS and proxy_proto are disabled, we are not in a secure context.
    /// assert_eq!(req.context_is_likely_secure(), false);
    ///
    /// // Configuring proxy_proto and receiving a header value of `https` is
    /// // interpreted as likely being in a secure context.
    /// // Here we set it to `x-forwarded-proto`, considered de-facto standard.
    /// # let figment = rocket::figment::Figment::from(rocket::Config::debug_default());
    /// let figment = figment.merge(("proxy_proto_header", "x-forwarded-proto"));
    /// # let c = rocket::local::blocking::Client::debug(rocket::custom(figment)).unwrap();
    /// # let req = c.get("/");
    /// let req = req.header(Header::new("x-forwarded-proto", "https"));
    /// assert_eq!(req.context_is_likely_secure(), true);
    /// ```
    pub fn context_is_likely_secure(&self) -> bool {
        self.cookies().state.secure
    }

    /// Attempts to return the client's IP address by first inspecting the
    /// [`ip_header`](crate::Config::ip_header) and then using the remote
    /// connection's IP address. Note that the built-in `IpAddr` request guard
    /// can be used to retrieve the same information in a handler:
    ///
    /// ```rust
    /// # use rocket::get;
    /// use std::net::IpAddr;
    ///
    /// #[get("/")]
    /// fn get_ip(client_ip: IpAddr) { /* ... */ }
    ///
    /// #[get("/")]
    /// fn try_get_ip(client_ip: Option<IpAddr>) { /* ... */ }
    /// ````
    ///
    /// If the `ip_header` exists and contains a valid IP address, that address
    /// is returned. Otherwise, if the address of the remote connection is
    /// known, that address is returned. Otherwise, `None` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::http::Header;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    /// # use std::net::{SocketAddr, IpAddr, Ipv4Addr};
    /// # use rocket::listener::Endpoint;
    ///
    /// // starting without an "X-Real-IP" header or remote address
    /// assert!(request.client_ip().is_none());
    ///
    /// // add a remote address; this is done by Rocket automatically
    /// let localhost_9190 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9190);
    /// request.set_remote(Endpoint::Tcp(localhost_9190));
    /// assert_eq!(request.client_ip().unwrap(), Ipv4Addr::LOCALHOST);
    ///
    /// // now with an X-Real-IP header, the default value for `ip_header`.
    /// request.add_header(Header::new("X-Real-IP", "8.8.8.8"));
    /// assert_eq!(request.client_ip().unwrap(), Ipv4Addr::new(8, 8, 8, 8));
    /// ```
    #[inline]
    pub fn client_ip(&self) -> Option<IpAddr> {
        self.real_ip().or_else(|| self.remote()?.ip())
    }

    /// Returns a wrapped borrow to the cookies in `self`.
    ///
    /// [`CookieJar`] implements internal mutability, so this method allows you
    /// to get _and_ add/remove cookies in `self`.
    ///
    /// # Example
    ///
    /// Add a new cookie to a request's cookies:
    ///
    /// ```rust
    /// use rocket::http::Cookie;
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let request = c.get("/");
    /// # let req = request.inner();
    /// req.cookies().add(("key", "val"));
    /// req.cookies().add(("ans", format!("life: {}", 38 + 4)));
    ///
    /// assert_eq!(req.cookies().get_pending("key").unwrap().value(), "val");
    /// assert_eq!(req.cookies().get_pending("ans").unwrap().value(), "life: 42");
    /// ```
    #[inline(always)]
    pub fn cookies(&self) -> &CookieJar<'r> {
        &self.state.cookies
    }

    /// Returns a [`HeaderMap`] of all of the headers in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::{Accept, ContentType};
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// assert!(get("/").headers().is_empty());
    ///
    /// let req = get("/").header(Accept::HTML).header(ContentType::HTML);
    /// assert_eq!(req.headers().len(), 2);
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
    /// use rocket::http::ContentType;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    ///
    /// assert!(request.headers().is_empty());
    ///
    /// request.add_header(ContentType::HTML);
    /// assert!(request.headers().contains("Content-Type"));
    /// assert_eq!(request.headers().len(), 1);
    /// ```
    #[inline]
    pub fn add_header<'h: 'r, H: Into<Header<'h>>>(&mut self, header: H) {
        let header = header.into();
        self.bust_header_cache(&header, false);
        self.headers.add(header);
    }

    /// Replaces the value of the header with name `header.name` with
    /// `header.value`. If no such header exists, `header` is added as a header
    /// to `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let mut req = c.get("/");
    /// # let request = req.inner_mut();
    ///
    /// assert!(request.headers().is_empty());
    ///
    /// request.add_header(ContentType::Any);
    /// assert_eq!(request.headers().get_one("Content-Type"), Some("*/*"));
    /// assert_eq!(request.content_type(), Some(&ContentType::Any));
    ///
    /// request.replace_header(ContentType::PNG);
    /// assert_eq!(request.headers().get_one("Content-Type"), Some("image/png"));
    /// assert_eq!(request.content_type(), Some(&ContentType::PNG));
    /// ```
    #[inline]
    pub fn replace_header<'h: 'r, H: Into<Header<'h>>>(&mut self, header: H) {
        let header = header.into();
        self.bust_header_cache(&header, true);
        self.headers.replace(header);
    }

    /// Returns the Content-Type header of `self`. If the header is not present,
    /// returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::ContentType;
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// assert_eq!(get("/").content_type(), None);
    ///
    /// let req = get("/").header(ContentType::JSON);
    /// assert_eq!(req.content_type(), Some(&ContentType::JSON));
    /// ```
    #[inline]
    pub fn content_type(&self) -> Option<&ContentType> {
        self.state.content_type
            .get_or_init(|| self.headers().get_one("Content-Type").and_then(|v| v.parse().ok()))
            .as_ref()
    }

    /// Returns the Accept header of `self`. If the header is not present,
    /// returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::Accept;
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// assert_eq!(get("/").accept(), None);
    /// assert_eq!(get("/").header(Accept::JSON).accept(), Some(&Accept::JSON));
    /// ```
    #[inline]
    pub fn accept(&self) -> Option<&Accept> {
        self.state.accept
            .get_or_init(|| self.headers().get_one("Accept").and_then(|v| v.parse().ok()))
            .as_ref()
    }

    /// Returns the media type "format" of the request.
    ///
    /// The returned `MediaType` is derived from either the `Content-Type` or
    /// the `Accept` header of the request, based on whether the request's
    /// method allows a body (see [`Method::allows_request_body()`]). The table
    /// below summarized this:
    ///
    /// | Method Allows Body | Returned Format                 |
    /// |--------------------|---------------------------------|
    /// | Always             | `Option<ContentType>`           |
    /// | Maybe or Never     | `Some(Preferred Accept or Any)` |
    ///
    /// In short, if the request's method indicates support for a payload, the
    /// request's `Content-Type` header value, if any, is returned. Otherwise
    /// the [preferred](Accept::preferred()) `Accept` header value is returned,
    /// or if none is present, [`Accept::Any`].
    ///
    /// The media type returned from this method is used to match against the
    /// `format` route attribute.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::{Accept, ContentType, MediaType};
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// # let post = |uri| c.post(uri);
    ///
    /// // Non-payload-bearing: format is accept header.
    /// let req = get("/").header(Accept::HTML);
    /// assert_eq!(req.format(), Some(&MediaType::HTML));
    ///
    /// let req = get("/").header(ContentType::JSON).header(Accept::HTML);
    /// assert_eq!(req.format(), Some(&MediaType::HTML));
    ///
    /// // Payload: format is content-type header.
    /// let req = post("/").header(ContentType::HTML);
    /// assert_eq!(req.format(), Some(&MediaType::HTML));
    ///
    /// let req = post("/").header(ContentType::JSON).header(Accept::HTML);
    /// assert_eq!(req.format(), Some(&MediaType::JSON));
    ///
    /// // Non-payload-bearing method and no accept header: `Any`.
    /// assert_eq!(get("/").format(), Some(&MediaType::Any));
    /// ```
    pub fn format(&self) -> Option<&MediaType> {
        static ANY: MediaType = MediaType::Any;
        if self.method().allows_request_body().unwrap_or(false) {
            self.content_type().map(|ct| ct.media_type())
        } else {
            // TODO: Should we be using `accept_first` or `preferred`? Or
            // should we be checking neither and instead pass things through
            // where the client accepts the thing at all?
            self.accept()
                .map(|accept| accept.preferred().media_type())
                .or(Some(&ANY))
        }
    }

    /// Returns the [`Rocket`] instance that is handling this request.
    ///
    /// # Example
    ///
    /// ```rust
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let request = c.get("/");
    /// # type Pool = usize;
    /// // Retrieve the application config via `Rocket::config()`.
    /// let config = request.rocket().config();
    ///
    /// // Retrieve managed state via `Rocket::state()`.
    /// let state = request.rocket().state::<Pool>();
    ///
    /// // Get a list of all of the registered routes and catchers.
    /// let routes = request.rocket().routes();
    /// let catchers = request.rocket().catchers();
    /// ```
    #[inline(always)]
    pub fn rocket(&self) -> &'r Rocket<Orbit> {
        self.state.rocket
    }

    /// Returns the configured application data limits.
    ///
    /// This is convenience function equivalent to:
    ///
    /// ```rust
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let request = c.get("/");
    /// &request.rocket().config().limits
    /// # ;
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::ToByteUnit;
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let request = c.get("/");
    ///
    /// // This is the default `form` limit.
    /// assert_eq!(request.limits().get("form"), Some(32.kibibytes()));
    ///
    /// // Retrieve the limit for files with extension `.pdf`; etails to 1MiB.
    /// assert_eq!(request.limits().get("file/pdf"), Some(1.mebibytes()));
    /// ```
    #[inline(always)]
    pub fn limits(&self) -> &'r Limits {
        &self.rocket().config().limits
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
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let request = c.get("/");
    /// let route = request.route();
    /// ```
    #[inline(always)]
    pub fn route(&self) -> Option<&'r Route> {
        self.state.route.load(Ordering::Acquire)
    }

    /// Invokes the request guard implementation for `T`, returning its outcome.
    ///
    /// # Example
    ///
    /// Assuming a `User` request guard exists, invoke it:
    ///
    /// ```rust
    /// # type User = rocket::http::Method;
    /// # rocket::async_test(async move {
    /// # let c = rocket::local::asynchronous::Client::debug_with(vec![]).await.unwrap();
    /// # let request = c.get("/");
    /// let outcome = request.guard::<User>().await;
    /// # })
    /// ```
    #[inline(always)]
    pub fn guard<'z, 'a, T>(&'a self) -> BoxFuture<'z, Outcome<T, T::Error>>
        where T: FromRequest<'a> + 'z, 'a: 'z, 'r: 'z
    {
        T::from_request(self)
    }

    /// Retrieves the cached value for type `T` from the request-local cached
    /// state of `self`. If no such value has previously been cached for this
    /// request, `f` is called to produce the value which is subsequently
    /// returned.
    ///
    /// Different values of the same type _cannot_ be cached without using a
    /// proxy, wrapper type. To avoid the need to write these manually, or for
    /// libraries wishing to store values of public types, use the
    /// [`local_cache!`](crate::request::local_cache) or
    /// [`local_cache_once!`](crate::request::local_cache_once) macros to
    /// generate a locally anonymous wrapper type, store, and retrieve the
    /// wrapped value from request-local cache.
    ///
    /// # Example
    ///
    /// ```rust
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let request = c.get("/");
    /// // The first store into local cache for a given type wins.
    /// let value = request.local_cache(|| "hello");
    /// assert_eq!(*request.local_cache(|| "hello"), "hello");
    ///
    /// // The following return the cached, previously stored value for the type.
    /// assert_eq!(*request.local_cache(|| "goodbye"), "hello");
    /// ```
    #[inline]
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

    /// Retrieves the cached value for type `T` from the request-local cached
    /// state of `self`. If no such value has previously been cached for this
    /// request, `fut` is `await`ed to produce the value which is subsequently
    /// returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::Request;
    /// # type User = ();
    /// async fn current_user<'r>(request: &Request<'r>) -> User {
    ///     // validate request for a given user, load from database, etc
    /// }
    ///
    /// # rocket::async_test(async move {
    /// # let c = rocket::local::asynchronous::Client::debug_with(vec![]).await.unwrap();
    /// # let request = c.get("/");
    /// let current_user = request.local_cache_async(async {
    ///     current_user(&request).await
    /// }).await;
    /// # })
    /// ```
    #[inline]
    pub async fn local_cache_async<'a, T, F>(&'a self, fut: F) -> &'a T
        where F: Future<Output = T>,
              T: Send + Sync + 'static
    {
        match self.state.cache.try_get() {
            Some(s) => s,
            None => {
                self.state.cache.set(fut.await);
                self.state.cache.get()
            }
        }
    }

    /// Retrieves and parses into `T` the 0-indexed `n`th non-empty segment from
    /// the _routed_ request, that is, the `n`th segment _after_ the mount
    /// point. If the request has not been routed, then this is simply the `n`th
    /// non-empty request URI segment.
    ///
    /// Returns `None` if `n` is greater than the number of non-empty segments.
    /// Returns `Some(Err(T::Error))` if the parameter type `T` failed to be
    /// parsed from the `n`th dynamic parameter.
    ///
    /// This method exists only to be used by manual routing. To retrieve
    /// parameters from a request, use Rocket's code generation facilities.
    ///
    /// # Example
    ///
    /// ```rust
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// use rocket::error::Empty;
    ///
    /// assert_eq!(get("/a/b/c").param(0), Some(Ok("a")));
    /// assert_eq!(get("/a/b/c").param(1), Some(Ok("b")));
    /// assert_eq!(get("/a/b/c").param(2), Some(Ok("c")));
    /// assert_eq!(get("/a/b/c").param::<&str>(3), None);
    ///
    /// assert_eq!(get("/1/b/3").param(0), Some(Ok(1)));
    /// assert!(get("/1/b/3").param::<usize>(1).unwrap().is_err());
    /// assert_eq!(get("/1/b/3").param(2), Some(Ok(3)));
    ///
    /// assert_eq!(get("/").param::<&str>(0), Some(Err(Empty)));
    /// ```
    #[inline]
    pub fn param<'a, T>(&'a self, n: usize) -> Option<Result<T, T::Error>>
        where T: FromParam<'a>
    {
        self.routed_segment(n).map(T::from_param)
    }

    /// Retrieves and parses into `T` all of the path segments in the request
    /// URI beginning and including the 0-indexed `n`th non-empty segment
    /// _after_ the mount point.,that is, the `n`th segment _after_ the mount
    /// point. If the request has not been routed, then this is simply the `n`th
    /// non-empty request URI segment.
    ///
    /// `T` must implement [`FromSegments`], which is used to parse the
    /// segments. If there are no non-empty segments, the `Segments` iterator
    /// will be empty.
    ///
    /// This method exists only to be used by manual routing. To retrieve
    /// segments from a request, use Rocket's code generation facilities.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::path::PathBuf;
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// assert_eq!(get("/").segments(0..), Ok(PathBuf::new()));
    /// assert_eq!(get("/").segments(2..), Ok(PathBuf::new()));
    ///
    /// // Empty segments are skipped.
    /// assert_eq!(get("///").segments(2..), Ok(PathBuf::new()));
    /// assert_eq!(get("/a/b/c").segments(0..), Ok(PathBuf::from("a/b/c")));
    /// assert_eq!(get("/a/b/c").segments(1..), Ok(PathBuf::from("b/c")));
    /// assert_eq!(get("/a/b/c").segments(2..), Ok(PathBuf::from("c")));
    /// assert_eq!(get("/a/b/c").segments(3..), Ok(PathBuf::new()));
    /// assert_eq!(get("/a/b/c").segments(4..), Ok(PathBuf::new()));
    /// ```
    #[inline]
    pub fn segments<'a, T>(&'a self, n: RangeFrom<usize>) -> Result<T, T::Error>
        where T: FromSegments<'a>
    {
        T::from_segments(self.routed_segments(n))
    }

    /// Retrieves and parses into `T` the query value with field name `name`.
    /// `T` must implement [`FromForm`], which is used to parse the query's
    /// value. Key matching is performed case-sensitively.
    ///
    /// # Warning
    ///
    /// This method exists _only_ to be used by manual routing and should
    /// _never_ be used in a regular Rocket application. It is much more
    /// expensive to use this method than to retrieve query parameters via
    /// Rocket's codegen. To retrieve query values from a request, _always_
    /// prefer to use Rocket's code generation facilities.
    ///
    /// # Error
    ///
    /// If a query segment with name `name` isn't present, returns `None`. If
    /// parsing the value fails, returns `Some(Err(_))`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::FromForm;
    ///
    /// #[derive(Debug, PartialEq, FromForm)]
    /// struct Dog<'r> {
    ///     name: &'r str,
    ///     age: usize
    /// }
    ///
    /// # let c = rocket::local::blocking::Client::debug_with(vec![]).unwrap();
    /// # let get = |uri| c.get(uri);
    /// let req = get("/?a=apple&z=zebra&a=aardvark");
    /// assert_eq!(req.query_value::<&str>("a").unwrap(), Ok("apple"));
    /// assert_eq!(req.query_value::<&str>("z").unwrap(), Ok("zebra"));
    /// assert_eq!(req.query_value::<&str>("b"), None);
    ///
    /// let a_seq = req.query_value::<Vec<&str>>("a");
    /// assert_eq!(a_seq.unwrap().unwrap(), ["apple", "aardvark"]);
    ///
    /// let req = get("/?dog.name=Max+Fido&dog.age=3");
    /// let dog = req.query_value::<Dog>("dog");
    /// assert_eq!(dog.unwrap().unwrap(), Dog { name: "Max Fido", age: 3 });
    /// ```
    #[inline]
    pub fn query_value<'a, T>(&'a self, name: &str) -> Option<form::Result<'a, T>>
        where T: FromForm<'a>
    {
        if !self.query_fields().any(|f| f.name == name) {
            return None;
        }

        let mut ctxt = T::init(form::Options::Lenient);

        self.query_fields()
            .filter(|f| f.name == name)
            .for_each(|f| T::push_value(&mut ctxt, f.shift()));

        Some(T::finalize(ctxt))
    }
}

// All of these methods only exist for internal, including codegen, purposes.
// They _are not_ part of the stable API. Please, don't use these.
#[doc(hidden)]
impl<'r> Request<'r> {
    /// Resets the cached value (if any) for the header with name `name`.
    fn bust_header_cache(&mut self, header: &Header<'_>, replace: bool) {
        if header.name() == "Content-Type" {
            if self.content_type().is_none() || replace {
                self.state.content_type = InitCell::new();
            }
        } else if header.name() == "Accept" {
            if self.accept().is_none() || replace {
                self.state.accept = InitCell::new();
            }
        } else if Some(header.name()) == self.rocket().config.proxy_proto_header.as_deref() {
            if !self.cookies().state.secure || replace {
                self.cookies_mut().state.secure |= ProxyProto::from(header.value()).is_https();
            }
        }
    }

    /// Get the `n`th non-empty path segment, 0-indexed, after the mount point
    /// for the currently matched route, as a string, if it exists. Used by
    /// codegen.
    #[inline]
    pub fn routed_segment(&self, n: usize) -> Option<&str> {
        self.routed_segments(0..).get(n)
    }

    /// Get the segments beginning at the `range`, 0-indexed, after the mount
    /// point for the currently matched route, if they exist. Used by codegen.
    #[inline]
    pub fn routed_segments(&self, range: RangeFrom<usize>) -> Segments<'_, Path> {
        let mount_segments = self.route()
            .map(|r| r.uri.metadata.base_len)
            .unwrap_or(0);

        trace!(name: "segments", mount_segments, range.start);
        self.uri().path().segments().skip(mount_segments + range.start)
    }

    // Retrieves the pre-parsed query items. Used by matching and codegen.
    #[inline]
    pub fn query_fields(&self) -> impl Iterator<Item = ValueField<'_>> {
        self.uri().query()
            .map(|q| q.segments().map(ValueField::from))
            .into_iter()
            .flatten()
    }

    /// Set `self`'s parameters given that the route used to reach this request
    /// was `route`. Use during routing when attempting a given route.
    #[inline(always)]
    pub(crate) fn set_route(&self, route: &'r Route) {
        self.state.route.store(Some(route), Ordering::Release)
    }

    #[inline(always)]
    pub(crate) fn _set_method(&self, method: Method) {
        self.method.store(method)
    }

    pub(crate) fn cookies_mut(&mut self) -> &mut CookieJar<'r> {
        &mut self.state.cookies
    }

    /// Convert from Hyper types into a Rocket Request.
    pub(crate) fn from_hyp(
        rocket: &'r Rocket<Orbit>,
        hyper: &'r hyper::http::request::Parts,
        connection: ConnectionMeta,
    ) -> Result<Request<'r>, Request<'r>> {
        // Keep track of parsing errors; emit a `BadRequest` if any exist.
        let mut errors = vec![];

        // Ensure that the method is known.
        let method = match hyper.method {
            hyper::Method::GET => Method::Get,
            hyper::Method::PUT => Method::Put,
            hyper::Method::POST => Method::Post,
            hyper::Method::DELETE => Method::Delete,
            hyper::Method::OPTIONS => Method::Options,
            hyper::Method::HEAD => Method::Head,
            hyper::Method::TRACE => Method::Trace,
            hyper::Method::CONNECT => Method::Connect,
            hyper::Method::PATCH => Method::Patch,
            ref ext => Method::from_str(ext.as_str()).unwrap_or_else(|_| {
                errors.push(RequestError::BadMethod(hyper.method.clone()));
                Method::Get
            }),
        };

        // TODO: Keep around not just the path/query, but the rest, if there?
        let uri = hyper.uri.path_and_query()
            .map(|uri| {
                // In debug, make sure we agree with Hyper about URI validity.
                // If we disagree, log a warning but continue anyway; if this is
                // a security issue with Hyper, there isn't much we can do.
                #[cfg(debug_assertions)]
                if Origin::parse(uri.as_str()).is_err() {
                    warn!(
                        name: "uri_discord",
                        %uri,
                        "Hyper/Rocket URI validity discord: {uri}\n\
                        Hyper believes the URI is valid while Rocket disagrees.\n\
                        This is likely a Hyper bug with potential security implications.\n\
                        Please report this warning to Rocket's GitHub issue tracker."
                    )
                }

                Origin::new(uri.path(), uri.query().map(Cow::Borrowed))
            })
            .unwrap_or_else(|| {
                errors.push(RequestError::InvalidUri(hyper.uri.clone()));
                Origin::root().clone()
            });

        // Construct the request object; fill in metadata and headers next.
        let mut request = Request::new(rocket, method, uri, match hyper.version {
            Version::HTTP_09 => Some(HttpVersion::Http09),
            Version::HTTP_10 => Some(HttpVersion::Http10),
            Version::HTTP_11 => Some(HttpVersion::Http11),
            Version::HTTP_2 => Some(HttpVersion::Http2),
            Version::HTTP_3 => Some(HttpVersion::Http3),
            _ => None,
        });
        request.errors = errors;

        // Set the passed in connection metadata.
        request.connection = connection;

        // Determine + set host. On HTTP < 2, use the `HOST` header. Otherwise,
        // use the `:authority` pseudo-header which hyper makes part of the URI.
        // TODO: Use an `InitCell` to compute this later.
        request.state.host = if hyper.version < hyper::Version::HTTP_2 {
            hyper.headers.get("host").and_then(|h| Host::parse_bytes(h.as_bytes()).ok())
        } else {
            hyper.uri.host().map(|h| Host::new(Authority::new(None, h, hyper.uri.port_u16())))
        };

        // Set the request cookies, if they exist.
        for header in hyper.headers.get_all("Cookie") {
            let Ok(raw_str) = std::str::from_utf8(header.as_bytes()) else {
                continue
            };

            for cookie_str in raw_str.split(';').map(|s| s.trim()) {
                if let Ok(cookie) = Cookie::parse_encoded(cookie_str) {
                    request.state.cookies.add_original(cookie.into_owned());
                }
            }
        }

        // Set the rest of the headers. This is rather unfortunate and slow.
        for (header, value) in hyper.headers.iter() {
            // FIXME: This is rather unfortunate. Header values needn't be UTF8.
            let Ok(value) = std::str::from_utf8(value.as_bytes()) else {
                warn!(%header, "dropping header with invalid UTF-8");
                continue;
            };

            request.add_header(Header::new(header.as_str(), value));
        }

        match request.errors.is_empty() {
            true => Ok(request),
            false => Err(request),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum RequestError {
    InvalidUri(hyper::Uri),
    BadMethod(hyper::Method),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::InvalidUri(u) => write!(f, "invalid origin URI: {}", u),
            RequestError::BadMethod(m) => write!(f, "invalid or unrecognized method: {}", m),
        }
    }
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Request")
            .field("method", &self.method())
            .field("uri", &self.uri())
            .field("headers", &self.headers())
            .field("remote", &self.remote())
            .field("cookies", &self.cookies())
            .finish()
    }
}
