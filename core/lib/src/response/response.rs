use std::{io, fmt, str};
use std::borrow::Cow;

use crate::response::Responder;
use crate::http::{Header, HeaderMap, Status, ContentType, Cookie};

/// The default size, in bytes, of a chunk for streamed responses.
pub const DEFAULT_CHUNK_SIZE: u64 = 4096;

#[derive(PartialEq, Clone, Hash)]
/// The body of a response: can be sized or streamed/chunked.
pub enum Body<T> {
    /// A fixed-size body.
    Sized(T, u64),
    /// A streamed/chunked body, akin to `Transfer-Encoding: chunked`.
    Chunked(T, u64)
}

impl<T> Body<T> {
    /// Returns a new `Body` with a mutable borrow to `self`'s inner type.
    pub fn as_mut(&mut self) -> Body<&mut T> {
        match *self {
            Body::Sized(ref mut b, n) => Body::Sized(b, n),
            Body::Chunked(ref mut b, n) => Body::Chunked(b, n)
        }
    }

    /// Consumes `self`. Passes the inner type as a parameter to `f` and
    /// constructs a new body with the size of `self` and the return value of
    /// the call to `f`.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Body<U> {
        match self {
            Body::Sized(b, n) => Body::Sized(f(b), n),
            Body::Chunked(b, n) => Body::Chunked(f(b), n)
        }
    }

    /// Consumes `self` and returns the inner body.
    pub fn into_inner(self) -> T {
        match self {
            Body::Sized(b, _) | Body::Chunked(b, _) => b
        }
    }

    /// Returns `true` if `self` is a `Body::Sized`.
    pub fn is_sized(&self) -> bool {
        match *self {
            Body::Sized(..) => true,
            Body::Chunked(..) => false,
        }
    }

    /// Returns `true` if `self` is a `Body::Chunked`.
    pub fn is_chunked(&self) -> bool {
        match *self {
            Body::Chunked(..) => true,
            Body::Sized(..) => false,
        }
    }
}

impl<T: io::Read> Body<T> {
    /// Attempts to read `self` into a `Vec` and returns it. If reading fails,
    /// returns `None`.
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        let mut vec = Vec::new();
        let mut body = self.into_inner();
        if let Err(e) = body.read_to_end(&mut vec) {
            error_!("Error reading body: {:?}", e);
            return None;
        }

        Some(vec)
    }

    /// Attempts to read `self` into a `String` and returns it. If reading or
    /// conversion fails, returns `None`.
    pub fn into_string(self) -> Option<String> {
        self.into_bytes()
            .and_then(|bytes| match String::from_utf8(bytes) {
                Ok(string) => Some(string),
                Err(e) => {
                    error_!("Body is invalid UTF-8: {}", e);
                    None
                }
            })
    }
}

impl<T> fmt::Debug for Body<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Body::Sized(_, n) => writeln!(f, "Sized Body [{} bytes]", n),
            Body::Chunked(_, n) => writeln!(f, "Chunked Body [{} bytes]", n),
        }
    }
}

/// Type for easily building `Response`s.
///
/// Building a [`Response`] can be a low-level ordeal; this structure presents a
/// higher-level API that simplifies building `Response`s.
///
/// # Usage
///
/// `ResponseBuilder` follows the builder pattern and is usually obtained by
/// calling [`Response::build()`] on `Response`. Almost all methods take the
/// current builder as a mutable reference and return the same mutable reference
/// with field(s) modified in the `Responder` being built. These method calls
/// can be chained: `build.a().b()`.
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
/// # #[allow(unused_variables)]
/// let response = Response::build()
///     .status(Status::ImATeapot)
///     .header(ContentType::Plain)
///     .raw_header("X-Teapot-Make", "Rocket")
///     .raw_header("X-Teapot-Model", "Utopia")
///     .raw_header_adjoin("X-Teapot-Model", "Series 1")
///     .sized_body(Cursor::new("Brewing the best coffee!"))
///     .finalize();
/// ```
pub struct ResponseBuilder<'r> {
    response: Response<'r>
}

impl<'r> ResponseBuilder<'r> {
    /// Creates a new `ResponseBuilder` that will build on top of the `base`
    /// `Response`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::response::{ResponseBuilder, Response};
    ///
    /// # #[allow(unused_variables)]
    /// let builder = ResponseBuilder::new(Response::new());
    /// ```
    #[inline(always)]
    pub fn new(base: Response<'r>) -> ResponseBuilder<'r> {
        ResponseBuilder {
            response: base
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
    /// # #[allow(unused_variables)]
    /// let response = Response::build()
    ///     .status(Status::NotFound)
    ///     .finalize();
    /// ```
    #[inline(always)]
    pub fn status(&mut self, status: Status) -> &mut ResponseBuilder<'r> {
        self.response.set_status(status);
        self
    }

    /// Sets the status of the `Response` being built to a custom status
    /// constructed from the `code` and `reason` phrase.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    ///
    /// # #[allow(unused_variables)]
    /// let response = Response::build()
    ///     .raw_status(699, "Alien Encounter")
    ///     .finalize();
    /// ```
    #[inline(always)]
    pub fn raw_status(&mut self, code: u16, reason: &'static str)
            -> &mut ResponseBuilder<'r> {
        self.response.set_raw_status(code, reason);
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
    pub fn header<'h: 'r, H>(&mut self, header: H) -> &mut ResponseBuilder<'r>
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
    /// use rocket::http::hyper::header::Accept;
    ///
    /// let response = Response::build()
    ///     .header_adjoin(Accept::json())
    ///     .header_adjoin(Accept::text())
    ///     .finalize();
    ///
    /// assert_eq!(response.headers().get("Accept").count(), 2);
    /// ```
    #[inline(always)]
    pub fn header_adjoin<'h: 'r, H>(&mut self, header: H) -> &mut ResponseBuilder<'r>
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
    pub fn raw_header<'a: 'r, 'b: 'r, N, V>(&mut self, name: N, value: V)
            -> &mut ResponseBuilder<'r>
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>
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
    pub fn raw_header_adjoin<'a: 'r, 'b: 'r, N, V>(&mut self, name: N, value: V)
            -> &mut ResponseBuilder<'r>
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>
    {
        self.response.adjoin_raw_header(name, value);
        self
    }

    /// Sets the body of the `Response` to be the fixed-sized `body`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # #[allow(dead_code)]
    /// # fn test() -> io::Result<()> {
    /// # #[allow(unused_variables)]
    /// let response = Response::build()
    ///     .sized_body(File::open("body.txt")?)
    ///     .finalize();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn sized_body<B>(&mut self, body: B) -> &mut ResponseBuilder<'r>
        where B: io::Read + io::Seek + 'r
    {
        self.response.set_sized_body(body);
        self
    }

    /// Sets the body of the `Response` to be the streamed `body`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # #[allow(dead_code)]
    /// # fn test() -> io::Result<()> {
    /// # #[allow(unused_variables)]
    /// let response = Response::build()
    ///     .streamed_body(File::open("body.txt")?)
    ///     .finalize();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn streamed_body<B>(&mut self, body: B) -> &mut ResponseBuilder<'r>
        where B: io::Read + 'r
    {
        self.response.set_streamed_body(body);
        self
    }

    /// Sets the body of the `Response` to be the streamed `body` with a custom
    /// chunk size, in bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # #[allow(dead_code)]
    /// # fn test() -> io::Result<()> {
    /// # #[allow(unused_variables)]
    /// let response = Response::build()
    ///     .chunked_body(File::open("body.txt")?, 8096)
    ///     .finalize();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn chunked_body<B: io::Read + 'r>(&mut self, body: B, chunk_size: u64)
            -> &mut ResponseBuilder<'r>
    {
        self.response.set_chunked_body(body, chunk_size);
        self
    }

    /// Sets the body of `self` to be `body`. This method should typically not
    /// be used, opting instead for one of `sized_body`, `streamed_body`, or
    /// `chunked_body`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::response::{Response, Body};
    ///
    /// # #[allow(unused_variables)]
    /// let response = Response::build()
    ///     .raw_body(Body::Sized(Cursor::new("Hello!"), 6))
    ///     .finalize();
    /// ```
    #[inline(always)]
    pub fn raw_body<T: io::Read + 'r>(&mut self, body: Body<T>)
            -> &mut ResponseBuilder<'r>
    {
        self.response.set_raw_body(body);
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
    /// # {
    /// let ctype: Vec<_> = response.headers().get("Content-Type").collect();
    /// assert_eq!(ctype, vec![ContentType::HTML.to_string()]);
    /// # }
    ///
    /// # {
    /// let custom_values: Vec<_> = response.headers().get("X-Custom").collect();
    /// assert_eq!(custom_values, vec!["value 1"]);
    /// # }
    /// ```
    #[inline(always)]
    pub fn merge(&mut self, other: Response<'r>) -> &mut ResponseBuilder<'r> {
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
    /// # {
    /// let ctype: Vec<_> = response.headers().get("Content-Type").collect();
    /// assert_eq!(ctype, vec![ContentType::HTML.to_string()]);
    /// # }
    ///
    /// # {
    /// let custom_values: Vec<_> = response.headers().get("X-Custom").collect();
    /// assert_eq!(custom_values, vec!["value 2", "value 3", "value 1"]);
    /// # }
    /// ```
    #[inline(always)]
    pub fn join(&mut self, other: Response<'r>) -> &mut ResponseBuilder<'r> {
        self.response.join(other);
        self
    }

    /// Retrieve the built `Response`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    ///
    /// # #[allow(unused_variables)]
    /// let response = Response::build()
    ///     // build the response
    ///     .finalize();
    /// ```
    #[inline(always)]
    pub fn finalize(&mut self) -> Response<'r> {
        std::mem::replace(&mut self.response, Response::new())
    }

    /// Retrieve the built `Response` wrapped in `Ok`.
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
    pub fn ok<T>(&mut self) -> Result<Response<'r>, T> {
        Ok(self.finalize())
    }
}

/// A response, as returned by types implementing [`Responder`].
#[derive(Default)]
pub struct Response<'r> {
    status: Option<Status>,
    headers: HeaderMap<'r>,
    body: Option<Body<Box<dyn io::Read + 'r>>>,
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
        Response {
            status: None,
            headers: HeaderMap::new(),
            body: None,
        }
    }

    /// Returns a `ResponseBuilder` with a base of `Response::new()`.
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
    pub fn build() -> ResponseBuilder<'r> {
        Response::build_from(Response::new())
    }

    /// Returns a `ResponseBuilder` with a base of `other`.
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
    pub fn build_from(other: Response<'r>) -> ResponseBuilder<'r> {
        ResponseBuilder::new(other)
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

    /// Sets the status of `self` to a custom `status` with status code `code`
    /// and reason phrase `reason`. This method should be used sparingly; prefer
    /// to use [set_status](#method.set_status) instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Response;
    /// use rocket::http::Status;
    ///
    /// let mut response = Response::new();
    /// response.set_raw_status(699, "Tripped a Wire");
    /// assert_eq!(response.status(), Status::new(699, "Tripped a Wire"));
    /// ```
    #[inline(always)]
    pub fn set_raw_status(&mut self, code: u16, reason: &'static str) {
        self.status = Some(Status::new(code, reason));
    }

    /// Returns a vector of the cookies set in `self` as identified by the
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
    /// assert_eq!(response.cookies(), vec![Cookie::new("hello", "world!")]);
    /// ```
    pub fn cookies(&self) -> Vec<Cookie<'_>> {
        let mut cookies = vec![];
        for header in self.headers().get("Set-Cookie") {
            if let Ok(cookie) = Cookie::parse_encoded(header) {
                cookies.push(cookie);
            }
        }

        cookies
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
    /// use rocket::http::hyper::header::Accept;
    ///
    /// let mut response = Response::new();
    /// response.adjoin_header(Accept::json());
    /// response.adjoin_header(Accept::text());
    ///
    /// let mut accept_headers = response.headers().iter();
    /// assert_eq!(accept_headers.next(), Some(Accept::json().into()));
    /// assert_eq!(accept_headers.next(), Some(Accept::text().into()));
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

    /// Returns a mutable borrow of the body of `self`, if there is one. The
    /// body is borrowed mutably to allow for reading.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// let mut response = Response::new();
    /// assert!(response.body().is_none());
    ///
    /// response.set_sized_body(Cursor::new("Hello, world!"));
    /// assert_eq!(response.body_string(), Some("Hello, world!".to_string()));
    /// ```
    #[inline(always)]
    pub fn body(&mut self) -> Option<Body<&mut dyn io::Read>> {
        // Looks crazy, right? Needed so Rust infers lifetime correctly. Weird.
        match self.body.as_mut() {
            Some(body) => Some(match body.as_mut() {
                Body::Sized(b, size) => Body::Sized(b, size),
                Body::Chunked(b, chunk_size) => Body::Chunked(b, chunk_size),
            }),
            None => None
        }
    }

    /// Consumes `self's` body and reads it into a string. If `self` doesn't
    /// have a body, reading fails, or string conversion (for non-UTF-8 bodies)
    /// fails, returns `None`. Note that `self`'s `body` is consumed after a
    /// call to this method.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// let mut response = Response::new();
    /// assert!(response.body().is_none());
    ///
    /// response.set_sized_body(Cursor::new("Hello, world!"));
    /// assert_eq!(response.body_string(), Some("Hello, world!".to_string()));
    /// assert!(response.body().is_none());
    /// ```
    #[inline(always)]
    pub fn body_string(&mut self) -> Option<String> {
        self.take_body().and_then(Body::into_string)
    }

    /// Consumes `self's` body and reads it into a `Vec` of `u8` bytes. If
    /// `self` doesn't have a body or reading fails returns `None`. Note that
    /// `self`'s `body` is consumed after a call to this method.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// let mut response = Response::new();
    /// assert!(response.body().is_none());
    ///
    /// response.set_sized_body(Cursor::new("hi!"));
    /// assert_eq!(response.body_bytes(), Some(vec![0x68, 0x69, 0x21]));
    /// assert!(response.body().is_none());
    /// ```
    #[inline(always)]
    pub fn body_bytes(&mut self) -> Option<Vec<u8>> {
        self.take_body().and_then(Body::into_bytes)
    }

    /// Moves the body of `self` out and returns it, if there is one, leaving no
    /// body in its place.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// let mut response = Response::new();
    /// assert!(response.body().is_none());
    ///
    /// response.set_sized_body(Cursor::new("Hello, world!"));
    /// assert!(response.body().is_some());
    ///
    /// let body = response.take_body();
    /// let body_string = body.and_then(|b| b.into_string());
    /// assert_eq!(body_string, Some("Hello, world!".to_string()));
    /// assert!(response.body().is_none());
    /// ```
    #[inline(always)]
    pub fn take_body(&mut self) -> Option<Body<Box<dyn io::Read + 'r>>> {
        self.body.take()
    }

    // Makes the `Read`er in the body empty but leaves the size of the body if
    // it exists. Only meant to be used to handle HEAD requests automatically.
    #[inline(always)]
    crate fn strip_body(&mut self) {
        if let Some(body) = self.take_body() {
            self.body = match body {
                Body::Sized(_, n) => Some(Body::Sized(Box::new(io::empty()), n)),
                Body::Chunked(..) => None
            };
        }
    }

    /// Sets the body of `self` to be the fixed-sized `body`. The size of the
    /// body is obtained by `seek`ing to the end and then `seek`ing back to the
    /// start.
    ///
    /// # Panics
    ///
    /// If either seek fails, this method panics. If you believe it is possible
    /// for `seek` to panic for `B`, use [set_raw_body](#method.set_raw_body)
    /// instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::Response;
    ///
    /// let mut response = Response::new();
    /// response.set_sized_body(Cursor::new("Hello, world!"));
    /// assert_eq!(response.body_string(), Some("Hello, world!".to_string()));
    /// ```
    #[inline]
    pub fn set_sized_body<B>(&mut self, mut body: B)
        where B: io::Read + io::Seek + 'r
    {
        let size = body.seek(io::SeekFrom::End(0))
            .expect("Attempted to retrieve size by seeking, but failed.");
        body.seek(io::SeekFrom::Start(0))
            .expect("Attempted to reset body by seeking after getting size.");
        self.body = Some(Body::Sized(Box::new(body.take(size)), size));
    }

    /// Sets the body of `self` to be `body`, which will be streamed. The chunk
    /// size of the stream is
    /// [DEFAULT_CHUNK_SIZE](crate::response::DEFAULT_CHUNK_SIZE). Use
    /// [set_chunked_body](#method.set_chunked_body) for custom chunk sizes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::{Read, repeat};
    /// use rocket::Response;
    ///
    /// let mut response = Response::new();
    /// response.set_streamed_body(repeat(97).take(5));
    /// assert_eq!(response.body_string(), Some("aaaaa".to_string()));
    /// ```
    #[inline(always)]
    pub fn set_streamed_body<B>(&mut self, body: B) where B: io::Read + 'r {
        self.set_chunked_body(body, DEFAULT_CHUNK_SIZE);
    }

    /// Sets the body of `self` to be `body`, which will be streamed with chunk
    /// size `chunk_size`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::{Read, repeat};
    /// use rocket::Response;
    ///
    /// let mut response = Response::new();
    /// response.set_chunked_body(repeat(97).take(5), 10);
    /// assert_eq!(response.body_string(), Some("aaaaa".to_string()));
    /// ```
    #[inline(always)]
    pub fn set_chunked_body<B>(&mut self, body: B, chunk_size: u64)
            where B: io::Read + 'r {
        self.body = Some(Body::Chunked(Box::new(body), chunk_size));
    }

    /// Sets the body of `self` to be `body`. This method should typically not
    /// be used, opting instead for one of `set_sized_body`,
    /// `set_streamed_body`, or `set_chunked_body`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use rocket::response::{Response, Body};
    ///
    /// let body = Body::Sized(Cursor::new("Hello!"), 6);
    ///
    /// let mut response = Response::new();
    /// response.set_raw_body(body);
    ///
    /// assert_eq!(response.body_string(), Some("Hello!".to_string()));
    /// ```
    #[inline(always)]
    pub fn set_raw_body<T: io::Read + 'r>(&mut self, body: Body<T>) {
        self.body = Some(match body {
            Body::Sized(b, n) => Body::Sized(Box::new(b.take(n)), n),
            Body::Chunked(b, n) => Body::Chunked(Box::new(b), n),
        });
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
    /// # {
    /// let ctype: Vec<_> = response.headers().get("Content-Type").collect();
    /// assert_eq!(ctype, vec![ContentType::HTML.to_string()]);
    /// # }
    ///
    /// # {
    /// let custom_values: Vec<_> = response.headers().get("X-Custom").collect();
    /// assert_eq!(custom_values, vec!["value 1"]);
    /// # }
    /// ```
    pub fn merge(&mut self, other: Response<'r>) {
        if let Some(status) = other.status {
            self.status = Some(status);
        }

        if let Some(body) = other.body {
            self.body = Some(body);
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
    /// # {
    /// let ctype: Vec<_> = response.headers().get("Content-Type").collect();
    /// assert_eq!(ctype, vec![ContentType::HTML.to_string()]);
    /// # }
    ///
    /// # {
    /// let custom_values: Vec<_> = response.headers().get("X-Custom").collect();
    /// assert_eq!(custom_values, vec!["value 2", "value 3", "value 1"]);
    /// # }
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

        match self.body {
            Some(ref body) => writeln!(f, "{:?}", body),
            None => writeln!(f, "Empty Body")
        }
    }
}

use crate::request::Request;

impl<'r> Responder<'r> for Response<'r> {
    /// This is the identity implementation. It simply returns `Ok(self)`.
    fn respond_to(self, _: &Request<'_>) -> Result<Response<'r>, Status> {
        Ok(self)
    }
}
