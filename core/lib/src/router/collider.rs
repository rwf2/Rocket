use crate::catcher::Catcher;
use crate::route::{Route, Segment, RouteUri};

use crate::http::MediaType;

pub trait Collide<T = Self> {
    fn collides_with(&self, other: &T) -> bool;
}

impl Route {
    /// Returns `true` if `self` collides with `other`.
    ///
    /// A [_collision_](Route#collisions) between two routes occurs when there
    /// exists a request that could [match](Route::matches()) either route. That
    /// is, a routing ambiguity would ensue if both routes were made available
    /// to the router.
    ///
    /// Specifically, a collision occurs when two routes `a` and `b`:
    ///
    ///  * Have the same [method](Route::method).
    ///  * Have the same [rank](Route#default-ranking).
    ///  * The routes' methods don't support a payload _or_ the routes'
    ///    methods support a payload and the formats overlap. Formats overlap
    ///    when:
    ///    - The top-level type of either is `*` or the top-level types are
    ///      equivalent.
    ///    - The sub-level type of either is `*` or the sub-level types are
    ///      equivalent.
    ///  * Have overlapping route URIs. This means that either:
    ///    - The URIs have the same number of segments `n`, and for `i` in
    ///      `0..n`, either `a.uri[i]` is dynamic _or_ `b.uri[i]` is dynamic
    ///      _or_ they're both static with the same value.
    ///    - One URI has fewer segments _and_ ends with a trailing dynamic
    ///      parameter _and_ the preceding segments in both routes match the
    ///      conditions above.
    ///
    /// Collisions are symmetric: for any routes `a` and `b`,
    /// `a.collides_with(b) => b.collides_with(a)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::{Method, MediaType};
    /// # use rocket::route::dummy_handler as handler;
    ///
    /// // Two routes with the same method, rank, URI, and formats collide.
    /// let a = Route::new(Method::Get, "/", handler);
    /// let b = Route::new(Method::Get, "/", handler);
    /// assert!(a.collides_with(&b));
    ///
    /// // Two routes with the same method, rank, URI, and overlapping formats.
    /// let mut a = Route::new(Method::Post, "/", handler);
    /// a.format = Some(MediaType::new("*", "custom"));
    /// let mut b = Route::new(Method::Post, "/", handler);
    /// b.format = Some(MediaType::new("text", "*"));
    /// assert!(a.collides_with(&b));
    ///
    /// // Two routes with different ranks don't collide.
    /// let a = Route::ranked(1, Method::Get, "/", handler);
    /// let b = Route::ranked(2, Method::Get, "/", handler);
    /// assert!(!a.collides_with(&b));
    ///
    /// // Two routes with different methods don't collide.
    /// let a = Route::new(Method::Put, "/", handler);
    /// let b = Route::new(Method::Post, "/", handler);
    /// assert!(!a.collides_with(&b));
    ///
    /// // Two routes with non-overlapping URIs do not collide.
    /// let a = Route::new(Method::Get, "/foo", handler);
    /// let b = Route::new(Method::Get, "/bar/<baz>", handler);
    /// assert!(!a.collides_with(&b));
    ///
    /// // Two payload-supporting routes with non-overlapping formats.
    /// let mut a = Route::new(Method::Post, "/", handler);
    /// a.format = Some(MediaType::HTML);
    /// let mut b = Route::new(Method::Post, "/", handler);
    /// b.format = Some(MediaType::JSON);
    /// assert!(!a.collides_with(&b));
    ///
    /// // Two non payload-supporting routes with non-overlapping formats
    /// // collide. A request with `Accept: */*` matches both.
    /// let mut a = Route::new(Method::Get, "/", handler);
    /// a.format = Some(MediaType::HTML);
    /// let mut b = Route::new(Method::Get, "/", handler);
    /// b.format = Some(MediaType::JSON);
    /// assert!(a.collides_with(&b));
    /// ```
    pub fn collides_with(&self, other: &Route) -> bool {
        self.method == other.method
            && self.rank == other.rank
            && self.uri.collides_with(&other.uri)
            && formats_collide(self, other)
    }
}

impl Catcher {
    /// Returns `true` if `self` collides with `other`.
    ///
    /// A [_collision_](Catcher#collisions) between two catchers occurs when
    /// there exists a request and ensuing error that could
    /// [match](Catcher::matches()) both catchers. That is, a routing ambiguity
    /// would ensue if both catchers were made available to the router.
    ///
    /// Specifically, a collision occurs when two catchers:
    ///
    ///  * Have the same [base](Catcher::base()).
    ///  * Have the same status [code](Catcher::code) or are both `default`.
    ///
    /// Collisions are symmetric: for any catchers `a` and `b`,
    /// `a.collides_with(b) => b.collides_with(a)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Catcher;
    /// # use rocket::catcher::dummy_handler as handler;
    ///
    /// // Two catchers with the same status code and base collide.
    /// let a = Catcher::new(404, handler).map_base(|_| format!("/foo")).unwrap();
    /// let b = Catcher::new(404, handler).map_base(|_| format!("/foo")).unwrap();
    /// assert!(a.collides_with(&b));
    ///
    /// // Two catchers with a different base _do not_ collide.
    /// let a = Catcher::new(404, handler);
    /// let b = a.clone().map_base(|_| format!("/bar")).unwrap();
    /// assert_eq!(a.base(), "/");
    /// assert_eq!(b.base(), "/bar");
    /// assert!(!a.collides_with(&b));
    ///
    /// // Two catchers with a different codes _do not_ collide.
    /// let a = Catcher::new(404, handler);
    /// let b = Catcher::new(500, handler);
    /// assert_eq!(a.base(), "/");
    /// assert_eq!(b.base(), "/");
    /// assert!(!a.collides_with(&b));
    ///
    /// // A catcher _with_ a status code and one _without_ do not collide.
    /// let a = Catcher::new(404, handler);
    /// let b = Catcher::new(None, handler);
    /// assert!(!a.collides_with(&b));
    /// ```
    pub fn collides_with(&self, other: &Self) -> bool {
        self.code == other.code && self.base().segments().eq(other.base().segments())
    }
}

impl Collide for Route {
    #[inline(always)]
    fn collides_with(&self, other: &Route) -> bool {
        Route::collides_with(&self, other)
    }
}

impl Collide for Catcher {
    #[inline(always)]
    fn collides_with(&self, other: &Self) -> bool {
        Catcher::collides_with(&self, other)
    }
}

impl Collide for RouteUri<'_> {
    fn collides_with(&self, other: &Self) -> bool {
        let a_segments = &self.metadata.uri_segments;
        let b_segments = &other.metadata.uri_segments;
        for (seg_a, seg_b) in a_segments.iter().zip(b_segments.iter()) {
            if seg_a.dynamic_trail || seg_b.dynamic_trail {
                return true;
            }

            if !seg_a.collides_with(seg_b) {
                return false;
            }
        }

        a_segments.len() == b_segments.len()
    }
}

impl Collide for Segment {
    fn collides_with(&self, other: &Self) -> bool {
        self.dynamic || other.dynamic || self.value == other.value
    }
}

impl Collide for MediaType {
    fn collides_with(&self, other: &Self) -> bool {
        let collide = |a, b| a == "*" || b == "*" || a == b;
        collide(self.top(), other.top()) && collide(self.sub(), other.sub())
    }
}

fn formats_collide(route: &Route, other: &Route) -> bool {
    // If the routes' method doesn't support a payload, then format matching
    // considers the `Accept` header. The client can always provide a media type
    // that will cause a collision through non-specificity, i.e, `*/*`.
    if !route.method.supports_payload() && !other.method.supports_payload() {
        return true;
    }

    // Payload supporting methods match against `Content-Type`. We only
    // consider requests as having a `Content-Type` if they're fully
    // specified. A route without a `format` accepts all `Content-Type`s. A
    // request without a format only matches routes without a format.
    match (route.format.as_ref(), other.format.as_ref()) {
        (Some(a), Some(b)) => a.collides_with(b),
        _ => true
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::route::{Route, dummy_handler};
    use crate::http::{Method, Method::*, MediaType};

    fn dummy_route(ranked: bool, method: impl Into<Option<Method>>, uri: &'static str) -> Route {
        let method = method.into().unwrap_or(Get);
        Route::ranked((!ranked).then(|| 0), method, uri, dummy_handler)
    }

    macro_rules! assert_collision {
        ($ranked:expr, $p1:expr, $p2:expr) => (assert_collision!($ranked, None $p1, None $p2));
        ($ranked:expr, $m1:ident $p1:expr, $m2:ident $p2:expr) => {
            let (a, b) = (dummy_route($ranked, $m1, $p1), dummy_route($ranked, $m2, $p2));
            assert! {
                a.collides_with(&b),
                "\nroutes failed to collide:\n{} does not collide with {}\n", a, b
            }
        };
        (ranked $($t:tt)+) => (assert_collision!(true, $($t)+));
        ($($t:tt)+) => (assert_collision!(false, $($t)+));
    }

    macro_rules! assert_no_collision {
        ($ranked:expr, $p1:expr, $p2:expr) => (assert_no_collision!($ranked, None $p1, None $p2));
        ($ranked:expr, $m1:ident $p1:expr, $m2:ident $p2:expr) => {
            let (a, b) = (dummy_route($ranked, $m1, $p1), dummy_route($ranked, $m2, $p2));
            assert! {
                !a.collides_with(&b),
                "\nunexpected collision:\n{} collides with {}\n", a, b
            }
        };
        (ranked $($t:tt)+) => (assert_no_collision!(true, $($t)+));
        ($($t:tt)+) => (assert_no_collision!(false, $($t)+));
    }

    #[test]
    fn non_collisions() {
        assert_no_collision!("/a", "/b");
        assert_no_collision!("/a/b", "/a");
        assert_no_collision!("/a/b", "/a/c");
        assert_no_collision!("/a/hello", "/a/c");
        assert_no_collision!("/hello", "/a/c");
        assert_no_collision!("/hello/there", "/hello/there/guy");
        assert_no_collision!("/a/<b>", "/b/<b>");
        assert_no_collision!("/<a>/b", "/<b>/a");
        assert_no_collision!("/t", "/test");
        assert_no_collision!("/a", "/aa");
        assert_no_collision!("/a", "/aaa");
        assert_no_collision!("/", "/a");

        assert_no_collision!("/hello", "/hello/");
        assert_no_collision!("/hello/there", "/hello/there/");

        assert_no_collision!("/a?<b>", "/b");
        assert_no_collision!("/a/b", "/a?<b>");
        assert_no_collision!("/a/b/c?<d>", "/a/b/c/d");
        assert_no_collision!("/a/hello", "/a/?<hello>");
        assert_no_collision!("/?<a>", "/hi");

        assert_no_collision!(Get "/", Post "/");
        assert_no_collision!(Post "/", Put "/");
        assert_no_collision!(Put "/a", Put "/");
        assert_no_collision!(Post "/a", Put "/");
        assert_no_collision!(Get "/a", Put "/");
        assert_no_collision!(Get "/hello", Put "/hello");
        assert_no_collision!(Get "/<foo..>", Post "/");

        assert_no_collision!("/a", "/b");
        assert_no_collision!("/a/b", "/a");
        assert_no_collision!("/a/b", "/a/c");
        assert_no_collision!("/a/hello", "/a/c");
        assert_no_collision!("/hello", "/a/c");
        assert_no_collision!("/hello/there", "/hello/there/guy");
        assert_no_collision!("/a/<b>", "/b/<b>");
        assert_no_collision!("/a", "/b");
        assert_no_collision!("/a/b", "/a");
        assert_no_collision!("/a/b", "/a/c");
        assert_no_collision!("/a/hello", "/a/c");
        assert_no_collision!("/hello", "/a/c");
        assert_no_collision!("/hello/there", "/hello/there/guy");
        assert_no_collision!("/a/<b>", "/b/<b>");
        assert_no_collision!("/a", "/b");
        assert_no_collision!("/a/b", "/a");
        assert_no_collision!("/a/b", "/a/c");
        assert_no_collision!("/a/hello", "/a/c");
        assert_no_collision!("/hello", "/a/c");
        assert_no_collision!("/hello/there", "/hello/there/guy");
        assert_no_collision!("/a/<b>", "/b/<b>");
        assert_no_collision!("/t", "/test");
        assert_no_collision!("/a", "/aa");
        assert_no_collision!("/a", "/aaa");
        assert_no_collision!("/", "/a");

        assert_no_collision!("/foo", "/foo/");
        assert_no_collision!("/foo/bar", "/foo/");
        assert_no_collision!("/foo/bar", "/foo/bar/");
        assert_no_collision!("/foo/<a>", "/foo/<a>/");
        assert_no_collision!("/foo/<a>", "/<b>/<a>/");
        assert_no_collision!("/<b>/<a>", "/<b>/<a>/");
        assert_no_collision!("/a/", "/<a>/<b>/<c..>");

        assert_no_collision!("/a", "/a/<a..>");
        assert_no_collision!("/<a>", "/a/<a..>");
        assert_no_collision!("/a/b", "/<a>/<b>/<c..>");
        assert_no_collision!("/a/<b>", "/<a>/<b>/<c..>");
        assert_no_collision!("/<a>/b", "/<a>/<b>/<c..>");
        assert_no_collision!("/hi/<a..>", "/hi");

        assert_no_collision!(ranked "/<a>", "/");
        assert_no_collision!(ranked "/a/", "/<a>/");
        assert_no_collision!(ranked "/hello/<a>", "/hello/");
        assert_no_collision!(ranked "/", "/?a");
        assert_no_collision!(ranked "/", "/?<a>");
        assert_no_collision!(ranked "/a/<b>", "/a/<b>?d");
    }

    #[test]
    fn collisions() {
        assert_collision!("/<a>", "/");
        assert_collision!("/a", "/a");
        assert_collision!("/hello", "/hello");
        assert_collision!("/hello/there/how/ar", "/hello/there/how/ar");
        assert_collision!("/hello/<a>", "/hello/");

        assert_collision!("/<a>", "/<b>");
        assert_collision!("/<a>", "/b");
        assert_collision!("/hello/<name>", "/hello/<person>");
        assert_collision!("/hello/<name>/hi", "/hello/<person>/hi");
        assert_collision!("/hello/<name>/hi/there", "/hello/<person>/hi/there");
        assert_collision!("/<name>/hi/there", "/<person>/hi/there");
        assert_collision!("/<name>/hi/there", "/dude/<name>/there");
        assert_collision!("/<name>/<a>/<b>", "/<a>/<b>/<c>");
        assert_collision!("/<name>/<a>/<b>/", "/<a>/<b>/<c>/");
        assert_collision!("/<a..>", "/hi");
        assert_collision!("/<a..>", "/hi/hey");
        assert_collision!("/<a..>", "/hi/hey/hayo");
        assert_collision!("/a/<a..>", "/a/hi/hey/hayo");
        assert_collision!("/a/<b>/<a..>", "/a/hi/hey/hayo");
        assert_collision!("/a/<b>/<c>/<a..>", "/a/hi/hey/hayo");
        assert_collision!("/<b>/<c>/<a..>", "/a/hi/hey/hayo");
        assert_collision!("/<b>/<c>/hey/hayo", "/a/hi/hey/hayo");
        assert_collision!("/<a..>", "/foo");

        assert_collision!("/", "/<a..>");
        assert_collision!("/a/", "/a/<a..>");
        assert_collision!("/<a>/", "/a/<a..>");
        assert_collision!("/<a>/bar/", "/a/<a..>");

        assert_collision!("/<a>", "/b");
        assert_collision!("/hello/<name>", "/hello/bob");
        assert_collision!("/<name>", "//bob");

        assert_collision!("/<a..>", "///a///");
        assert_collision!("/<a..>", "//a/bcjdklfj//<c>");
        assert_collision!("/a/<a..>", "//a/bcjdklfj//<c>");
        assert_collision!("/a/<b>/<c..>", "//a/bcjdklfj//<c>");
        assert_collision!("/<a..>", "/");
        assert_collision!("/", "/<_..>");
        assert_collision!("/a/b/<a..>", "/a/<b..>");
        assert_collision!("/a/b/<a..>", "/a/<b>/<b..>");
        assert_collision!("/hi/<a..>", "/hi/");
        assert_collision!("/<a..>", "//////");

        assert_collision!("/?<a>", "/?<a>");
        assert_collision!("/a/?<a>", "/a/?<a>");
        assert_collision!("/a?<a>", "/a?<a>");
        assert_collision!("/<r>?<a>", "/<r>?<a>");
        assert_collision!("/a/b/c?<a>", "/a/b/c?<a>");
        assert_collision!("/<a>/b/c?<d>", "/a/b/<c>?<d>");
        assert_collision!("/?<a>", "/");
        assert_collision!("/a?<a>", "/a");
        assert_collision!("/a?<a>", "/a");
        assert_collision!("/a/b?<a>", "/a/b");
        assert_collision!("/a/b", "/a/b?<c>");

        assert_collision!("/a/hi/<a..>", "/a/hi/");
        assert_collision!("/hi/<a..>", "/hi/");
        assert_collision!("/<a..>", "/");
    }

    fn mt_mt_collide(mt1: &str, mt2: &str) -> bool {
        let mt_a = MediaType::from_str(mt1).expect(mt1);
        let mt_b = MediaType::from_str(mt2).expect(mt2);
        mt_a.collides_with(&mt_b)
    }

    #[test]
    fn test_content_type_collisions() {
        assert!(mt_mt_collide("application/json", "application/json"));
        assert!(mt_mt_collide("*/json", "application/json"));
        assert!(mt_mt_collide("*/*", "application/json"));
        assert!(mt_mt_collide("application/*", "application/json"));
        assert!(mt_mt_collide("application/*", "*/json"));
        assert!(mt_mt_collide("something/random", "something/random"));

        assert!(!mt_mt_collide("text/*", "application/*"));
        assert!(!mt_mt_collide("*/text", "*/json"));
        assert!(!mt_mt_collide("*/text", "application/test"));
        assert!(!mt_mt_collide("something/random", "something_else/random"));
        assert!(!mt_mt_collide("something/random", "*/else"));
        assert!(!mt_mt_collide("*/random", "*/else"));
        assert!(!mt_mt_collide("something/*", "random/else"));
    }

    fn r_mt_mt_collide<S1, S2>(m: Method, mt1: S1, mt2: S2) -> bool
        where S1: Into<Option<&'static str>>, S2: Into<Option<&'static str>>
    {
        let mut route_a = Route::new(m, "/", dummy_handler);
        if let Some(mt_str) = mt1.into() {
            route_a.format = Some(mt_str.parse::<MediaType>().unwrap());
        }

        let mut route_b = Route::new(m, "/", dummy_handler);
        if let Some(mt_str) = mt2.into() {
            route_b.format = Some(mt_str.parse::<MediaType>().unwrap());
        }

        route_a.collides_with(&route_b)
    }

    #[test]
    fn test_route_content_type_collisions() {
        // non-payload bearing routes always collide
        assert!(r_mt_mt_collide(Get, "application/json", "application/json"));
        assert!(r_mt_mt_collide(Get, "*/json", "application/json"));
        assert!(r_mt_mt_collide(Get, "*/json", "application/*"));
        assert!(r_mt_mt_collide(Get, "text/html", "text/*"));
        assert!(r_mt_mt_collide(Get, "any/thing", "*/*"));

        assert!(r_mt_mt_collide(Get, None, "text/*"));
        assert!(r_mt_mt_collide(Get, None, "text/html"));
        assert!(r_mt_mt_collide(Get, None, "*/*"));
        assert!(r_mt_mt_collide(Get, "text/html", None));
        assert!(r_mt_mt_collide(Get, "*/*", None));
        assert!(r_mt_mt_collide(Get, "application/json", None));

        assert!(r_mt_mt_collide(Get, "application/*", "text/*"));
        assert!(r_mt_mt_collide(Get, "application/json", "text/*"));
        assert!(r_mt_mt_collide(Get, "application/json", "text/html"));
        assert!(r_mt_mt_collide(Get, "text/html", "text/html"));

        // payload bearing routes collide if the media types collide
        assert!(r_mt_mt_collide(Post, "application/json", "application/json"));
        assert!(r_mt_mt_collide(Post, "*/json", "application/json"));
        assert!(r_mt_mt_collide(Post, "*/json", "application/*"));
        assert!(r_mt_mt_collide(Post, "text/html", "text/*"));
        assert!(r_mt_mt_collide(Post, "any/thing", "*/*"));

        assert!(r_mt_mt_collide(Post, None, "text/*"));
        assert!(r_mt_mt_collide(Post, None, "text/html"));
        assert!(r_mt_mt_collide(Post, None, "*/*"));
        assert!(r_mt_mt_collide(Post, "text/html", None));
        assert!(r_mt_mt_collide(Post, "*/*", None));
        assert!(r_mt_mt_collide(Post, "application/json", None));

        assert!(!r_mt_mt_collide(Post, "text/html", "application/*"));
        assert!(!r_mt_mt_collide(Post, "application/html", "text/*"));
        assert!(!r_mt_mt_collide(Post, "*/json", "text/html"));
        assert!(!r_mt_mt_collide(Post, "text/html", "text/css"));
        assert!(!r_mt_mt_collide(Post, "other/html", "text/html"));
    }

    fn catchers_collide<A, B>(a: A, ap: &str, b: B, bp: &str) -> bool
        where A: Into<Option<u16>>, B: Into<Option<u16>>
    {
        use crate::catcher::dummy_handler as handler;

        let a = Catcher::new(a, handler).map_base(|_| ap.into()).unwrap();
        let b = Catcher::new(b, handler).map_base(|_| bp.into()).unwrap();
        a.collides_with(&b)
    }

    #[test]
    fn catcher_collisions() {
        for path in &["/a", "/foo", "/a/b/c", "/a/b/c/d/e"] {
            assert!(catchers_collide(404, path, 404, path));
            assert!(catchers_collide(500, path, 500, path));
            assert!(catchers_collide(None, path, None, path));
        }
    }

    #[test]
    fn catcher_non_collisions() {
        assert!(!catchers_collide(404, "/foo", 405, "/foo"));
        assert!(!catchers_collide(404, "/", None, "/foo"));
        assert!(!catchers_collide(404, "/", None, "/"));
        assert!(!catchers_collide(404, "/a/b", None, "/a/b"));
        assert!(!catchers_collide(404, "/a/b", 404, "/a/b/c"));

        assert!(!catchers_collide(None, "/a/b", None, "/a/b/c"));
        assert!(!catchers_collide(None, "/b", None, "/a/b/c"));
        assert!(!catchers_collide(None, "/", None, "/a/b/c"));
    }
}
