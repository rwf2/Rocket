use std::fmt;
use std::convert::From;

use yansi::Color::*;

use codegen::StaticRouteInfo;
use handler::Handler;
use http::{Method, MediaType};
use http::uri::URI;

/// A route: a method, its handler, path, rank, and format/media type.
pub struct Route {
    /// The name of this route, if one was given.
    pub name: Option<&'static str>,
    /// The method this route matches against.
    pub method: Method,
    /// The function that should be called when the route matches.
    pub handler: Handler,
    /// The base mount point of this `Route`.
    pub base: URI<'static>,
    /// The uri (in Rocket format) that should be matched against. This uri
    /// already includes the base mount point.
    pub uri: URI<'static>,
    /// The rank of this route. Lower ranks have higher priorities.
    pub rank: isize,
    /// The media type this route matches against, if any.
    pub format: Option<MediaType>,
}

#[inline(always)]
fn default_rank(uri: &URI) -> isize {
    // static path, query = -4; static path, no query = -3
    // dynamic path, query = -2; dynamic path, no query = -1
    match (!uri.path().contains('<'),  uri.query().is_some()) {
        (true, true) => -4,
        (true, false) => -3,
        (false, true) => -2,
        (false, false) => -1,
    }
}

impl Route {
    /// Creates a new route with the given method, path, and handler with a base
    /// of `/`.
    ///
    /// # Ranking
    ///
    /// The route rank's is set so that routes with static paths are ranked
    /// higher than route's with dynamic paths, and routes with query strings
    /// are ranked higher than ranks without query strings. This default ranking
    /// is summarized by the table below:
    ///
    /// | static path | query | rank |
    /// |-------------|-------|------|
    /// | yes         | yes   | -4   |
    /// | yes         | no    | -3   |
    /// | no          | yes   | -2   |
    /// | no          | no    | -1   |
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Request, Route, Data};
    /// use rocket::handler::Outcome;
    /// use rocket::http::Method;
    ///
    /// fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    ///     Outcome::from(request, "Hello, world!")
    /// }
    ///
    /// // this is a rank -3 route matching requests to `GET /`
    /// let index = Route::new(Method::Get, "/", handler);
    ///
    /// // this is a rank -4 route matching requests to `GET /?<name>`
    /// let index_name = Route::new(Method::Get, "/?<name>", handler);
    ///
    /// // this is a rank -1 route matching requests to `GET /<name>`
    /// let name = Route::new(Method::Get, "/<name>", handler);
    /// ```
    pub fn new<S>(m: Method, path: S, handler: Handler) -> Route
        where S: AsRef<str>
    {
        let uri = URI::from(path.as_ref().to_string());
        Route {
            name: None,
            method: m,
            handler: handler,
            rank: default_rank(&uri),
            base: URI::from("/"),
            uri: uri,
            format: None,
        }
    }

    /// Creates a new route with the given rank, method, path, and handler with
    /// a base of `/`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Request, Route, Data};
    /// use rocket::handler::Outcome;
    /// use rocket::http::Method;
    ///
    /// fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    ///     Outcome::from(request, "Hello, world!")
    /// }
    ///
    /// // this is a rank 1 route matching requests to `GET /`
    /// let index = Route::ranked(1, Method::Get, "/", handler);
    /// ```
    pub fn ranked<S>(rank: isize, m: Method, uri: S, handler: Handler) -> Route
        where S: AsRef<str>
    {
        Route {
            name: None,
            method: m,
            handler: handler,
            base: URI::from("/"),
            uri: URI::from(uri.as_ref().to_string()),
            rank: rank,
            format: None,
        }
    }

    /// Retrieves the path of the base mount point of this route as an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Request, Route, Data};
    /// use rocket::handler::Outcome;
    /// use rocket::http::Method;
    ///
    /// fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    ///     Outcome::from(request, "Hello, world!")
    /// }
    ///
    /// let mut index = Route::ranked(1, Method::Get, "/", handler);
    /// assert_eq!(index.base(), "/");
    /// assert_eq!(index.base.path(), "/");
    /// ```
    #[inline]
    pub fn base(&self) -> &str {
        self.base.path()
    }

    /// Sets the base mount point of the route. Does not update the rank or any
    /// other parameters.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Request, Route, Data};
    /// use rocket::handler::Outcome;
    /// use rocket::http::Method;
    ///
    /// fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    ///     Outcome::from(request, "Hello, world!")
    /// }
    ///
    /// let mut index = Route::ranked(1, Method::Get, "/", handler);
    /// assert_eq!(index.base(), "/");
    /// assert_eq!(index.base.path(), "/");
    ///
    /// index.set_base("/hi");
    /// assert_eq!(index.base(), "/hi");
    /// assert_eq!(index.base.path(), "/hi");
    /// ```
    pub fn set_base<S>(&mut self, path: S) where S: AsRef<str> {
        self.base = URI::from(path.as_ref().to_string());
    }

    /// Sets the path of the route. Does not update the rank or any other
    /// parameters.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Request, Route, Data};
    /// use rocket::handler::Outcome;
    /// use rocket::http::Method;
    ///
    /// fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    ///     Outcome::from(request, "Hello, world!")
    /// }
    ///
    /// let mut index = Route::ranked(1, Method::Get, "/", handler);
    /// assert_eq!(index.uri.path(), "/");
    ///
    /// index.set_uri("/hello");
    /// assert_eq!(index.uri.path(), "/hello");
    /// ```
    pub fn set_uri<S>(&mut self, uri: S) where S: AsRef<str> {
        self.uri = URI::from(uri.as_ref().to_string());
    }

    // FIXME: Decide whether a component has to be fully variable or not. That
    // is, whether you can have: /a<a>b/ or even /<a>:<b>/
    // TODO: Don't return a Vec...take in an &mut [&'a str] (no alloc!)
    /// Given a URI, returns a vector of slices of that URI corresponding to the
    /// dynamic segments in this route.
    pub(crate) fn get_param_indexes(&self, uri: &URI) -> Vec<(usize, usize)> {
        let route_segs = self.uri.segments();
        let uri_segs = uri.segments();
        let start_addr = uri.path().as_ptr() as usize;

        let mut result = Vec::with_capacity(self.uri.segment_count());
        for (route_seg, uri_seg) in route_segs.zip(uri_segs) {
            let i = (uri_seg.as_ptr() as usize) - start_addr;
            if route_seg.ends_with("..>") {
                result.push((i, uri.path().len()));
                break;
            } else if route_seg.ends_with('>') {
                let j = i + uri_seg.len();
                result.push((i, j));
            }
        }

        result
    }
}

impl Clone for Route {
    fn clone(&self) -> Route {
        Route {
            name: self.name,
            method: self.method,
            handler: self.handler,
            rank: self.rank,
            base: self.base.clone(),
            uri: self.uri.clone(),
            format: self.format.clone(),
        }
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", Green.paint(&self.method), Blue.paint(&self.uri))?;

        if self.rank > 1 {
            write!(f, " [{}]", White.paint(&self.rank))?;
        }

        if let Some(ref format) = self.format {
            write!(f, " {}", Yellow.paint(format))?;
        }

        if let Some(name) = self.name {
            write!(f, " {}{}{}",
                   Cyan.paint("("), Purple.paint(name), Cyan.paint(")"))?;
        }

        Ok(())
    }
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Route as fmt::Display>::fmt(self, f)
    }
}

#[doc(hidden)]
impl<'a> From<&'a StaticRouteInfo> for Route {
    fn from(info: &'a StaticRouteInfo) -> Route {
        let mut route = Route::new(info.method, info.path, info.handler);
        route.format = info.format.clone();
        route.name = Some(info.name);
        if let Some(rank) = info.rank {
            route.rank = rank;
        }

        route
    }
}
