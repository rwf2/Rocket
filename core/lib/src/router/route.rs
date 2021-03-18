use std::fmt;
use std::convert::From;
use std::borrow::Cow;

use yansi::Paint;

use crate::codegen::StaticRouteInfo;
use crate::handler::Handler;
use crate::http::{uri, Method, MediaType};
use crate::router::RouteUri;

/// A route: a method, its handler, path, rank, and format/media type.
#[derive(Clone)]
pub struct Route {
    /// The name of this route, if one was given.
    pub name: Option<Cow<'static, str>>,
    /// The method this route matches against.
    pub method: Method,
    /// The function that should be called when the route matches.
    pub handler: Box<dyn Handler>,
    /// The route URI.
    pub uri: RouteUri<'static>,
    /// The rank of this route. Lower ranks have higher priorities.
    pub rank: isize,
    /// The media type this route matches against, if any.
    pub format: Option<MediaType>,
}

impl Route {
    /// Creates a new route with the given method, path, and handler with a base
    /// of `/`.
    ///
    /// # Ranking
    ///
    /// The route's rank is set so that routes with static paths (no dynamic
    /// parameters) have lower ranks (higher precedence) than routes with
    /// dynamic paths, routes with query strings with static segments have lower
    /// ranks than routes with fully dynamic queries, and routes with queries
    /// have lower ranks than routes without queries. This default ranking is
    /// summarized by the table below:
    ///
    /// | static path | query         | rank |
    /// |-------------|---------------|------|
    /// | yes         | partly static | -6   |
    /// | yes         | fully dynamic | -5   |
    /// | yes         | none          | -4   |
    /// | no          | partly static | -3   |
    /// | no          | fully dynamic | -2   |
    /// | no          | none          | -1   |
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::{Request, Data};
    /// # use rocket::handler::{dummy as handler, Outcome, HandlerFuture};
    ///
    /// // this is rank -6 (static path, ~static query)
    /// let route = Route::new(Method::Get, "/foo?bar=baz&<zoo>", handler);
    /// assert_eq!(route.rank, -6);
    ///
    /// // this is rank -5 (static path, fully dynamic query)
    /// let route = Route::new(Method::Get, "/foo?<zoo..>", handler);
    /// assert_eq!(route.rank, -5);
    ///
    /// // this is a rank -4 route (static path, no query)
    /// let route = Route::new(Method::Get, "/", handler);
    /// assert_eq!(route.rank, -4);
    ///
    /// // this is a rank -3 route (dynamic path, ~static query)
    /// let route = Route::new(Method::Get, "/foo/<bar>?blue", handler);
    /// assert_eq!(route.rank, -3);
    ///
    /// // this is a rank -2 route (dynamic path, fully dynamic query)
    /// let route = Route::new(Method::Get, "/<bar>?<blue>", handler);
    /// assert_eq!(route.rank, -2);
    ///
    /// // this is a rank -1 route (dynamic path, no query)
    /// let route = Route::new(Method::Get, "/<bar>/foo/<baz..>", handler);
    /// assert_eq!(route.rank, -1);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `path` is not a valid origin URI or Rocket route URI.
    pub fn new<H: Handler>(method: Method, uri: &str, handler: H) -> Route {
        Route::ranked(None, method, uri, handler)
    }

    /// Creates a new route with the given rank, method, path, and handler with
    /// a base of `/`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::{Request, Data};
    /// # use rocket::handler::{dummy as handler, Outcome, HandlerFuture};
    ///
    /// // this is a rank 1 route matching requests to `GET /`
    /// let index = Route::ranked(1, Method::Get, "/", handler);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `path` is not a valid origin URI or Rocket route URI.
    pub fn ranked<H, R>(rank: R, method: Method, uri: &str, handler: H) -> Route
        where H: Handler + 'static, R: Into<Option<isize>>,
    {
        let uri = RouteUri::new("/", uri);
        let rank = rank.into().unwrap_or_else(|| uri.default_rank());
        Route {
            name: None,
            format: None,
            handler: Box::new(handler),
            rank, uri, method,
        }
    }


    /// Maps the `base` of this route using `mapper`, returning a new `Route`
    /// with the returned base.
    ///
    /// `mapper` is called with the current base. The returned `String` is used
    /// as the new base if it is a valid URI. If the returned base URI contains
    /// a query, it is ignored. Returns an error if the base produced by
    /// `mapper` is not a valid origin URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::{Method, uri::Origin};
    /// # use rocket::handler::{dummy as handler, Outcome, HandlerFuture};
    ///
    /// let index = Route::new(Method::Get, "/foo/bar", handler);
    /// assert_eq!(index.uri.base(), "/");
    /// assert_eq!(index.uri.unmounted_origin.path(), "/foo/bar");
    /// assert_eq!(index.uri.path(), "/foo/bar");
    ///
    /// let index = index.map_base(|base| format!("{}{}", "/boo", base)).unwrap();
    /// assert_eq!(index.uri.base(), "/boo");
    /// assert_eq!(index.uri.unmounted_origin.path(), "/foo/bar");
    /// assert_eq!(index.uri.path(), "/boo/foo/bar");
    /// ```
    pub fn map_base<'a, F>(mut self, mapper: F) -> Result<Self, uri::Error<'static>>
        where F: FnOnce(uri::Origin<'a>) -> String
    {
        let base = mapper(self.uri.base);
        self.uri = RouteUri::try_new(&base, &self.uri.unmounted_origin.to_string())?;
        Ok(self)
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref n) = self.name {
            write!(f, "{}{}{} ", Paint::cyan("("), Paint::white(n), Paint::cyan(")"))?;
        }

        write!(f, "{} ", Paint::green(&self.method))?;
        if self.uri.base() != "/" {
            write!(f, "{}", Paint::blue(self.uri.base()).underline())?;
        }

        write!(f, "{}", Paint::blue(&self.uri.unmounted_origin))?;

        if self.rank > 1 {
            write!(f, " [{}]", Paint::default(&self.rank).bold())?;
        }

        if let Some(ref format) = self.format {
            write!(f, " {}", Paint::yellow(format))?;
        }

        Ok(())
    }
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Route")
            .field("name", &self.name)
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("rank", &self.rank)
            .field("format", &self.format)
            .finish()
    }
}

#[doc(hidden)]
impl From<StaticRouteInfo> for Route {
    fn from(info: StaticRouteInfo) -> Route {
        // This should never panic since `info.path` is statically checked.
        let mut route = Route::new(info.method, info.path, info.handler);
        route.format = info.format;
        route.name = Some(info.name.into());
        if let Some(rank) = info.rank {
            route.rank = rank;
        }

        route
    }
}
