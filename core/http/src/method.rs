use std::fmt;
use std::str::FromStr;

use {hyper, uncased::uncased_eq};

use self::Method::*;

// TODO: Support non-standard methods, here and in codegen.

/// Representation of HTTP methods.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Method {
    Get,
    Put,
    Post,
    Delete,
    Options,
    Head,
    Trace,
    Connect,
    Patch
}

impl Method {
    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    pub fn from_hyp(method: &hyper::Method) -> Option<Method> {
        match *method {
            hyper::Method::Get => Some(Get),
            hyper::Method::Put => Some(Put),
            hyper::Method::Post => Some(Post),
            hyper::Method::Delete => Some(Delete),
            hyper::Method::Options => Some(Options),
            hyper::Method::Head => Some(Head),
            hyper::Method::Trace => Some(Trace),
            hyper::Method::Connect => Some(Connect),
            hyper::Method::Patch => Some(Patch),
            hyper::Method::Extension(_) => None,
        }
    }

    /// Returns `true` if an HTTP request with the method represented by `self`
    /// always supports a payload.
    ///
    /// The following methods always support payloads:
    ///
    ///   * `PUT`, `POST`, `DELETE`, `PATCH`
    ///
    /// The following methods _do not_ always support payloads:
    ///
    ///   * `GET`, `HEAD`, `CONNECT`, `TRACE`, `OPTIONS`
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Method;
    ///
    /// assert_eq!(Method::Get.supports_payload(), false);
    /// assert_eq!(Method::Post.supports_payload(), true);
    /// ```
    #[inline]
    pub fn supports_payload(self) -> bool {
        match self {
            Put | Post | Delete | Patch => true,
            Get | Head | Connect | Trace | Options => false,
        }
    }

    /// Returns the string representation of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Method;
    ///
    /// assert_eq!(Method::Get.as_str(), "GET");
    /// ```
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            Get => "GET",
            Put => "PUT",
            Post => "POST",
            Delete => "DELETE",
            Options => "OPTIONS",
            Head => "HEAD",
            Trace => "TRACE",
            Connect => "CONNECT",
            Patch => "PATCH",
        }
    }
}

impl FromStr for Method {
    type Err = ();

    // According to the RFC, method names are case-sensitive. But some old
    // clients don't follow this, so we just do a case-insensitive match here.
    fn from_str(s: &str) -> Result<Method, ()> {
        match s {
            x if uncased_eq(x, Get.as_str()) => Ok(Get),
            x if uncased_eq(x, Put.as_str()) => Ok(Put),
            x if uncased_eq(x, Post.as_str()) => Ok(Post),
            x if uncased_eq(x, Delete.as_str()) => Ok(Delete),
            x if uncased_eq(x, Options.as_str()) => Ok(Options),
            x if uncased_eq(x, Head.as_str()) => Ok(Head),
            x if uncased_eq(x, Trace.as_str()) => Ok(Trace),
            x if uncased_eq(x, Connect.as_str()) => Ok(Connect),
            x if uncased_eq(x, Patch.as_str()) => Ok(Patch),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Method {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
