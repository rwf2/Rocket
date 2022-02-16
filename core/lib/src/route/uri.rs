use std::fmt;
use std::borrow::Cow;

use crate::http::uri::{self, Origin};
use crate::http::ext::IntoOwned;
use crate::form::ValueField;
use crate::route::Segment;

/// A route URI which is matched against requests.
///
/// A route URI is composed of two components:
///
///   * `base`
///
///     Otherwise known as the route's "mount point", the `base` is a static
///     [`Origin`] that prefixes the route URI. All route URIs have a `base`.
///     When routes are created manually with [`Route::new()`], the base
///     defaults to `/`. When mounted via [`Rocket::mount()`], the base is
///     explicitly specified as the first argument.
///
///     ```rust
///     use rocket::Route;
///     use rocket::http::Method;
///     # use rocket::route::dummy_handler as handler;
///
///     let route = Route::new(Method::Get, "/foo/<bar>", handler);
///     assert_eq!(route.uri.base(), "/");
///
///     let rocket = rocket::build().mount("/base", vec![route]);
///     let routes: Vec<_> = rocket.routes().collect();
///     assert_eq!(routes[0].uri.base(), "/base");
///     ```
///
///   * `origin`
///
///     Otherwise known as the "route URI", the `origin` is an [`Origin`] with
///     potentially dynamic (`<dyn>` or `<dyn..>`) segments. It is prefixed with
///     the `base`. This is the URI which is matched against incoming requests
///     for routing.
///
///     ```rust
///     use rocket::Route;
///     use rocket::http::Method;
///     # use rocket::route::dummy_handler as handler;
///
///     let route = Route::new(Method::Get, "/foo/<bar>", handler);
///     assert_eq!(route.uri, "/foo/<bar>");
///
///     let rocket = rocket::build().mount("/base", vec![route]);
///     let routes: Vec<_> = rocket.routes().collect();
///     assert_eq!(routes[0].uri, "/base/foo/<bar>");
///     ```
///
/// [`Rocket::mount()`]: crate::Rocket::mount()
/// [`Route::new()`]: crate::Route::new()
#[derive(Clone)]
pub struct RouteUri<'a> {
    /// The source string for this URI.
    source: Cow<'a, str>,
    /// The mount point.
    pub base: Origin<'a>,
    /// The URI _without_ the `base` mount point.
    pub unmounted_origin: Origin<'a>,
    /// The URI _with_ the base mount point. This is the canoncical route URI.
    pub origin: Origin<'a>,
    /// Cached metadata about this URI.
    pub(crate) metadata: Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Color {
    /// Fully static: no dynamic components.
    Static = 3,
    /// Partially static/dynamic: some, but not all, dynamic components.
    Partial = 2,
    /// Fully dynamic: no static components.
    Wild = 1,
}

#[derive(Debug, Clone)]
pub(crate) struct Metadata {
    /// Segments in the base.
    pub base_segs: Vec<Segment>,
    /// Segments in the path, including base.
    pub path_segs: Vec<Segment>,
    /// `(name, value)` of the query segments that are static.
    pub static_query_fields: Vec<(String, String)>,
    /// The "color" of the route path.
    pub path_color: Color,
    /// The "color" of the route query, if there is query.
    pub query_color: Option<Color>,
    /// Whether the path has a `<trailing..>` parameter.
    pub trailing_path: bool,
}

type Result<T, E = uri::Error<'static>> = std::result::Result<T, E>;

impl<'a> RouteUri<'a> {
    /// Create a new `RouteUri`.
    ///
    /// This is a fallible variant of [`RouteUri::new`] which returns an `Err`
    /// if `base` or `uri` cannot be parsed as [`Origin`]s.
    pub(crate) fn try_new(base: &str, uri: &str) -> Result<RouteUri<'static>> {
        let mut base = Origin::parse(base)
            .map_err(|e| e.into_owned())?
            .into_normalized()
            .into_owned();

        base.clear_query();

        let unmounted_origin = Origin::parse_route(uri)
            .map_err(|e| e.into_owned())?
            .into_normalized()
            .into_owned();

        let origin = Origin::parse_route(&format!("{}/{}", base, unmounted_origin))
            .map_err(|e| e.into_owned())?
            .into_normalized()
            .into_owned();

        let source = origin.to_string().into();
        let metadata = Metadata::from(&base, &origin);

        Ok(RouteUri { source, base, unmounted_origin, origin, metadata })
    }

    /// Create a new `RouteUri`.
    ///
    /// Panics if  `base` or `uri` cannot be parsed as `Origin`s.
    #[track_caller]
    pub(crate) fn new(base: &str, uri: &str) -> RouteUri<'static> {
        Self::try_new(base, uri).expect("Expected valid URIs")
    }

    /// The path of the base mount point of this route URI as an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::route::dummy_handler as handler;
    ///
    /// let index = Route::new(Method::Get, "/foo/bar?a=1", handler);
    /// assert_eq!(index.uri.base(), "/");
    /// let index = index.map_base(|base| format!("{}{}", "/boo", base)).unwrap();
    /// assert_eq!(index.uri.base(), "/boo");
    /// ```
    #[inline(always)]
    pub fn base(&self) -> &str {
        self.base.path().as_str()
    }

    /// The path part of this route URI as an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::route::dummy_handler as handler;
    ///
    /// let index = Route::new(Method::Get, "/foo/bar?a=1", handler);
    /// assert_eq!(index.uri.path(), "/foo/bar");
    /// let index = index.map_base(|base| format!("{}{}", "/boo", base)).unwrap();
    /// assert_eq!(index.uri.path(), "/boo/foo/bar");
    /// ```
    #[inline(always)]
    pub fn path(&self) -> &str {
        self.origin.path().as_str()
    }

    /// The query part of this route URI, if there is one.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::route::dummy_handler as handler;
    ///
    /// let index = Route::new(Method::Get, "/foo/bar", handler);
    /// assert!(index.uri.query().is_none());
    ///
    /// // Normalization clears the empty '?'.
    /// let index = Route::new(Method::Get, "/foo/bar?", handler);
    /// assert!(index.uri.query().is_none());
    ///
    /// let index = Route::new(Method::Get, "/foo/bar?a=1", handler);
    /// assert_eq!(index.uri.query().unwrap(), "a=1");
    ///
    /// let index = index.map_base(|base| format!("{}{}", "/boo", base)).unwrap();
    /// assert_eq!(index.uri.query().unwrap(), "a=1");
    /// ```
    #[inline(always)]
    pub fn query(&self) -> Option<&str> {
        self.origin.query().map(|q| q.as_str())
    }

    /// The full URI as an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::route::dummy_handler as handler;
    ///
    /// let index = Route::new(Method::Get, "/foo/bar?a=1", handler);
    /// assert_eq!(index.uri.as_str(), "/foo/bar?a=1");
    /// let index = index.map_base(|base| format!("{}{}", "/boo", base)).unwrap();
    /// assert_eq!(index.uri.as_str(), "/boo/foo/bar?a=1");
    /// ```
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// Get the default rank of a route with this URI.
    ///
    /// The route's default rank is determined based on the presence or absence
    /// of static and dynamic paths and queries. See the documentation for
    /// [`Route::new`][`crate::Route::new`] for a table summarizing the exact default ranks.
    ///
    /// | path    | query   | rank |
    /// |---------|---------|------|
    /// | static  | static  | -12  |
    /// | static  | partial | -11  |
    /// | static  | wild    | -10  |
    /// | static  | none    | -9   |
    /// | partial | static  | -8   |
    /// | partial | partial | -7   |
    /// | partial | wild    | -6   |
    /// | partial | none    | -5   |
    /// | wild    | static  | -4   |
    /// | wild    | partial | -3   |
    /// | wild    | wild    | -2   |
    /// | wild    | none    | -1   |
    pub(crate) fn default_rank(&self) -> isize {
        let raw_path_weight = self.metadata.path_color as u8;
        let raw_query_weight = self.metadata.query_color.map_or(0, |c| c as u8);
        let raw_weight = (raw_path_weight << 2) | raw_query_weight;

        // We subtract `3` because `raw_path` is never `0`: 0b0100 = 4 - 3 = 1.
        -((raw_weight as isize) - 3)
    }
}

impl Metadata {
    fn from(base: &Origin<'_>, origin: &Origin<'_>) -> Self {
        let base_segs = base.path().raw_segments()
            .map(Segment::from)
            .collect::<Vec<_>>();

        let path_segs = origin.path().raw_segments()
            .map(Segment::from)
            .collect::<Vec<_>>();

        let query_segs = origin.query()
            .map(|q| q.raw_segments().map(Segment::from).collect::<Vec<_>>())
            .unwrap_or_default();

        let static_query_fields = query_segs.iter().filter(|s| !s.dynamic)
            .map(|s| ValueField::parse(&s.value))
            .map(|f| (f.name.source().to_string(), f.value.to_string()))
            .collect();

        let static_path = path_segs.iter().all(|s| !s.dynamic);
        let wild_path = !path_segs.is_empty() && path_segs.iter().all(|s| s.dynamic);
        let path_color = match (static_path, wild_path) {
            (true, _) => Color::Static,
            (_, true) => Color::Wild,
            (_, _) => Color::Partial
        };

        let query_color = (!query_segs.is_empty()).then(|| {
            let static_query = query_segs.iter().all(|s| !s.dynamic);
            let wild_query = query_segs.iter().all(|s| s.dynamic);
            match (static_query, wild_query) {
                (true, _) => Color::Static,
                (_, true) => Color::Wild,
                (_, _) => Color::Partial
            }
        });

        let trailing_path = path_segs.last().map_or(false, |p| p.trailing);

        Metadata {
            base_segs, path_segs, static_query_fields, path_color, query_color,
            trailing_path,
        }
    }
}

impl<'a> std::ops::Deref for RouteUri<'a> {
    type Target = Origin<'a>;

    fn deref(&self) -> &Self::Target {
        &self.origin
    }
}

impl fmt::Display for RouteUri<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.origin.fmt(f)
    }
}

impl fmt::Debug for RouteUri<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouteUri")
            .field("base", &self.base)
            .field("unmounted_origin", &self.unmounted_origin)
            .field("origin", &self.origin)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl<'a, 'b> PartialEq<Origin<'b>> for RouteUri<'a> {
    fn eq(&self, other: &Origin<'b>) -> bool { &self.origin == other }
}

impl PartialEq<str> for RouteUri<'_> {
    fn eq(&self, other: &str) -> bool { self.as_str() == other }
}

impl PartialEq<&str> for RouteUri<'_> {
    fn eq(&self, other: &&str) -> bool { self.as_str() == *other }
}
