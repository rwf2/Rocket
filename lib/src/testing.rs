//! A tiny module for testing Rocket applications.
//!
//! # Enabling
//!
//! The `testing` module is only available when Rocket is compiled with the
//! `testing` feature flag. The suggested way to enable the `testing` module is
//! through Cargo's `[dev-dependencies]` feature which allows features (and
//! other dependencies) to be enabled exclusively when testing/benchmarking your
//! application.
//!
//! To compile Rocket with the `testing` feature for testing/benchmarking, add
//! the following to your `Cargo.toml`:
//!
//! ```toml
//! [dev-dependencies]
//! rocket = { version = "*", features = ["testing"] }
//! ```
//!
//! Then, in your testing module, `use` the testing types. This typically looks
//! as follows:
//!
//! ```rust,ignore
//! #[cfg(test)]
//! mod test {
//!   use super::rocket;
//!   use rocket::testing::MockRequest;
//!   use rocket::http::Method::*;
//! }
//! ```
//!
//! # Usage
//!
//! The testing methadology is simple:
//!
//!   1. Construct a `Rocket` instance.
//!   2. Construct a request.
//!   3. Dispatch the request using the Rocket instance.
//!   4. Inspect, validate, and verify the response.
//!
//! ## Construct a `Rocket` Instance
//!
//! Constructing a `Rocket` instance for testing is identical to constructing
//! one for launching, except you should not call the `launch` method. That is,
//! use `rocket::ignite`, then mount routes and catchers. That's it!
//!
//! ## Construct a (Mock)Request
//!
//! The [MockRequest](struct.MockRequest.html) object enables the creation of an
//! HTTP request without using any networking. A `MockRequest` object is
//! constructed using the builder pattern. For example, the following code
//! builds a request for submitting a login form with three fields:
//!
//! ```rust
//! # use rocket::http::Method::*;
//! # use rocket::testing::MockRequest;
//! # use rocket::http::ContentType;
//! let (username, password, age) = ("user", "password", 32);
//! MockRequest::new(Post, "/login")
//!     .header(ContentType::Form)
//!     .body(&format!("username={}&password={}&age={}", username, password, age));
//! ```
//!
//! ## Dispatch and Validate
//!
//! Finally, requests can be dispatched using the
//! [dispatch_with](struct.MockRequest.html#method.dispatch_with) method on the
//! contructed `MockRequest` instance. The method returns the body of the
//! response. At present, the API does not allow for headers in the response to
//! be examined.
//!
//! # Example
//!
//! The following is an example of a complete application with testing.
//!
//! ```rust
//! #![feature(plugin)]
//! #![plugin(rocket_codegen)]
//!
//! extern crate rocket;
//!
//! #[get("/")]
//! fn hello() -> &'static str {
//!     "Hello, world!"
//! }
//!
//! # fn main() {  }
//! #[cfg(test)]
//! mod test {
//!     use super::rocket;
//!     use rocket::testing::MockRequest;
//!     use rocket::http::Method::*;
//!
//!     #[test]
//!     fn test_hello_world() {
//!         let rocket = rocket::ignite().mount("/", routes![super::hello]);
//!         let mut req = MockRequest::new(Get, "/");
//!         let mut response = req.dispatch_with(&rocket);
//!
//!         // Write the body out as a string.
//!         let body_str = response.body().and_then(|b| b.into_string());
//!
//!         // Check that the body contains what we expect.
//!         assert_eq!(body_str, Some("Hello, world!".to_string()));
//!     }
//! }
//! ```

use ::{Rocket, Request, Response, Data};
use http::{Method, Header, Cookie};

use std::net::SocketAddr;

/// A type for mocking requests for testing Rocket applications.
pub struct MockRequest<'r> {
    request: Request<'r>,
    data: Data
}

impl<'r> MockRequest<'r> {
    /// Constructs a new mocked request with the given `method` and `uri`.
    #[inline]
    pub fn new<S: AsRef<str>>(method: Method, uri: S) -> Self {
        MockRequest {
            request: Request::new(method, uri.as_ref().to_string()),
            data: Data::new(vec![])
        }
    }

    /// Add a header to this request.
    ///
    /// # Examples
    ///
    /// Add the Content-Type header:
    ///
    /// ```rust
    /// use rocket::http::Method::*;
    /// use rocket::testing::MockRequest;
    /// use rocket::http::ContentType;
    ///
    /// # #[allow(unused_variables)]
    /// let req = MockRequest::new(Get, "/").header(ContentType::JSON);
    /// ```
    #[inline]
    pub fn header<H: Into<Header<'static>>>(mut self, header: H) -> Self {
        self.request.add_header(header.into());
        self
    }

    /// Adds a header to this request without consuming `self`.
    ///
    /// # Examples
    ///
    /// Add the Content-Type header:
    ///
    /// ```rust
    /// use rocket::http::Method::*;
    /// use rocket::testing::MockRequest;
    /// use rocket::http::ContentType;
    ///
    /// let mut req = MockRequest::new(Get, "/");
    /// req.add_header(ContentType::JSON);
    /// ```
    #[inline]
    pub fn add_header<H: Into<Header<'static>>>(&mut self, header: H) {
        self.request.add_header(header.into());
    }

    /// Set the remote address of this request.
    ///
    /// # Examples
    ///
    /// Set the remote address to "8.8.8.8:80":
    ///
    /// ```rust
    /// use rocket::http::Method::*;
    /// use rocket::testing::MockRequest;
    ///
    /// let address = "8.8.8.8:80".parse().unwrap();
    /// # #[allow(unused_variables)]
    /// let req = MockRequest::new(Get, "/").remote(address);
    /// ```
    #[inline]
    pub fn remote(mut self, address: SocketAddr) -> Self {
        self.request.set_remote(address);
        self
    }

    /// Add a cookie to this request.
    ///
    /// # Examples
    ///
    /// Add `user_id` cookie:
    ///
    /// ```rust
    /// use rocket::http::Method::*;
    /// use rocket::testing::MockRequest;
    /// use rocket::http::Cookie;
    ///
    /// # #[allow(unused_variables)]
    /// let req = MockRequest::new(Get, "/")
    ///     .cookie(Cookie::new("username", "sb"))
    ///     .cookie(Cookie::new("user_id", format!("{}", 12)));
    /// ```
    #[inline]
    pub fn cookie(self, cookie: Cookie<'static>) -> Self {
        self.request.cookies().add(cookie);
        self
    }

    /// Set the body (data) of the request.
    ///
    /// # Examples
    ///
    /// Set the body to be a JSON structure; also sets the Content-Type.
    ///
    /// ```rust
    /// use rocket::http::Method::*;
    /// use rocket::testing::MockRequest;
    /// use rocket::http::ContentType;
    ///
    /// # #[allow(unused_variables)]
    /// let req = MockRequest::new(Post, "/")
    ///     .header(ContentType::JSON)
    ///     .body(r#"{ "key": "value", "array": [1, 2, 3], }"#);
    /// ```
    #[inline]
    pub fn body<S: AsRef<[u8]>>(mut self, body: S) -> Self {
        self.data = Data::new(body.as_ref().into());
        self
    }

    /// Dispatch this request using a given instance of Rocket.
    ///
    /// Returns the body of the response if there was a response. The return
    /// value is `None` if any of the following occurs:
    ///
    ///   1. The returned body was not valid UTF8.
    ///   2. The application failed to respond.
    ///
    /// # Examples
    ///
    /// Dispatch to a Rocket instance with the `"Hello, world!"` example
    /// mounted:
    ///
    /// ```rust
    /// # #![feature(plugin)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
    /// #
    /// #[get("/")]
    /// fn hello() -> &'static str {
    ///     "Hello, world!"
    /// }
    ///
    /// use rocket::testing::MockRequest;
    /// use rocket::http::Method::*;
    ///
    /// # fn main() {
    /// let rocket = rocket::ignite().mount("/", routes![hello]);
    /// let mut req = MockRequest::new(Get, "/");
    /// let mut response = req.dispatch_with(&rocket);
    ///
    /// let body_str = response.body().and_then(|b| b.into_string());
    /// assert_eq!(body_str, Some("Hello, world!".to_string()));
    /// # }
    /// ```
    pub fn dispatch_with<'s>(&'s mut self, rocket: &'r Rocket) -> Response<'s> {
        let data = ::std::mem::replace(&mut self.data, Data::new(vec![]));
        rocket.dispatch(&mut self.request, data)
    }
}
