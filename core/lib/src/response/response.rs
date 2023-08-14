use std::{fmt, str};
use std::borrow::Cow;
use std::collections::HashMap;
use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncSeek};

use crate::http::{Header, HeaderMap, Status, ContentType, Cookie};
use crate::http::uncased::{Uncased, AsUncased};
use crate::data::IoHandler;
use crate::response::Body;

/// Builder for the [`Response`] type.
///
/// Building a [`Response`] can be a low-level ordeal; this structure presents a
/// higher-level API that simplifies building `Response`s.
///
/// # Usage
///
/// `Builder` follows the builder pattern and is usually obtained by calling
/// [`Response::build()`] on `Response`. Almost all methods take the current
/// builder as a mutable reference and return the same mutable reference with
/// field(s) modified in the `Response` being built. These method calls can be
/// chained: `build.a().b()`.
///
/// To finish building and retrieve the built `Response`, use the
/// [`finalize()`](#method.finalize) or [`ok()`](#method.ok) methods.
///
/// ## Headers
///
/// When building a `Response`, headers can either be _replaced_ or _adjoined_;
/// the default behavior (using `header(..)`) is to _replace_. When a header is
/// _replaced_, any existing values for headers with the same name are removed,
/// and the new value is set. If no header exists, the header is simply added.
/// On the other hand, when a header is _adjoined_, all existing values will
/// remain, and the `value` of the adjoined header will be added to the set of
/// existing values, if any. Adjoining maintains order: headers adjoined first
/// will appear first in the `Response`.
///
/// ## Joining and Merging
///
/// It is often necessary to combine multiple `Response`s in some way. The
/// [merge](#method.merge) and [join](#method.join) methods facilitate this. The
/// `merge` method replaces all of the fields in `self` with those present in
/// `other`. The `join` method sets any fields not set in `self` to the value in
/// `other`. See their documentation for more details.
/// ## Example
///
/// The following example builds a `Response` with:
///
///   * **Status**: `418 I'm a teapot`
///   * **Content-Type** header: `text/plain; charset=utf-8`
///   * **X-Teapot-Make** header: `Rocket`
///   * **X-Teapot-Model** headers: `Utopia`, `Series 1`
///   * **Body**: fixed-size string `"Brewing the best coffee!"`
///
/// ```rust
/// use std::io::Cursor;
/// use rocket::response::Response;
/// use rocket::http::{Status, ContentType};
///
/// let body = "Brewing the best coffee!";
/// let response = Response::build()
///     .status(Status::ImATeapot)
///     .header(ContentType::Plain)
///     .raw_header("X-Teapot-Make", "Rocket")
///     .raw_header("X-Teapot-Model", "Utopia")
///     .raw_header_adjoin("X-Teapot-Model", "Series 1")
///     .sized_body(body.len(), Cursor::new(body))
///     .finalize();
/// ```
pub struct Builder<'r> {
    response: Response<'r>,
}

impl<'r> Builder<'r> {
    /// Creates a new `Builder` that will build on top of the `base`
    /// `Response`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::{Builder, Response};
    ///
    /// # #[allow(unused_variables)]
    /// let builder = Builder::new(Response::new());
    /// ```
    #[inline(always)]
    pub fn new(base: Response<'r>) -> Builder<'r> {
        Builder {
            response: base,
        }
    }

    /// Sets the status of the `Response` being built to `status`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Status;
    ///
    /// let response = Response::build()
    ///     .status(Status::NotFound)
    ///     .finalize();
    /// ```
    #[inline(always)]
    pub fn status(&mut self, status: Status) -> &mut Builder<'r> {
        self.response.set_status(status);
        self
    }

    /// Adds `header` to the `Response`, replacing any header with the same name
    /// that already exists in the response. If multiple headers with
    /// the same name exist, they are all removed, and only the new header and
    /// value will remain.
    ///
    /// The type of `header` can be any type that implements `Into<Header>`.
    /// This includes `Header` itself, [`ContentType`](crate::http::ContentType) and
    /// [hyper::header types](crate::http::hyper::header).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::ContentType;
    ///
    /// let response = Response::build()
    ///     .header(ContentType::JSON)
    ///     .header(ContentType::HTML)
    ///     .finalize();
    ///
    /// assert_eq!(response.headers().get("Content-Type").count(), 1);
    /// ```
    #[inline(always)]
    pub fn header<'h: 'r, H>(&mut self, header: H) -> &mut Builder<'r>
        where H: Into<Header<'h>>
    {
        self.response.set_header(header);
        self
    }

    /// Adds `header` to the `Response` by adjoining the header with any
    /// existing headers with the same name that already exist in the
    /// `Response`. This allows for multiple headers with the same name and
    /// potentially different values to be present in the `Response`.
    ///
    /// The type of `header` can be any type that implements `Into<Header>`.
    /// This includes `Header` itself, [`ContentType`](crate::http::ContentType) and
    /// [hyper::header types](crate::http::hyper::header).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Header;
    /// use rocket::http::hyper::header::ACCEPT;
    ///
    /// let response = Response::build()
    ///     .header_adjoin(Header::new(ACCEPT.as_str(), "application/json"))
    ///     .header_adjoin(Header::new(ACCEPT.as_str(), "text/plain"))
    ///     .finalize();
    ///
    /// assert_eq!(response.headers().get("Accept").count(), 2);
    /// ```
    #[inline(always)]
    pub fn header_adjoin<'h: 'r, H>(&mut self, header: H) -> &mut Builder<'r>
        where H: Into<Header<'h>>
    {
        self.response.adjoin_header(header);
        self
    }

    /// Adds a custom header to the `Response` with the given name and value,
    /// replacing any header with the same name that already exists in the
    /// response. If multiple headers with the same name exist, they are all
    /// removed, and only the new header and value will remain.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    ///
    /// let response = Response::build()
    ///     .raw_header("X-Custom", "first")
    ///     .raw_header("X-Custom", "second")
    ///     .finalize();
    ///
    /// assert_eq!(response.headers().get("X-Custom").count(), 1);
    /// ```
    #[inline(always)]
    pub fn raw_header<'a, 'b, N, V>(&mut self, name: N, value: V) -> &mut Builder<'r>
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>, 'a: 'r, 'b: 'r
    {
        self.response.set_raw_header(name, value);
        self
    }

    /// Adds custom header to the `Response` with the given name and value,
    /// adjoining the header with any existing headers with the same name that
    /// already exist in the `Response`. This allows for multiple headers with
    /// the same name and potentially different values to be present in the
    /// `Response`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    ///
    /// let response = Response::build()
    ///     .raw_header_adjoin("X-Custom", "first")
    ///     .raw_header_adjoin("X-Custom", "second")
    ///     .finalize();
    ///
    /// assert_eq!(response.headers().get("X-Custom").count(), 2);
    /// ```
    #[inline(always)]
    pub fn raw_header_adjoin<'a, 'b, N, V>(&mut self, name: N, value: V) -> &mut Builder<'r>
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>, 'a: 'r, 'b: 'r
    {
        self.response.adjoin_raw_header(name, value);
        self
    }

    /// Sets the body of the `Response` to be the fixed-sized `body` with size
    /// `size`, which may be `None`. If `size` is `None`, the body's size will
    /// be computed with calls to `seek` when the response is written out.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// let body = "Hello, world!";
    /// let response = Response::build()
    ///     .sized_body(body.len(), Cursor::new(body))
    ///     .finalize();
    /// ```
    pub fn sized_body<B, S>(&mut self, size: S, body: B) -> &mut Builder<'r>
        where B: AsyncRead + AsyncSeek + Send + 'r,
              S: Into<Option<usize>>
    {
        self.response.set_sized_body(size, body);
        self
    }

    /// Sets the body of the `Response` to be the streamed `body`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// let response = Response::build()
    ///     .streamed_body(Cursor::new("Hello, world!"))
    ///     .finalize();
    /// ```
    #[inline(always)]
    pub fn streamed_body<B>(&mut self, body: B) -> &mut Builder<'r>
        where B: AsyncRead + Send + 'r
    {
        self.response.set_streamed_body(body);
        self
    }

    /// Registers `handler` as the I/O handler for upgrade protocol `protocol`.
    ///
    /// This is equivalent to [`Response::add_upgrade()`].
    ///
    /// **NOTE**: Responses registering I/O handlers for upgraded protocols
    /// **should not** set the response status to `101 Switching Protocols`, nor set the
    /// `Connection` or `Upgrade` headers. Rocket automatically sets these
    /// headers as needed. See [`Response`#upgrading] for details.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::pin::Pin;
    ///
    /// use rocket::Response;
    /// use rocket::data::{IoHandler, IoStream};
    /// use rocket::tokio::io;
    ///
    /// struct EchoHandler;
    ///
    /// #[rocket::async_trait]
    /// impl IoHandler for EchoHandler {
    ///     async fn io(self: Pin<Box<Self>>, io: IoStream) -> io::Result<()> {
    ///         let (mut reader, mut writer) = io::split(io);
    ///         io::copy(&mut reader, &mut writer).await?;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let response = Response::build()
    ///     .upgrade("raw-echo", EchoHandler)
    ///     .streamed_body(std::io::Cursor::new("We didn't upgrade!"))
    ///     .finalize();
    /// ```
    #[inline(always)]
    pub fn upgrade<P, H>(&mut self, protocol: P, handler: H) -> &mut Builder<'r>
        where P: Into<Uncased<'r>>, H: IoHandler + 'r
    {
        self.response.add_upgrade(protocol.into(), handler);
        self
    }

    /// Sets the max chunk size of a body, if any, to `size`.
    ///
    /// See [`Response::set_max_chunk_size()`] for notes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// let response = Response::build()
    ///     .streamed_body(Cursor::new("Hello, world!"))
    ///     .max_chunk_size(3072)
    ///     .finalize();
    /// ```
    #[inline(always)]
    pub fn max_chunk_size(&mut self, size: usize) -> &mut Builder<'r> {
        self.response.set_max_chunk_size(size);
        self
    }

    /// Merges the `other` `Response` into `self` by setting any fields in
    /// `self` to the corresponding value in `other` if they are set in `other`.
    /// Fields in `self` are unchanged if they are not set in `other`. If a
    /// header is set in both `self` and `other`, the values in `other` are
    /// kept. Headers set only in `self` remain.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::{Status, ContentType};
    ///
    /// let base = Response::build()
    ///     .status(Status::NotFound)
    ///     .header(ContentType::HTML)
    ///     .raw_header("X-Custom", "value 1")
    ///     .finalize();
    ///
    /// let response = Response::build()
    ///     .status(Status::ImATeapot)
    ///     .raw_header("X-Custom", "value 2")
    ///     .raw_header_adjoin("X-Custom", "value 3")
    ///     .merge(base)
    ///     .finalize();
    ///
    /// assert_eq!(response.status(), Status::NotFound);
    ///
    /// let ctype: Vec<_> = response.headers().get("Content-Type").collect();
    /// assert_eq!(ctype, vec![ContentType::HTML.to_string()]);
    ///
    /// let custom_values: Vec<_> = response.headers().get("X-Custom").collect();
    /// assert_eq!(custom_values, vec!["value 1"]);
    /// ```
    #[inline(always)]
    pub fn merge(&mut self, other: Response<'r>) -> &mut Builder<'r> {
        self.response.merge(other);
        self
    }

    /// Joins the `other` `Response` into `self` by setting any fields in `self`
    /// to the corresponding value in `other` if they are set in `self`. Fields
    /// in `self` are unchanged if they are already set. If a header is set in
    /// both `self` and `other`, the values are adjoined, with the values in
    /// `self` coming first. Headers only in `self` or `other` are set in
    /// `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::{Status, ContentType};
    ///
    /// let other = Response::build()
    ///     .status(Status::NotFound)
    ///     .header(ContentType::HTML)
    ///     .raw_header("X-Custom", "value 1")
    ///     .finalize();
    ///
    /// let response = Response::build()
    ///     .status(Status::ImATeapot)
    ///     .raw_header("X-Custom", "value 2")
    ///     .raw_header_adjoin("X-Custom", "value 3")
    ///     .join(other)
    ///     .finalize();
    ///
    /// assert_eq!(response.status(), Status::ImATeapot);
    ///
    /// let ctype: Vec<_> = response.headers().get("Content-Type").collect();
    /// assert_eq!(ctype, vec![ContentType::HTML.to_string()]);
    ///
    /// let custom_values: Vec<_> = response.headers().get("X-Custom").collect();
    /// assert_eq!(custom_values, vec!["value 2", "value 3", "value 1"]);
    /// ```
    #[inline(always)]
    pub fn join(&mut self, other: Response<'r>) -> &mut Builder<'r> {
        self.response.join(other);
        self
    }

    /// Return the `Response` structure that was being built by this builder.
    /// After calling this method, `self` is cleared and must be rebuilt as if
    /// from `new()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    ///
    /// use rocket::Response;
    /// use rocket::http::Status;
    ///
    /// let body = "Brewing the best coffee!";
    /// let response = Response::build()
    ///     .status(Status::ImATeapot)
    ///     .sized_body(body.len(), Cursor::new(body))
    ///     .raw_header("X-Custom", "value 2")
    ///     .finalize();
    /// ```
    pub fn finalize(&mut self) -> Response<'r> {
        std::mem::replace(&mut self.response, Response::new())
    }

    /// Retrieve the built `Response` wrapped in `Ok`. After calling this
    /// method, `self` is cleared and must be rebuilt as if from `new()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    ///
    /// let response: Result<Response, ()> = Response::build()
    ///     // build the response
    ///     .ok();
    ///
    /// assert!(response.is_ok());
    /// ```
    #[inline(always)]
    pub fn ok<E>(&mut self) -> Result<Response<'r>, E> {
        Ok(self.finalize())
    }
}

/// A response, as returned by types implementing
/// [`Responder`](crate::response::Responder).
///
/// See [`Builder`] for docs on how a `Response` is typically created and the
/// [module docs](crate::response) for notes on composing responses
///
/// ## Upgrading
///
/// A response may optionally register [`IoHandler`]s for upgraded requests via
/// [`Response::add_upgrade()`] or the corresponding builder method
/// [`Builder::upgrade()`]. If the incoming request 1) requests an upgrade via a
/// `Connection: Upgrade` header _and_ 2) includes a protocol in its `Upgrade`
/// header that is registered by the returned `Response`, the connection will be
/// upgraded. An upgrade response is sent to the client, and the registered
/// `IoHandler` for the client's preferred protocol is invoked with an
/// [`IoStream`](crate::data::IoStream) representing a raw byte stream to the
/// client. Note that protocol names are treated case-insensitively during
/// matching.
///
/// If a connection is upgraded, Rocket automatically set the following in the
/// upgrade response:
///   * The response status to `101 Switching Protocols`.
///   * The `Connection: Upgrade` header.
///   * The `Upgrade` header's value to the selected protocol.
///
/// As such, a response **should never** set a `101` status nor the `Connection`
/// or `Upgrade` headers: Rocket handles this automatically. Instead, it should
/// set a status and headers to use in case the connection is not upgraded,
/// either due to an error or because the client did not request an upgrade.
///
/// If a connection _is not_ upgraded due to an error, even though there was a
/// matching, registered protocol, the `IoHandler` is not invoked, and the
/// original response is sent to the client without alteration.
#[derive(Default)]
pub struct Response<'r> {
    status: Option<Status>,
    headers: HeaderMap<'r>,
    body: Body<'r>,
    upgrade: HashMap<Uncased<'r>, Pin<Box<dyn IoHandler + 'r>>>,
}

impl<'r> Response<'r> {
    /// Creates a new, empty `Response` without a status, body, or headers.
    /// Because all HTTP responses must have a status, if a default `Response`
    /// is written to the client without a status, the status defaults to `200
    /// Ok`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Status;
    ///
    /// let mut response = Response::new();
    ///
    /// assert_eq!(response.status(), Status::Ok);
    /// assert_eq!(response.headers().len(), 0);
    /// assert!(response.body().is_none());
    /// ```
    #[inline(always)]
    pub fn new() -> Response<'r> {
        Response::default()
    }

    /// Returns a `Builder` with a base of `Response::new()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    ///
    /// # #[allow(unused_variables)]
    /// let builder = Response::build();
    /// ```
    #[inline(always)]
    pub fn build() -> Builder<'r> {
        Response::build_from(Response::new())
    }

    /// Returns a `Builder` with a base of `other`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![allow(unused_variables)]
    /// use rocket::Response;
    ///
    /// let other = Response::new();
    /// let builder = Response::build_from(other);
    /// ```
    #[inline(always)]
    pub fn build_from(other: Response<'r>) -> Builder<'r> {
        Builder::new(other)
    }

    /// Returns the status of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Status;
    ///
    /// let mut response = Response::new();
    /// assert_eq!(response.status(), Status::Ok);
    ///
    /// response.set_status(Status::NotFound);
    /// assert_eq!(response.status(), Status::NotFound);
    /// ```
    #[inline(always)]
    pub fn status(&self) -> Status {
        self.status.unwrap_or(Status::Ok)
    }

    /// Sets the status of `self` to `status`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Status;
    ///
    /// let mut response = Response::new();
    /// response.set_status(Status::ImATeapot);
    /// assert_eq!(response.status(), Status::ImATeapot);
    /// ```
    #[inline(always)]
    pub fn set_status(&mut self, status: Status) {
        self.status = Some(status);
    }

    /// Returns the Content-Type header of `self`. If the header is not present
    /// or is malformed, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::ContentType;
    ///
    /// let mut response = Response::new();
    /// response.set_header(ContentType::HTML);
    /// assert_eq!(response.content_type(), Some(ContentType::HTML));
    /// ```
    #[inline(always)]
    pub fn content_type(&self) -> Option<ContentType> {
        self.headers().get_one("Content-Type").and_then(|v| v.parse().ok())
    }

    /// Returns an iterator over the cookies in `self` as identified by the
    /// `Set-Cookie` header. Malformed cookies are skipped.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Cookie;
    ///
    /// let mut response = Response::new();
    /// response.set_header(Cookie::new("hello", "world!"));
    /// let cookies: Vec<_> = response.cookies().collect();
    /// assert_eq!(cookies, vec![Cookie::new("hello", "world!")]);
    /// ```
    pub fn cookies(&self) -> impl Iterator<Item = Cookie<'_>> {
        self.headers()
            .get("Set-Cookie")
            .filter_map(|header| Cookie::parse_encoded(header).ok())
    }

    /// Returns a [`HeaderMap`] of all of the headers in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Header;
    ///
    /// let mut response = Response::new();
    /// response.adjoin_raw_header("X-Custom", "1");
    /// response.adjoin_raw_header("X-Custom", "2");
    ///
    /// let mut custom_headers = response.headers().iter();
    /// assert_eq!(custom_headers.next(), Some(Header::new("X-Custom", "1")));
    /// assert_eq!(custom_headers.next(), Some(Header::new("X-Custom", "2")));
    /// assert_eq!(custom_headers.next(), None);
    /// ```
    #[inline(always)]
    pub fn headers(&self) -> &HeaderMap<'r> {
        &self.headers
    }

    /// Sets the header `header` in `self`. Any existing headers with the name
    /// `header.name` will be lost, and only `header` will remain. The type of
    /// `header` can be any type that implements `Into<Header>`. This includes
    /// `Header` itself, [`ContentType`](crate::http::ContentType) and
    /// [`hyper::header` types](crate::http::hyper::header).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::ContentType;
    ///
    /// let mut response = Response::new();
    ///
    /// response.set_header(ContentType::HTML);
    /// assert_eq!(response.headers().iter().next(), Some(ContentType::HTML.into()));
    /// assert_eq!(response.headers().len(), 1);
    ///
    /// response.set_header(ContentType::JSON);
    /// assert_eq!(response.headers().iter().next(), Some(ContentType::JSON.into()));
    /// assert_eq!(response.headers().len(), 1);
    /// ```
    #[inline(always)]
    pub fn set_header<'h: 'r, H: Into<Header<'h>>>(&mut self, header: H) -> bool {
        self.headers.replace(header)
    }

    /// Sets the custom header with name `name` and value `value` in `self`. Any
    /// existing headers with the same `name` will be lost, and the new custom
    /// header will remain. This method should be used sparingly; prefer to use
    /// [set_header](#method.set_header) instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Header;
    ///
    /// let mut response = Response::new();
    ///
    /// response.set_raw_header("X-Custom", "1");
    /// assert_eq!(response.headers().get_one("X-Custom"), Some("1"));
    /// assert_eq!(response.headers().len(), 1);
    ///
    /// response.set_raw_header("X-Custom", "2");
    /// assert_eq!(response.headers().get_one("X-Custom"), Some("2"));
    /// assert_eq!(response.headers().len(), 1);
    /// ```
    #[inline(always)]
    pub fn set_raw_header<'a: 'r, 'b: 'r, N, V>(&mut self, name: N, value: V) -> bool
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>
    {
        self.set_header(Header::new(name, value))
    }

    /// Adds the header `header` to `self`. If `self` contains headers with the
    /// name `header.name`, another header with the same name and value
    /// `header.value` is added. The type of `header` can be any type that
    /// implements `Into<Header>`. This includes `Header` itself,
    /// [`ContentType`](crate::http::ContentType) and [`hyper::header`
    /// types](crate::http::hyper::header).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Header;
    /// use rocket::http::hyper::header::ACCEPT;
    ///
    /// let mut response = Response::new();
    /// response.adjoin_header(Header::new(ACCEPT.as_str(), "application/json"));
    /// response.adjoin_header(Header::new(ACCEPT.as_str(), "text/plain"));
    ///
    /// let mut accept_headers = response.headers().iter();
    /// assert_eq!(accept_headers.next(), Some(Header::new(ACCEPT.as_str(), "application/json")));
    /// assert_eq!(accept_headers.next(), Some(Header::new(ACCEPT.as_str(), "text/plain")));
    /// assert_eq!(accept_headers.next(), None);
    /// ```
    #[inline(always)]
    pub fn adjoin_header<'h: 'r, H: Into<Header<'h>>>(&mut self, header: H) {
        self.headers.add(header)
    }

    /// Adds a custom header with name `name` and value `value` to `self`. If
    /// `self` already contains headers with the name `name`, another header
    /// with the same `name` and `value` is added. The type of `header` can be
    /// any type implements `Into<Header>`. This includes `Header` itself,
    /// [`ContentType`](crate::http::ContentType) and [`hyper::header`
    /// types](crate::http::hyper::header).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Header;
    ///
    /// let mut response = Response::new();
    /// response.adjoin_raw_header("X-Custom", "one");
    /// response.adjoin_raw_header("X-Custom", "two");
    ///
    /// let mut custom_headers = response.headers().iter();
    /// assert_eq!(custom_headers.next(), Some(Header::new("X-Custom", "one")));
    /// assert_eq!(custom_headers.next(), Some(Header::new("X-Custom", "two")));
    /// assert_eq!(custom_headers.next(), None);
    /// ```
    #[inline(always)]
    pub fn adjoin_raw_header<'a: 'r, 'b: 'r, N, V>(&mut self, name: N, value: V)
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>
    {
        self.adjoin_header(Header::new(name, value));
    }

    /// Removes all headers with the name `name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    ///
    /// let mut response = Response::new();
    ///
    /// response.adjoin_raw_header("X-Custom", "one");
    /// response.adjoin_raw_header("X-Custom", "two");
    /// response.adjoin_raw_header("X-Other", "hi");
    /// assert_eq!(response.headers().len(), 3);
    ///
    /// response.remove_header("X-Custom");
    /// assert_eq!(response.headers().len(), 1);
    /// ```
    #[inline(always)]
    pub fn remove_header(&mut self, name: &str) {
        self.headers.remove(name);
    }

    /// Returns an immutable borrow of the body of `self`, if there is one.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// # rocket::async_test(async {
    /// let mut response = Response::new();
    /// assert!(response.body().is_none());
    ///
    /// let string = "Hello, world!";
    /// response.set_sized_body(string.len(), Cursor::new(string));
    /// assert!(response.body().is_some());
    /// # })
    /// ```
    #[inline(always)]
    pub fn body(&self) -> &Body<'r> {
        &self.body
    }

    /// Returns `Ok(Some(_))` if `self` contains a suitable handler for any of
    /// the comma-separated protocols any of the strings in `I`. Returns
    /// `Ok(None)` if `self` doesn't support any kind of upgrade. Returns
    /// `Err(_)` if `protocols` is non-empty but no match was found in `self`.
    pub(crate) fn take_upgrade<I: Iterator<Item = &'r str>>(
        &mut self,
        protocols: I
    ) -> Result<Option<(Uncased<'r>, Pin<Box<dyn IoHandler + 'r>>)>, ()> {
        if self.upgrade.is_empty() {
            return Ok(None);
        }

        let mut protocols = protocols.peekable();
        let have_protocols = protocols.peek().is_some();
        let found = protocols
            .flat_map(|v| v.split(',').map(str::trim))
            .find_map(|p| self.upgrade.remove_entry(p.as_uncased()));

        match found {
            Some(handler) => Ok(Some(handler)),
            None if have_protocols => Err(()),
            None => Ok(None)
        }
    }

    /// Returns the [`IoHandler`] for the protocol `proto`.
    ///
    /// Returns `Some` if such a handler was registered via
    /// [`Response::add_upgrade()`] or the corresponding builder method
    /// [`upgrade()`](Builder::upgrade()). Otherwise returns `None`.
    ///
    /// ```rust
    /// use std::pin::Pin;
    ///
    /// use rocket::Response;
    /// use rocket::data::{IoHandler, IoStream};
    /// use rocket::tokio::io;
    ///
    /// struct EchoHandler;
    ///
    /// #[rocket::async_trait]
    /// impl IoHandler for EchoHandler {
    ///     async fn io(self: Pin<Box<Self>>, io: IoStream) -> io::Result<()> {
    ///         let (mut reader, mut writer) = io::split(io);
    ///         io::copy(&mut reader, &mut writer).await?;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # rocket::async_test(async {
    /// let mut response = Response::new();
    /// assert!(response.upgrade("raw-echo").is_none());
    ///
    /// response.add_upgrade("raw-echo", EchoHandler);
    /// assert!(response.upgrade("raw-echo").is_some());
    /// # })
    /// ```
    pub fn upgrade(&mut self, proto: &str) -> Option<Pin<&mut (dyn IoHandler + 'r)>> {
        self.upgrade.get_mut(proto.as_uncased()).map(|h| h.as_mut())
    }

    /// Returns a mutable borrow of the body of `self`, if there is one. A
    /// mutable borrow allows for reading the body.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// # rocket::async_test(async {
    /// let mut response = Response::new();
    /// assert!(response.body().is_none());
    ///
    /// let string = "Hello, world!";
    /// response.set_sized_body(string.len(), Cursor::new(string));
    /// let string = response.body_mut().to_string().await;
    /// assert_eq!(string.unwrap(), "Hello, world!");
    /// # })
    /// ```
    #[inline(always)]
    pub fn body_mut(&mut self) -> &mut Body<'r> {
        &mut self.body
    }

    // Makes the `AsyncRead`er in the body empty but leaves the size of the body
    // if it exists. Meant to be used during HEAD handling.
    #[inline(always)]
    pub(crate) fn strip_body(&mut self) {
        self.body.strip();
    }

    /// Sets the body of `self` to be the fixed-sized `body` with size
    /// `size`, which may be `None`. If `size` is `None`, the body's size will
    /// be computing with calls to `seek` just before being written out in a
    /// response.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::Response;
    ///
    /// # let o: io::Result<()> = rocket::async_test(async {
    /// let string = "Hello, world!";
    ///
    /// let mut response = Response::new();
    /// response.set_sized_body(string.len(), io::Cursor::new(string));
    /// assert_eq!(response.body_mut().to_string().await?, "Hello, world!");
    /// # Ok(())
    /// # });
    /// # assert!(o.is_ok());
    /// ```
    pub fn set_sized_body<B, S>(&mut self, size: S, body: B)
        where B: AsyncRead + AsyncSeek + Send + 'r,
              S: Into<Option<usize>>
    {
        self.body = Body::with_sized(body, size.into());
    }

    /// Sets the body of `self` to `body`, which will be streamed.
    ///
    /// The max chunk size is configured via [`Response::set_max_chunk_size()`]
    /// and defaults to [`Body::DEFAULT_MAX_CHUNK`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::io;
    /// use tokio::io::{repeat, AsyncReadExt};
    /// use rocket::Response;
    ///
    /// # let o: io::Result<()> = rocket::async_test(async {
    /// let mut response = Response::new();
    /// response.set_streamed_body(repeat(97).take(5));
    /// assert_eq!(response.body_mut().to_string().await?, "aaaaa");
    /// # Ok(())
    /// # });
    /// # assert!(o.is_ok());
    /// ```
    #[inline(always)]
    pub fn set_streamed_body<B>(&mut self, body: B)
        where B: AsyncRead + Send + 'r
    {
        self.body = Body::with_unsized(body);
    }

    /// Registers `handler` as the I/O handler for upgrade protocol `protocol`.
    ///
    /// Responses registering I/O handlers for upgraded protocols **should not**
    /// set the response status to `101`, nor set the `Connection` or `Upgrade`
    /// headers. Rocket automatically sets these headers as needed. See
    /// [`Response`#upgrading] for details.
    ///
    /// If a handler was previously registered for `protocol`, this `handler`
    /// replaces it. If the connection is upgraded to `protocol`, the last
    /// `handler` registered for the protocol is used to handle the connection.
    /// See [`IoHandler`] for details on implementing an I/O handler. For
    /// details on connection upgrading, see [`Response`#upgrading].
    ///
    /// [`Response`#upgrading]: Response#upgrading
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::pin::Pin;
    ///
    /// use rocket::Response;
    /// use rocket::data::{IoHandler, IoStream};
    /// use rocket::tokio::io;
    ///
    /// struct EchoHandler;
    ///
    /// #[rocket::async_trait]
    /// impl IoHandler for EchoHandler {
    ///     async fn io(self: Pin<Box<Self>>, io: IoStream) -> io::Result<()> {
    ///         let (mut reader, mut writer) = io::split(io);
    ///         io::copy(&mut reader, &mut writer).await?;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # rocket::async_test(async {
    /// let mut response = Response::new();
    /// assert!(response.upgrade("raw-echo").is_none());
    ///
    /// response.add_upgrade("raw-echo", EchoHandler);
    /// assert!(response.upgrade("raw-echo").is_some());
    /// # })
    /// ```
    pub fn add_upgrade<N, H>(&mut self, protocol: N, handler: H)
        where N: Into<Uncased<'r>>, H: IoHandler + 'r
    {
        self.upgrade.insert(protocol.into(), Box::pin(handler));
    }

    /// Sets the body's maximum chunk size to `size` bytes.
    ///
    /// The default max chunk size is [`Body::DEFAULT_MAX_CHUNK`]. The max chunk
    /// size is a property of the body and is thus reset whenever a body is set
    /// via [`Response::set_streamed_body()`], [`Response::set_sized_body()`],
    /// or the corresponding builder methods.
    ///
    /// This setting does not typically need to be changed. Configuring a high
    /// value can result in high memory usage. Similarly, configuring a low
    /// value can result in excessive network writes. When unsure, leave the
    /// value unchanged.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tokio::io::{repeat, AsyncReadExt};
    /// use rocket::Response;
    ///
    /// # let o: Option<()> = rocket::async_test(async {
    /// let mut response = Response::new();
    /// response.set_streamed_body(repeat(97).take(5));
    /// response.set_max_chunk_size(3072);
    /// # Some(())
    /// # });
    /// # assert!(o.is_some());
    #[inline(always)]
    pub fn set_max_chunk_size(&mut self, size: usize) {
        self.body_mut().set_max_chunk_size(size);
    }

    /// Replaces this response's status and body with that of `other`, if they
    /// exist in `other`. Any headers that exist in `other` replace the ones in
    /// `self`. Any in `self` that aren't in `other` remain in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::{Status, ContentType};
    ///
    /// let base = Response::build()
    ///     .status(Status::NotFound)
    ///     .header(ContentType::HTML)
    ///     .raw_header("X-Custom", "value 1")
    ///     .finalize();
    ///
    /// let response = Response::build()
    ///     .status(Status::ImATeapot)
    ///     .raw_header("X-Custom", "value 2")
    ///     .raw_header_adjoin("X-Custom", "value 3")
    ///     .merge(base)
    ///     .finalize();
    ///
    /// assert_eq!(response.status(), Status::NotFound);
    ///
    /// let ctype: Vec<_> = response.headers().get("Content-Type").collect();
    /// assert_eq!(ctype, vec![ContentType::HTML.to_string()]);
    ///
    /// let custom_values: Vec<_> = response.headers().get("X-Custom").collect();
    /// assert_eq!(custom_values, vec!["value 1"]);
    /// ```
    pub fn merge(&mut self, other: Response<'r>) {
        if let Some(status) = other.status {
            self.status = Some(status);
        }

        if other.body().is_some() {
            self.body = other.body;
        }

        for (name, values) in other.headers.into_iter_raw() {
            self.headers.replace_all(name.into_cow(), values);
        }
    }

    /// Sets `self`'s status and body to that of `other` if they are not already
    /// set in `self`. Any headers present in both `other` and `self` are
    /// adjoined.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::{Status, ContentType};
    ///
    /// let other = Response::build()
    ///     .status(Status::NotFound)
    ///     .header(ContentType::HTML)
    ///     .raw_header("X-Custom", "value 1")
    ///     .finalize();
    ///
    /// let response = Response::build()
    ///     .status(Status::ImATeapot)
    ///     .raw_header("X-Custom", "value 2")
    ///     .raw_header_adjoin("X-Custom", "value 3")
    ///     .join(other)
    ///     .finalize();
    ///
    /// assert_eq!(response.status(), Status::ImATeapot);
    ///
    /// let ctype: Vec<_> = response.headers().get("Content-Type").collect();
    /// assert_eq!(ctype, vec![ContentType::HTML.to_string()]);
    ///
    /// let custom_values: Vec<_> = response.headers().get("X-Custom").collect();
    /// assert_eq!(custom_values, vec!["value 2", "value 3", "value 1"]);
    /// ```
    pub fn join(&mut self, other: Response<'r>) {
        if self.status.is_none() {
            self.status = other.status;
        }

        if self.body.is_none() {
            self.body = other.body;
        }

        for (name, mut values) in other.headers.into_iter_raw() {
            self.headers.add_all(name.into_cow(), &mut values);
        }
    }
}

impl fmt::Debug for Response<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.status())?;

        for header in self.headers().iter() {
            writeln!(f, "{}", header)?;
        }

        self.body.fmt(f)
    }
}
