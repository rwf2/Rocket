use std::fmt::{self, Display};
use std::convert::From;

use yansi::Paint;

use crate::codegen::StaticRouteInfo;
use crate::handler::Handler;
use crate::http::{Method, MediaType};
use crate::http::route::{RouteSegment, Kind};
use crate::error::RouteUriError;
use crate::http::ext::IntoOwned;
use crate::http::uri::{Origin, Path, Query};

/// A route: a method, its handler, path, rank, and format/media type.
#[derive(Clone)]
pub struct Route {
    /// The name of this route, if one was given.
    pub name: Option<&'static str>,
    /// The method this route matches against.
    pub method: Method,
    /// The function that should be called when the route matches.
    pub handler: Box<dyn Handler>,
    /// The base mount point of this `Route`.
    pub base: Origin<'static>,
    /// The path of this `Route` in Rocket's route format.
    pub(crate) path: Origin<'static>,
    /// The complete URI (in Rocket's route format) that should be matched
    /// against. This is `base` + `path`.
    pub uri: Origin<'static>,
    /// The rank of this route. Lower ranks have higher priorities.
    pub rank: isize,
    /// The media type this route matches against, if any.
    pub format: Option<MediaType>,
    /// Cached metadata that aids in routing later.
    pub(crate) metadata: Metadata,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Metadata {
    pub path_segments: Vec<RouteSegment<'static, Path>>,
    pub query_segments: Option<Vec<RouteSegment<'static, Query>>>,
    pub fully_dynamic_query: bool,
}

impl Metadata {
    fn from(route: &Route) -> Result<Metadata, RouteUriError> {
        let path_segments = <RouteSegment<'_, Path>>::parse(&route.uri)
            .map(|res| res.map(|s| s.into_owned()))
            .collect::<Result<Vec<_>, _>>()?;

        let (query_segments, is_dyn) = match <RouteSegment<'_, Query>>::parse(&route.uri) {
            Some(results) => {
                let segments = results.map(|res| res.map(|s| s.into_owned()))
                    .collect::<Result<Vec<_>, _>>()?;

                let dynamic = !segments.iter().any(|s| s.kind == Kind::Static);

                (Some(segments), dynamic)
            }
            None => (None, true)
        };

        Ok(Metadata { path_segments, query_segments, fully_dynamic_query: is_dyn })
    }
}

#[inline(always)]
fn default_rank(route: &Route) -> isize {
    let static_path = route.metadata.path_segments.iter().all(|s| s.kind == Kind::Static);
    let partly_static_query = route.uri.query().map(|_| !route.metadata.fully_dynamic_query);
    match (static_path, partly_static_query) {
        (true, Some(true)) => -6,   // static path, partly static query
        (true, Some(false)) => -5,  // static path, fully dynamic query
        (true, None) => -4,         // static path, no query
        (false, Some(true)) => -3,  // dynamic path, partly static query
        (false, Some(false)) => -2, // dynamic path, fully dynamic query
        (false, None) => -1,        // dynamic path, no query
    }
}

fn panic<U: Display, E: Display, T>(uri: U, e: E) -> T {
    panic!("invalid URI '{}' used to construct route: {}", uri, e)
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
    pub fn new<S, H>(method: Method, path: S, handler: H) -> Route
        where S: AsRef<str>, H: Handler
    {
        let mut route = Route::ranked(0, method, path, handler);
        route.rank = default_rank(&route);
        route
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
    pub fn ranked<S, H>(rank: isize, method: Method, path: S, handler: H) -> Route
        where S: AsRef<str>, H: Handler + 'static
    {
        let path = path.as_ref();
        let route_path = Origin::parse_route(path)
            .unwrap_or_else(|e| panic(path, e))
            .into_normalized()
            .into_owned();

        let mut route = Route {
            path: route_path.clone(),
            uri: route_path,
            name: None,
            format: None,
            base: Origin::dummy(),
            handler: Box::new(handler),
            metadata: Metadata::default(),
            method, rank,
        };

        route.update_metadata().unwrap_or_else(|e| panic(path, e));
        route
    }

    /// Updates the cached routing metadata. MUST be called whenver the route's
    /// URI is set or changes.
    fn update_metadata(&mut self) -> Result<(), RouteUriError> {
        let new_metadata = Metadata::from(&*self)?;
        self.metadata = new_metadata;
        Ok(())
    }

    /// Retrieves the path of the base mount point of this route as an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::handler::dummy as handler;
    ///
    /// let mut index = Route::new(Method::Get, "/", handler);
    /// assert_eq!(index.base(), "/");
    /// assert_eq!(index.base.path(), "/");
    /// ```
    #[inline]
    pub fn base(&self) -> &str {
        self.base.path()
    }

    /// Retrieves this route's path.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::handler::dummy as handler;
    ///
    /// let index = Route::new(Method::Get, "/foo/bar?a=1", handler);
    /// let index = index.map_base(|base| format!("{}{}", "/boo", base)).unwrap();
    /// assert_eq!(index.uri.path(), "/boo/foo/bar");
    /// assert_eq!(index.uri.query(), Some("a=1"));
    /// assert_eq!(index.base(), "/boo");
    /// assert_eq!(index.path().path(), "/foo/bar");
    /// assert_eq!(index.path().query(), Some("a=1"));
    /// ```
    #[inline]
    pub fn path(&self) -> &Origin<'_> {
        &self.path
    }

    /// Maps the `base` of this route using `mapper`, returning a new `Route`
    /// with the returned base.
    ///
    /// `mapper` is called with the current base. The returned `String` is used
    /// as the new base if it is a valid URI. If the returned base URI contains
    /// a query, it is ignored. Returns an if the base produced by `mapper` is
    /// not a valid origin URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::{Method, uri::Origin};
    /// # use rocket::handler::{dummy as handler, Outcome, HandlerFuture};
    ///
    /// let index = Route::new(Method::Get, "/foo/bar", handler);
    /// assert_eq!(index.base(), "/");
    /// assert_eq!(index.path().path(), "/foo/bar");
    /// assert_eq!(index.uri.path(), "/foo/bar");
    ///
    /// let index = index.map_base(|base| format!("{}{}", "/boo", base)).unwrap();
    /// assert_eq!(index.base(), "/boo");
    /// assert_eq!(index.path().path(), "/foo/bar");
    /// assert_eq!(index.uri.path(), "/boo/foo/bar");
    /// ```
    pub fn map_base<'a, F>(mut self, mapper: F) -> Result<Self, RouteUriError>
        where F: FnOnce(Origin<'static>) -> String
    {
        self.base = Origin::parse_owned(mapper(self.base))?.into_normalized();
        self.base.clear_query();

        let new_uri = format!("{}{}", self.base, self.path);
        self.uri = Origin::parse_route(&new_uri)?.into_owned().into_normalized();
        self.update_metadata()?;
        Ok(self)
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", Paint::green(&self.method))?;
        if self.base.path() != "/" {
            write!(f, "{}", Paint::blue(&self.base).underline())?;
        }

        write!(f, "{}", Paint::blue(&self.path))?;

        if self.rank > 1 {
            write!(f, " [{}]", Paint::default(&self.rank).bold())?;
        }

        if let Some(ref format) = self.format {
            write!(f, " {}", Paint::yellow(format))?;
        }

        if let Some(name) = self.name {
            write!(f, " {}{}{}",
                   Paint::cyan("("), Paint::magenta(name), Paint::cyan(")"))?;
        }

        Ok(())
    }
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Route")
            .field("name", &self.name)
            .field("method", &self.method)
            .field("base", &self.base)
            .field("uri", &self.uri)
            .field("rank", &self.rank)
            .field("format", &self.format)
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[doc(hidden)]
impl From<StaticRouteInfo> for Route {
    fn from(info: StaticRouteInfo) -> Route {
        // This should never panic since `info.path` is statically checked.
        let mut route = Route::new(info.method, info.path, info.handler);
        route.format = info.format;
        route.name = Some(info.name);
        if let Some(rank) = info.rank {
            route.rank = rank;
        }

        route
    }
}
