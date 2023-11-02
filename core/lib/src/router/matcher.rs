use crate::{Route, Request, Catcher};
use crate::router::Collide;
use crate::http::Status;
use crate::route::Color;

impl Route {
    /// Returns `true` if `self` matches `request`.
    ///
    /// A [_match_](Route#routing) occurs when:
    ///
    ///   * The route's method matches that of the incoming request.
    ///   * Either the route has no format _or_:
    ///     - If the route's method supports a payload, the request's
    ///       `Content-Type` is [fully specified] and [collides with] the
    ///       route's format.
    ///     - If the route's method does not support a payload, the request
    ///       either has no `Accept` header or it [collides with] with the
    ///       route's format.
    ///   * All static segments in the route's URI match the corresponding
    ///     components in the same position in the incoming request URI.
    ///   * The route URI has no query part _or_ all static segments in the
    ///     route's query string are in the request query string, though in any
    ///     position.
    ///
    /// [fully specified]: crate::http::MediaType::specificity()
    /// [collides with]: Route::collides_with()
    ///
    /// For a request to be routed to a particular route, that route must both
    /// `match` _and_ have the highest precedence among all matching routes for
    /// that request. In other words, a `match` is a necessary but insufficient
    /// condition to determine if a route will handle a particular request.
    ///
    /// The precedence of a route is determined by its rank. Routes with lower
    /// ranks have higher precedence. [By default](Route#default-ranking), more
    /// specific routes are assigned a lower ranking.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Route;
    /// use rocket::http::Method;
    /// # use rocket::local::blocking::Client;
    /// # use rocket::route::dummy_handler as handler;
    ///
    /// // This route handles GET requests to `/<hello>`.
    /// let a = Route::new(Method::Get, "/<hello>", handler);
    ///
    /// // This route handles GET requests to `/здрасти`.
    /// let b = Route::new(Method::Get, "/здрасти", handler);
    ///
    /// # let client = Client::debug(rocket::build()).unwrap();
    /// // Let's say `request` is `GET /hello`. The request matches only `a`:
    /// let request = client.get("/hello");
    /// # let request = request.inner();
    /// assert!(a.matches(&request));
    /// assert!(!b.matches(&request));
    ///
    /// // Now `request` is `GET /здрасти`. It matches both `a` and `b`:
    /// let request = client.get("/здрасти");
    /// # let request = request.inner();
    /// assert!(a.matches(&request));
    /// assert!(b.matches(&request));
    ///
    /// // But `b` is more specific, so it has lower rank (higher precedence)
    /// // by default, so Rocket would route the request to `b`, not `a`.
    /// assert!(b.rank < a.rank);
    /// ```
    pub fn matches(&self, request: &Request<'_>) -> bool {
        self.method == request.method()
            && paths_match(self, request)
            && queries_match(self, request)
            && formats_match(self, request)
    }
}

impl Catcher {
    /// Returns `true` if `self` matches errors with `status` that occurred
    /// during `request`.
    ///
    /// A [_match_](Catcher#routing) between a `Catcher` and a (`Status`,
    /// `&Request`) pair occurs when:
    ///
    ///   * The catcher has the same [code](Catcher::code) as
    ///     [`status`](Status::code) _or_ is `default`.
    ///   * The catcher's [base](Catcher::base()) is a prefix of the `request`'s
    ///     [normalized](crate::http::uri::Origin#normalization) URI.
    ///
    /// For an error arising from a request to be routed to a particular
    /// catcher, that catcher must both `match` _and_ have higher precedence
    /// than any other catcher that matches. In other words, a `match` is a
    /// necessary but insufficient condition to determine if a catcher will
    /// handle a particular error.
    ///
    /// The precedence of a catcher is determined by:
    ///
    ///   1. The number of _complete_ segments in the catcher's `base`.
    ///   2. Whether the catcher is `default` or not.
    ///
    /// Non-default routes, and routes with more complete segments in their
    /// base, have higher precedence.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Catcher;
    /// use rocket::http::Status;
    /// # use rocket::local::blocking::Client;
    /// # use rocket::catcher::dummy_handler as handler;
    ///
    /// // This catcher handles 404 errors with a base of `/`.
    /// let a = Catcher::new(404, handler);
    ///
    /// // This catcher handles 404 errors with a base of `/bar`.
    /// let b = a.clone().map_base(|_| format!("/bar")).unwrap();
    ///
    /// # let client = Client::debug(rocket::build()).unwrap();
    /// // Let's say `request` is `GET /` that 404s. The error matches only `a`:
    /// let request = client.get("/");
    /// # let request = request.inner();
    /// assert!(a.matches(Status::NotFound, &request));
    /// assert!(!b.matches(Status::NotFound, &request));
    ///
    /// // Now `request` is a 404 `GET /bar`. The error matches `a` and `b`:
    /// let request = client.get("/bar");
    /// # let request = request.inner();
    /// assert!(a.matches(Status::NotFound, &request));
    /// assert!(b.matches(Status::NotFound, &request));
    ///
    /// // Note that because `b`'s base' has more complete segments that `a's,
    /// // Rocket would route the error to `b`, not `a`, even though both match.
    /// let a_count = a.base().segments().filter(|s| !s.is_empty()).count();
    /// let b_count = b.base().segments().filter(|s| !s.is_empty()).count();
    /// assert!(b_count > a_count);
    /// ```
    pub fn matches(&self, status: Status, request: &Request<'_>) -> bool {
        self.code.map_or(true, |code| code == status.code)
            && self.base().segments().prefix_of(request.uri().path().segments())
    }
}

fn paths_match(route: &Route, req: &Request<'_>) -> bool {
    trace!("checking path match: route {} vs. request {}", route, req);
    let route_segments = &route.uri.metadata.uri_segments;
    let req_segments = req.uri().path().segments();

    // A route can never have more segments than a request. Recall that a
    // trailing slash is considering a segment, albeit empty.
    if route_segments.len() > req_segments.num() {
        return false;
    }

    // requests with longer paths only match if we have dynamic trail (<a..>).
    if req_segments.num() > route_segments.len() {
        if !route.uri.metadata.dynamic_trail {
            return false;
        }
    }

    // We've checked everything beyond the zip of their lengths already.
    for (route_seg, req_seg) in route_segments.iter().zip(req_segments.clone()) {
        if route_seg.dynamic_trail {
            return true;
        }

        if !route_seg.dynamic && route_seg.value != req_seg {
            return false;
        }
    }

    true
}

fn queries_match(route: &Route, req: &Request<'_>) -> bool {
    trace!("checking query match: route {} vs. request {}", route, req);
    if matches!(route.uri.metadata.query_color, None | Some(Color::Wild)) {
        return true;
    }

    let route_query_fields = route.uri.metadata.static_query_fields.iter()
        .map(|(k, v)| (k.as_str(), v.as_str()));

    for route_seg in route_query_fields {
        if let Some(query) = req.uri().query() {
            if !query.segments().any(|req_seg| req_seg == route_seg) {
                trace_!("request {} missing static query {:?}", req, route_seg);
                return false;
            }
        } else {
            trace_!("query-less request {} missing static query {:?}", req, route_seg);
            return false;
        }
    }

    true
}

fn formats_match(route: &Route, req: &Request<'_>) -> bool {
    trace!("checking format match: route {} vs. request {}", route, req);
    let route_format = match route.format {
        Some(ref format) => format,
        None => return true,
    };

    if route.method.supports_payload() {
        match req.format() {
            Some(f) if f.specificity() == 2 => route_format.collides_with(f),
            _ => false
        }
    } else {
        match req.format() {
            Some(f) => route_format.collides_with(f),
            None => true
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::local::blocking::Client;
    use crate::route::{Route, dummy_handler};
    use crate::http::{Method, Method::*, MediaType, ContentType, Accept};

    fn req_matches_route(a: &'static str, b: &'static str) -> bool {
        let client = Client::debug_with(vec![]).expect("client");
        let route = Route::ranked(0, Get, b, dummy_handler);
        route.matches(&client.get(a))
    }

    #[test]
    fn request_route_matching() {
        assert!(req_matches_route("/a/b?a=b", "/a/b?<c>"));
        assert!(req_matches_route("/a/b?a=b", "/<a>/b?<c>"));
        assert!(req_matches_route("/a/b?a=b", "/<a>/<b>?<c>"));
        assert!(req_matches_route("/a/b?a=b", "/a/<b>?<c>"));
        assert!(req_matches_route("/?b=c", "/?<b>"));

        assert!(req_matches_route("/a/b?a=b", "/a/b"));
        assert!(req_matches_route("/a/b", "/a/b"));
        assert!(req_matches_route("/a/b/c/d?", "/a/b/c/d"));
        assert!(req_matches_route("/a/b/c/d?v=1&v=2", "/a/b/c/d"));

        assert!(req_matches_route("/a/b", "/a/b?<c>"));
        assert!(req_matches_route("/a/b", "/a/b?<c..>"));
        assert!(req_matches_route("/a/b?c", "/a/b?c"));
        assert!(req_matches_route("/a/b?c", "/a/b?<c>"));
        assert!(req_matches_route("/a/b?c=foo&d=z", "/a/b?<c>"));
        assert!(req_matches_route("/a/b?c=foo&d=z", "/a/b?<c..>"));
        assert!(req_matches_route("/a/b?c=foo&d=z", "/a/b?c=foo&<c..>"));
        assert!(req_matches_route("/a/b?c=foo&d=z", "/a/b?d=z&<c..>"));

        assert!(req_matches_route("/", "/<foo>"));
        assert!(req_matches_route("/a", "/<foo>"));
        assert!(req_matches_route("/a", "/a"));
        assert!(req_matches_route("/a/", "/a/"));

        assert!(req_matches_route("//", "/"));
        assert!(req_matches_route("/a///", "/a/"));
        assert!(req_matches_route("/a/b", "/a/b"));

        assert!(!req_matches_route("/a///", "/a"));
        assert!(!req_matches_route("/a", "/a/"));
        assert!(!req_matches_route("/a/", "/a"));
        assert!(!req_matches_route("/a/b", "/a/b/"));

        assert!(!req_matches_route("/a", "/<a>/"));
        assert!(!req_matches_route("/a/", "/<a>"));
        assert!(!req_matches_route("/a/b", "/<a>/b/"));
        assert!(!req_matches_route("/a/b", "/<a>/<b>/"));

        assert!(!req_matches_route("/a/b/c", "/a/b?<c>"));
        assert!(!req_matches_route("/a?b=c", "/a/b?<c>"));
        assert!(!req_matches_route("/?b=c", "/a/b?<c>"));
        assert!(!req_matches_route("/?b=c", "/a?<c>"));

        assert!(!req_matches_route("/a/", "/<a>/<b>/<c..>"));
        assert!(!req_matches_route("/a/b", "/<a>/<b>/<c..>"));

        assert!(!req_matches_route("/a/b?c=foo&d=z", "/a/b?a=b&<c..>"));
        assert!(!req_matches_route("/a/b?c=foo&d=z", "/a/b?d=b&<c..>"));
        assert!(!req_matches_route("/a/b", "/a/b?c"));
        assert!(!req_matches_route("/a/b", "/a/b?foo"));
        assert!(!req_matches_route("/a/b", "/a/b?foo&<rest..>"));
        assert!(!req_matches_route("/a/b", "/a/b?<a>&b&<rest..>"));
    }

    fn req_matches_format<S1, S2>(m: Method, mt1: S1, mt2: S2) -> bool
        where S1: Into<Option<&'static str>>, S2: Into<Option<&'static str>>
    {
        let client = Client::debug_with(vec![]).expect("client");
        let mut req = client.req(m, "/");
        if let Some(mt_str) = mt1.into() {
            if m.supports_payload() {
                req.replace_header(mt_str.parse::<ContentType>().unwrap());
            } else {
                req.replace_header(mt_str.parse::<Accept>().unwrap());
            }
        }

        let mut route = Route::new(m, "/", dummy_handler);
        if let Some(mt_str) = mt2.into() {
            route.format = Some(mt_str.parse::<MediaType>().unwrap());
        }

        route.matches(&req)
    }

    #[test]
    fn test_req_route_mt_collisions() {
        assert!(req_matches_format(Post, "application/json", "application/json"));
        assert!(req_matches_format(Post, "application/json", "application/*"));
        assert!(req_matches_format(Post, "application/json", "*/json"));
        assert!(req_matches_format(Post, "text/html", "*/*"));

        assert!(req_matches_format(Get, "application/json", "application/json"));
        assert!(req_matches_format(Get, "text/html", "text/html"));
        assert!(req_matches_format(Get, "text/html", "*/*"));
        assert!(req_matches_format(Get, None, "*/*"));
        assert!(req_matches_format(Get, None, "text/*"));
        assert!(req_matches_format(Get, None, "text/html"));
        assert!(req_matches_format(Get, None, "application/json"));

        assert!(req_matches_format(Post, "text/html", None));
        assert!(req_matches_format(Post, "application/json", None));
        assert!(req_matches_format(Post, "x-custom/anything", None));
        assert!(req_matches_format(Post, None, None));

        assert!(req_matches_format(Get, "text/html", None));
        assert!(req_matches_format(Get, "application/json", None));
        assert!(req_matches_format(Get, "x-custom/anything", None));
        assert!(req_matches_format(Get, None, None));
        assert!(req_matches_format(Get, None, "text/html"));
        assert!(req_matches_format(Get, None, "application/json"));

        assert!(req_matches_format(Get, "text/html, text/plain", "text/html"));
        assert!(req_matches_format(Get, "text/html; q=0.5, text/xml", "text/xml"));

        assert!(!req_matches_format(Post, None, "text/html"));
        assert!(!req_matches_format(Post, None, "text/*"));
        assert!(!req_matches_format(Post, None, "*/text"));
        assert!(!req_matches_format(Post, None, "*/*"));
        assert!(!req_matches_format(Post, None, "text/html"));
        assert!(!req_matches_format(Post, None, "application/json"));

        assert!(!req_matches_format(Post, "application/json", "text/html"));
        assert!(!req_matches_format(Post, "application/json", "text/*"));
        assert!(!req_matches_format(Post, "application/json", "*/xml"));
        assert!(!req_matches_format(Get, "application/json", "text/html"));
        assert!(!req_matches_format(Get, "application/json", "text/*"));
        assert!(!req_matches_format(Get, "application/json", "*/xml"));

        assert!(!req_matches_format(Post, None, "text/html"));
        assert!(!req_matches_format(Post, None, "application/json"));
    }
}
