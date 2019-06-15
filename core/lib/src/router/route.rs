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
    /// The uri (in Rocket's route format) that should be matched against. This
    /// URI already includes the base mount point.
    pub uri: Origin<'static>,
    /// The rank of this route. Lower ranks have higher priorities.
    pub rank: isize,
    /// The media type this route matches against, if any.
    pub format: Option<MediaType>,
    /// Cached metadata that aids in routing later.
    crate metadata: Metadata
}

#[derive(Debug, Default, Clone)]
crate struct Metadata {
    crate path_segments: Vec<RouteSegment<'static, Path>>,
    crate query_segments: Option<Vec<RouteSegment<'static, Query>>>,
    crate fully_dynamic_query: bool,
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
    /// parameters) are ranked higher than routes with dynamic paths, routes
    /// with query strings with static segments are ranked higher than routes
    /// with fully dynamic queries, and routes with queries are ranked higher
    /// than routes without queries. This default ranking is summarized by the
    /// table below:
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
    /// # use rocket::handler::Outcome;
    /// # fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    /// #     Outcome::from(request, "Hello, world!")
    /// # }
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
        where S: AsRef<str>, H: Handler + 'static
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
    /// # use rocket::handler::Outcome;
    /// # fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    /// #     Outcome::from(request, "Hello, world!")
    /// # }
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
        let uri = Origin::parse_route(path)
            .unwrap_or_else(|e| panic(path, e))
            .to_normalized()
            .into_owned();

        let mut route = Route {
            name: None,
            format: None,
            base: Origin::dummy(),
            handler: Box::new(handler),
            metadata: Metadata::default(),
            method, rank, uri
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
    /// # use rocket::{Request, Data};
    /// # use rocket::handler::Outcome;
    /// #
    /// # fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    /// #     Outcome::from(request, "Hello, world!")
    /// # }
    ///
    /// let mut index = Route::new(Method::Get, "/", handler);
    /// assert_eq!(index.base(), "/");
    /// assert_eq!(index.base.path(), "/");
    /// ```
    #[inline]
    pub fn base(&self) -> &str {
        self.base.path()
    }

    /// Sets the base mount point of the route to `base` and sets the path to
    /// `path`. The `path` should _not_ contains the `base` mount point. If
    /// `base` contains a query, it is ignored. Note that `self.uri` will
    /// include the new `base` after this method is called.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following occur:
    ///
    ///   * The base mount point contains dynamic parameters.
    ///   * The base mount point or path contain encoded characters.
    ///   * The path is not a valid Rocket route URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::{Method, uri::Origin};
    /// # use rocket::{Request, Data};
    /// # use rocket::handler::Outcome;
    /// #
    /// # fn handler<'r>(request: &'r Request, _data: Data) -> Outcome<'r> {
    /// #     Outcome::from(request, "Hello, world!")
    /// # }
    ///
    /// let mut index = Route::new(Method::Get, "/", handler);
    /// assert_eq!(index.base(), "/");
    /// assert_eq!(index.base.path(), "/");
    ///
    /// let new_base = Origin::parse("/greeting").unwrap();
    /// let new_uri = Origin::parse("/hi").unwrap();
    /// index.set_uri(new_base, new_uri);
    /// assert_eq!(index.base(), "/greeting");
    /// assert_eq!(index.uri.path(), "/greeting/hi");
    /// ```
    pub fn set_uri<'a>(
        &mut self,
        mut base: Origin<'a>,
        path: Origin<'a>
    ) -> Result<(), RouteUriError> {
        base.clear_query();
        for segment in <RouteSegment<'_, Path>>::parse(&base) {
            if segment?.kind != Kind::Static {
                return Err(RouteUriError::DynamicBase);
            }
        }

        let complete_uri = format!("{}/{}", base, path);
        let uri = Origin::parse_route(&complete_uri)?;
        self.base = base.to_normalized().into_owned();
        self.uri = uri.to_normalized().into_owned();
        self.update_metadata()?;

        Ok(())
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", Paint::green(&self.method), Paint::blue(&self.uri))?;

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
impl From<&StaticRouteInfo> for Route {
    fn from(info: &StaticRouteInfo) -> Route {
        // This should never panic since `info.path` is statically checked.
        let mut route = Route::new(info.method, info.path, info.handler);
        route.format = info.format.clone();
        route.name = Some(info.name);
        if let Some(rank) = info.rank {
            route.rank = rank;
        }

        route
    }
}
