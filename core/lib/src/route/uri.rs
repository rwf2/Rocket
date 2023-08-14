use std::fmt;

use crate::http::uri::{self, Origin, Path};
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
#[derive(Debug, Clone)]
pub struct RouteUri<'a> {
    /// The mount point.
    pub(crate) base: Origin<'a>,
    /// The URI _without_ the `base` mount point.
    pub(crate) unmounted_origin: Origin<'a>,
    /// The URI _with_ the base mount point. This is the canonical route URI.
    pub(crate) uri: Origin<'a>,
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
    /// Segments in the route URI, including base.
    pub uri_segments: Vec<Segment>,
    /// Numbers of segments in `uri_segments` that belong to the base.
    pub base_len: usize,
    /// `(name, value)` of the query segments that are static.
    pub static_query_fields: Vec<(String, String)>,
    /// The "color" of the route path.
    pub path_color: Color,
    /// The "color" of the route query, if there is query.
    pub query_color: Option<Color>,
    /// Whether the path has a `<trailing..>` parameter.
    pub dynamic_trail: bool,
}

type Result<T, E = uri::Error<'static>> = std::result::Result<T, E>;

impl<'a> RouteUri<'a> {
    /// Create a new `RouteUri`.
    ///
    /// Panics if  `base` or `uri` cannot be parsed as `Origin`s.
    #[track_caller]
    pub(crate) fn new(base: &str, uri: &str) -> RouteUri<'static> {
        Self::try_new(base, uri).expect("expected valid route URIs")
    }

    /// Creates a new `RouteUri` from a `base` mount point and a route `uri`.
    ///
    /// This is a fallible variant of [`RouteUri::new`] which returns an `Err`
    /// if `base` or `uri` cannot be parsed as [`Origin`]s.
    /// INTERNAL!
    #[doc(hidden)]
    pub fn try_new(base: &str, uri: &str) -> Result<RouteUri<'static>> {
        let mut base = Origin::parse(base)
            .map_err(|e| e.into_owned())?
            .into_normalized()
            .into_owned();

        base.clear_query();

        let origin = Origin::parse_route(uri)
            .map_err(|e| e.into_owned())?
            .into_normalized()
            .into_owned();

        // Distinguish for routes `/` with bases of `/foo/` and `/foo`. The
        // latter base, without a trailing slash, should combine as `/foo`.
        let route_uri = match origin.path().as_str() {
            "/" if !base.has_trailing_slash() => match origin.query() {
                Some(query) => format!("{}?{}", base, query),
                None => base.to_string(),
            }
            _ => format!("{}{}", base, origin),
        };

        let uri = Origin::parse_route(&route_uri)
            .map_err(|e| e.into_owned())?
            .into_normalized()
            .into_owned();

        let metadata = Metadata::from(&base, &uri);

        Ok(RouteUri { base, unmounted_origin: origin, uri, metadata })
    }

    /// Returns the complete route URI.
    ///
    /// **Note:** `RouteURI` derefs to the `Origin` returned by this method, so
    /// this method should rarely be called directly.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::route::dummy_handler as handler;
    ///
    /// let route = Route::new(Method::Get, "/foo/bar?a=1", handler);
    ///
    /// // Use `inner()` directly:
    /// assert_eq!(route.uri.inner().query().unwrap(), "a=1");
    ///
    /// // Use the deref implementation. This is preferred:
    /// assert_eq!(route.uri.query().unwrap(), "a=1");
    /// ```
    pub fn inner(&self) -> &Origin<'a> {
        &self.uri
    }

    /// The base mount point of this route URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::route::dummy_handler as handler;
    /// # use rocket::uri;
    ///
    /// let route = Route::new(Method::Get, "/foo/bar?a=1", handler);
    /// assert_eq!(route.uri.base(), "/");
    ///
    /// let route = route.rebase(uri!("/boo"));
    /// assert_eq!(route.uri.base(), "/boo");
    ///
    /// let route = route.rebase(uri!("/foo"));
    /// assert_eq!(route.uri.base(), "/foo/boo");
    /// ```
    #[inline(always)]
    pub fn base(&self) -> Path<'_> {
        self.base.path()
    }

    /// The route URI _without_ the base mount point.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::route::dummy_handler as handler;
    /// # use rocket::uri;
    ///
    /// let route = Route::new(Method::Get, "/foo/bar?a=1", handler);
    /// let route = route.rebase(uri!("/boo"));
    ///
    /// assert_eq!(route.uri, "/boo/foo/bar?a=1");
    /// assert_eq!(route.uri.base(), "/boo");
    /// assert_eq!(route.uri.unmounted(), "/foo/bar?a=1");
    /// ```
    #[inline(always)]
    pub fn unmounted(&self) -> &Origin<'a> {
        &self.unmounted_origin
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

    pub(crate) fn color_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use yansi::Paint;

        let (path, base, unmounted) = (self.uri.path(), self.base(), self.unmounted().path());
        let unmounted_part = path.strip_prefix(base.as_str())
            .map(|raw| raw.as_str())
            .unwrap_or(unmounted.as_str());

        write!(f, "{}{}", self.base().blue().underline(), unmounted_part.blue())?;
        if let Some(q) = self.unmounted().query() {
            write!(f, "{}{}", "?".yellow(), q.yellow())?;
        }

        Ok(())
    }
}

impl Metadata {
    fn from(base: &Origin<'_>, uri: &Origin<'_>) -> Self {
        let uri_segments = uri.path()
            .raw_segments()
            .map(Segment::from)
            .collect::<Vec<_>>();

        let query_segs = uri.query()
            .map(|q| q.raw_segments().map(Segment::from).collect::<Vec<_>>())
            .unwrap_or_default();

        let static_query_fields = query_segs.iter().filter(|s| !s.dynamic)
            .map(|s| ValueField::parse(&s.value))
            .map(|f| (f.name.source().to_string(), f.value.to_string()))
            .collect();

        let static_path = uri_segments.iter().all(|s| !s.dynamic);
        let wild_path = !uri_segments.is_empty() && uri_segments.iter().all(|s| s.dynamic);
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

        let dynamic_trail = uri_segments.last().map_or(false, |p| p.dynamic_trail);
        let segments = base.path().segments();
        let num_empty = segments.clone().filter(|s| s.is_empty()).count();
        let base_len = segments.num() - num_empty;

        Metadata {
            uri_segments, base_len, static_query_fields, path_color, query_color, dynamic_trail
        }
    }
}

impl<'a> std::ops::Deref for RouteUri<'a> {
    type Target = Origin<'a>;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl fmt::Display for RouteUri<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.uri.fmt(f)
    }
}

impl<'a, 'b> PartialEq<Origin<'b>> for RouteUri<'a> {
    fn eq(&self, other: &Origin<'b>) -> bool { self.inner() == other }
}

impl PartialEq<str> for RouteUri<'_> {
    fn eq(&self, other: &str) -> bool { self.inner() == other }
}

impl PartialEq<&str> for RouteUri<'_> {
    fn eq(&self, other: &&str) -> bool { self.inner() == *other }
}

#[cfg(test)]
mod tests {
    macro_rules! assert_uri_equality {
        ($base:expr, $path:expr => $ebase:expr, $epath:expr, $efull:expr) => {
            let uri = super::RouteUri::new($base, $path);
            assert_eq!(uri, $efull, "complete URI mismatch. expected {}, got {}", $efull, uri);
            assert_eq!(uri.base(), $ebase, "expected base {}, got {}", $ebase, uri.base());
            assert_eq!(uri.unmounted(), $epath, "expected unmounted {}, got {}", $epath,
                uri.unmounted());
        };
    }

    #[test]
    fn test_route_uri_composition() {
        assert_uri_equality!("/", "/" => "/", "/", "/");
        assert_uri_equality!("/", "/foo" => "/", "/foo", "/foo");
        assert_uri_equality!("/", "/foo/bar" => "/", "/foo/bar", "/foo/bar");
        assert_uri_equality!("/", "/foo/" => "/", "/foo/", "/foo/");
        assert_uri_equality!("/", "/foo/bar/" => "/", "/foo/bar/", "/foo/bar/");

        assert_uri_equality!("/foo", "/" => "/foo", "/", "/foo");
        assert_uri_equality!("/foo", "/bar" => "/foo", "/bar", "/foo/bar");
        assert_uri_equality!("/foo", "/bar/" => "/foo", "/bar/", "/foo/bar/");
        assert_uri_equality!("/foo", "/?baz" => "/foo", "/?baz", "/foo?baz");
        assert_uri_equality!("/foo", "/bar?baz" => "/foo", "/bar?baz", "/foo/bar?baz");
        assert_uri_equality!("/foo", "/bar/?baz" => "/foo", "/bar/?baz", "/foo/bar/?baz");

        assert_uri_equality!("/foo/", "/" => "/foo/", "/", "/foo/");
        assert_uri_equality!("/foo/", "/bar" => "/foo/", "/bar", "/foo/bar");
        assert_uri_equality!("/foo/", "/bar/" => "/foo/", "/bar/", "/foo/bar/");
        assert_uri_equality!("/foo/", "/?baz" => "/foo/", "/?baz", "/foo/?baz");
        assert_uri_equality!("/foo/", "/bar?baz" => "/foo/", "/bar?baz", "/foo/bar?baz");
        assert_uri_equality!("/foo/", "/bar/?baz" => "/foo/", "/bar/?baz", "/foo/bar/?baz");

        assert_uri_equality!("/foo?baz", "/" => "/foo", "/", "/foo");
        assert_uri_equality!("/foo?baz", "/bar" => "/foo", "/bar", "/foo/bar");
        assert_uri_equality!("/foo?baz", "/bar/" => "/foo", "/bar/", "/foo/bar/");
        assert_uri_equality!("/foo/?baz", "/" => "/foo/", "/", "/foo/");
        assert_uri_equality!("/foo/?baz", "/bar" => "/foo/", "/bar", "/foo/bar");
        assert_uri_equality!("/foo/?baz", "/bar/" => "/foo/", "/bar/", "/foo/bar/");
    }
}
