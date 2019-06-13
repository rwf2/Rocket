mod collider;
mod route;

use std::collections::hash_map::HashMap;

pub use self::route::Route;

use crate::request::Request;
use crate::http::Method;

// type Selector = (Method, usize);
type Selector = Method;

// A handler to use when one is needed temporarily.
crate fn dummy_handler<'r>(r: &'r crate::Request<'_>, _: crate::Data) -> crate::handler::Outcome<'r> {
    crate::Outcome::from(r, ())
}

#[derive(Default)]
pub struct Router {
    routes: HashMap<Selector, Vec<Route>>,
}

impl Router {
    pub fn new() -> Router {
        Router { routes: HashMap::new() }
    }

    pub fn add(&mut self, route: Route) {
        let selector = route.method;
        let entries = self.routes.entry(selector).or_insert_with(|| vec![]);
        let i = entries.binary_search_by_key(&route.rank, |r| r.rank)
            .unwrap_or_else(|i| i);

        entries.insert(i, route);
    }

    pub fn route<'b>(&'b self, req: &Request<'_>) -> Vec<&'b Route> {
        // Note that routes are presorted by rank on each `add`.
        let matches = self.routes.get(&req.method()).map_or(vec![], |routes| {
            routes.iter()
                .filter(|r| r.matches(req))
                .collect()
        });

        trace_!("Routing the request: {}", req);
        trace_!("All matches: {:?}", matches);
        matches
    }

    crate fn collisions(mut self) -> Result<Router, Vec<(Route, Route)>> {
        let mut collisions = vec![];
        for routes in self.routes.values_mut() {
            for i in 0..routes.len() {
                let (left, right) = routes.split_at_mut(i);
                for a_route in left.iter_mut() {
                    for b_route in right.iter_mut() {
                        if a_route.collides_with(b_route) {
                            let dummy_a = Route::new(Method::Get, "/", dummy_handler);
                            let a = std::mem::replace(a_route, dummy_a);
                            let dummy_b = Route::new(Method::Get, "/", dummy_handler);
                            let b = std::mem::replace(b_route, dummy_b);
                            collisions.push((a, b));
                        }
                    }
                }
            }
        }

        if collisions.is_empty() {
            Ok(self)
        } else {
            Err(collisions)
        }
    }

    #[inline]
    pub fn routes<'a>(&'a self) -> impl Iterator<Item=&'a Route> + 'a {
        self.routes.values().flat_map(|v| v.iter())
    }

    // This is slow. Don't expose this publicly; only for tests.
    #[cfg(test)]
    fn has_collisions(&self) -> bool {
        for routes in self.routes.values() {
            for (i, a_route) in routes.iter().enumerate() {
                for b_route in routes.iter().skip(i + 1) {
                    if a_route.collides_with(b_route) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod test {
    use super::{Router, Route, dummy_handler};

    use crate::rocket::Rocket;
    use crate::config::Config;
    use crate::http::Method;
    use crate::http::Method::*;
    use crate::http::uri::Origin;
    use crate::request::Request;

    fn router_with_routes(routes: &[&'static str]) -> Router {
        let mut router = Router::new();
        for route in routes {
            let route = Route::new(Get, route.to_string(), dummy_handler);
            router.add(route);
        }

        router
    }

    fn router_with_ranked_routes(routes: &[(isize, &'static str)]) -> Router {
        let mut router = Router::new();
        for &(rank, route) in routes {
            let route = Route::ranked(rank, Get, route.to_string(), dummy_handler);
            router.add(route);
        }

        router
    }

    fn router_with_unranked_routes(routes: &[&'static str]) -> Router {
        let mut router = Router::new();
        for route in routes {
            let route = Route::ranked(0, Get, route.to_string(), dummy_handler);
            router.add(route);
        }

        router
    }

    fn unranked_route_collisions(routes: &[&'static str]) -> bool {
        let router = router_with_unranked_routes(routes);
        router.has_collisions()
    }

    fn default_rank_route_collisions(routes: &[&'static str]) -> bool {
        let router = router_with_routes(routes);
        router.has_collisions()
    }

    #[test]
    fn test_collisions() {
        assert!(unranked_route_collisions(&["/hello", "/hello"]));
        assert!(unranked_route_collisions(&["/<a>", "/hello"]));
        assert!(unranked_route_collisions(&["/<a>", "/<b>"]));
        assert!(unranked_route_collisions(&["/hello/bob", "/hello/<b>"]));
        assert!(unranked_route_collisions(&["/a/b/<c>/d", "/<a>/<b>/c/d"]));
        assert!(unranked_route_collisions(&["/a/b", "/<a..>"]));
        assert!(unranked_route_collisions(&["/a/b/c", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/<a>/b", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/a/<b>", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/a/b/<c>", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/<a..>", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/a/<a..>", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/a/b/<a..>", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/a/b/c/d", "/a/<a..>"]));
    }

    #[test]
    fn test_collisions_normalize() {
        assert!(unranked_route_collisions(&["/hello/", "/hello"]));
        assert!(unranked_route_collisions(&["//hello/", "/hello"]));
        assert!(unranked_route_collisions(&["//hello/", "/hello//"]));
        assert!(unranked_route_collisions(&["/<a>", "/hello//"]));
        assert!(unranked_route_collisions(&["/<a>", "/hello///"]));
        assert!(unranked_route_collisions(&["/hello///bob", "/hello/<b>"]));
        assert!(unranked_route_collisions(&["/<a..>//", "/a//<a..>"]));
        assert!(unranked_route_collisions(&["/a/<a..>//", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/a/<a..>//", "/a/b//c//d/"]));
        assert!(unranked_route_collisions(&["/a/<a..>/", "/a/bd/e/"]));
        assert!(unranked_route_collisions(&["/a/<a..>//", "/a/b//c//d/e/"]));
        assert!(unranked_route_collisions(&["/a//<a..>//", "/a/b//c//d/e/"]));
    }

    #[test]
    fn test_collisions_query() {
        // Query shouldn't affect things when unranked.
        assert!(unranked_route_collisions(&["/hello?<foo>", "/hello"]));
        assert!(unranked_route_collisions(&["/<a>?foo=bar", "/hello?foo=bar&cat=fat"]));
        assert!(unranked_route_collisions(&["/<a>?foo=bar", "/hello?foo=bar&cat=fat"]));
        assert!(unranked_route_collisions(&["/<a>", "/<b>?<foo>"]));
        assert!(unranked_route_collisions(&["/hello/bob?a=b", "/hello/<b>?d=e"]));
        assert!(unranked_route_collisions(&["/<foo>?a=b", "/foo?d=e"]));
        assert!(unranked_route_collisions(&["/<foo>?a=b&<c>", "/<foo>?d=e&<c>"]));
        assert!(unranked_route_collisions(&["/<foo>?a=b&<c>", "/<foo>?d=e"]));
    }

    #[test]
    fn test_no_collisions() {
        assert!(!unranked_route_collisions(&["/<a>", "/a/<a..>"]));
        assert!(!unranked_route_collisions(&["/a/b", "/a/b/c"]));
        assert!(!unranked_route_collisions(&["/a/b/c/d", "/a/b/c/<d>/e"]));
        assert!(!unranked_route_collisions(&["/a/d/<b..>", "/a/b/c"]));
        assert!(!unranked_route_collisions(&["/a/d/<b..>", "/a/d"]));
    }

    #[test]
    fn test_no_collision_when_ranked() {
        assert!(!default_rank_route_collisions(&["/<a>", "/hello"]));
        assert!(!default_rank_route_collisions(&["/hello/bob", "/hello/<b>"]));
        assert!(!default_rank_route_collisions(&["/a/b/c/d", "/<a>/<b>/c/d"]));
        assert!(!default_rank_route_collisions(&["/hi", "/<hi>"]));
        assert!(!default_rank_route_collisions(&["/hi", "/<hi>"]));
        assert!(!default_rank_route_collisions(&["/a/b", "/a/b/<c..>"]));
    }

    #[test]
    fn test_collision_when_ranked_query() {
        assert!(default_rank_route_collisions(&["/a?a=b", "/a?c=d"]));
        assert!(default_rank_route_collisions(&["/<foo>?a=b", "/<foo>?c=d&<d>"]));
    }

    #[test]
    fn test_no_collision_when_ranked_query() {
        assert!(!default_rank_route_collisions(&["/", "/?<c..>"]));
        assert!(!default_rank_route_collisions(&["/hi", "/hi?<c>"]));
        assert!(!default_rank_route_collisions(&["/hi", "/hi?c"]));
        assert!(!default_rank_route_collisions(&["/hi?<c>", "/hi?c"]));
    }

    fn route<'a>(router: &'a Router, method: Method, uri: &str) -> Option<&'a Route> {
        let rocket = Rocket::custom(Config::development());
        let request = Request::new(&rocket, method, Origin::parse(uri).unwrap());
        let matches = router.route(&request);
        if matches.len() > 0 {
            Some(matches[0])
        } else {
            None
        }
    }

    fn matches<'a>(router: &'a Router, method: Method, uri: &str) -> Vec<&'a Route> {
        let rocket = Rocket::custom(Config::development());
        let request = Request::new(&rocket, method, Origin::parse(uri).unwrap());
        router.route(&request)
    }

    #[test]
    fn test_ok_routing() {
        let router = router_with_routes(&["/hello"]);
        assert!(route(&router, Get, "/hello").is_some());

        let router = router_with_routes(&["/<a>"]);
        assert!(route(&router, Get, "/hello").is_some());
        assert!(route(&router, Get, "/hi").is_some());
        assert!(route(&router, Get, "/bobbbbbbbbbby").is_some());
        assert!(route(&router, Get, "/dsfhjasdf").is_some());

        let router = router_with_routes(&["/<a>/<b>"]);
        assert!(route(&router, Get, "/hello/hi").is_some());
        assert!(route(&router, Get, "/a/b/").is_some());
        assert!(route(&router, Get, "/i/a").is_some());
        assert!(route(&router, Get, "/jdlk/asdij").is_some());

        let mut router = Router::new();
        router.add(Route::new(Put, "/hello".to_string(), dummy_handler));
        router.add(Route::new(Post, "/hello".to_string(), dummy_handler));
        router.add(Route::new(Delete, "/hello".to_string(), dummy_handler));
        assert!(route(&router, Put, "/hello").is_some());
        assert!(route(&router, Post, "/hello").is_some());
        assert!(route(&router, Delete, "/hello").is_some());

        let router = router_with_routes(&["/<a..>"]);
        assert!(route(&router, Get, "/hello/hi").is_some());
        assert!(route(&router, Get, "/a/b/").is_some());
        assert!(route(&router, Get, "/i/a").is_some());
        assert!(route(&router, Get, "/a/b/c/d/e/f").is_some());
    }

    #[test]
    fn test_err_routing() {
        let router = router_with_routes(&["/hello"]);
        assert!(route(&router, Put, "/hello").is_none());
        assert!(route(&router, Post, "/hello").is_none());
        assert!(route(&router, Options, "/hello").is_none());
        assert!(route(&router, Get, "/hell").is_none());
        assert!(route(&router, Get, "/hi").is_none());
        assert!(route(&router, Get, "/hello/there").is_none());
        assert!(route(&router, Get, "/hello/i").is_none());
        assert!(route(&router, Get, "/hillo").is_none());

        let router = router_with_routes(&["/<a>"]);
        assert!(route(&router, Put, "/hello").is_none());
        assert!(route(&router, Post, "/hello").is_none());
        assert!(route(&router, Options, "/hello").is_none());
        assert!(route(&router, Get, "/hello/there").is_none());
        assert!(route(&router, Get, "/hello/i").is_none());

        let router = router_with_routes(&["/<a>/<b>"]);
        assert!(route(&router, Get, "/a/b/c").is_none());
        assert!(route(&router, Get, "/a").is_none());
        assert!(route(&router, Get, "/a/").is_none());
        assert!(route(&router, Get, "/a/b/c/d").is_none());
        assert!(route(&router, Put, "/hello/hi").is_none());
        assert!(route(&router, Put, "/a/b").is_none());
        assert!(route(&router, Put, "/a/b").is_none());
    }

    macro_rules! assert_ranked_routes {
        ($routes:expr, $to:expr, $want:expr) => ({
            let router = router_with_routes($routes);
            let route_path = route(&router, Get, $to).unwrap().uri.to_string();
            assert_eq!(route_path, $want.to_string());
        })
    }

    #[test]
    fn test_default_ranking() {
        assert_ranked_routes!(&["/hello", "/<name>"], "/hello", "/hello");
        assert_ranked_routes!(&["/<name>", "/hello"], "/hello", "/hello");
        assert_ranked_routes!(&["/<a>", "/hi", "/<b>"], "/hi", "/hi");
        assert_ranked_routes!(&["/<a>/b", "/hi/c"], "/hi/c", "/hi/c");
        assert_ranked_routes!(&["/<a>/<b>", "/hi/a"], "/hi/c", "/<a>/<b>");
        assert_ranked_routes!(&["/hi/a", "/hi/<c>"], "/hi/c", "/hi/<c>");
        assert_ranked_routes!(&["/a", "/a?<b>"], "/a?b=c", "/a?<b>");
        assert_ranked_routes!(&["/a", "/a?<b>"], "/a", "/a?<b>");
        assert_ranked_routes!(&["/a", "/<a>", "/a?<b>", "/<a>?<b>"], "/a", "/a?<b>");
        assert_ranked_routes!(&["/a", "/<a>", "/a?<b>", "/<a>?<b>"], "/b", "/<a>?<b>");
        assert_ranked_routes!(&["/a", "/<a>", "/a?<b>", "/<a>?<b>"], "/b?v=1", "/<a>?<b>");
        assert_ranked_routes!(&["/a", "/<a>", "/a?<b>", "/<a>?<b>"], "/a?b=c", "/a?<b>");
        assert_ranked_routes!(&["/a", "/a?b"], "/a?b", "/a?b");
        assert_ranked_routes!(&["/<a>", "/a?b"], "/a?b", "/a?b");
        assert_ranked_routes!(&["/a", "/<a>?b"], "/a?b", "/a");
        assert_ranked_routes!(&["/a?<c>&b", "/a?<b>"], "/a", "/a?<b>");
        assert_ranked_routes!(&["/a?<c>&b", "/a?<b>"], "/a?b", "/a?<c>&b");
        assert_ranked_routes!(&["/a?<c>&b", "/a?<b>"], "/a?c", "/a?<b>");
    }

    fn ranked_collisions(routes: &[(isize, &'static str)]) -> bool {
        let router = router_with_ranked_routes(routes);
        router.has_collisions()
    }

    #[test]
    fn test_no_manual_ranked_collisions() {
        assert!(!ranked_collisions(&[(1, "/a/<b>"), (2, "/a/<b>")]));
        assert!(!ranked_collisions(&[(0, "/a/<b>"), (2, "/a/<b>")]));
        assert!(!ranked_collisions(&[(5, "/a/<b>"), (2, "/a/<b>")]));
        assert!(!ranked_collisions(&[(1, "/a/<b>"), (1, "/b/<b>")]));
        assert!(!ranked_collisions(&[(1, "/a/<b..>"), (2, "/a/<b..>")]));
        assert!(!ranked_collisions(&[(0, "/a/<b..>"), (2, "/a/<b..>")]));
        assert!(!ranked_collisions(&[(5, "/a/<b..>"), (2, "/a/<b..>")]));
        assert!(!ranked_collisions(&[(1, "/<a..>"), (2, "/<a..>")]));
    }

    #[test]
    fn test_ranked_collisions() {
        assert!(ranked_collisions(&[(2, "/a/<b..>"), (2, "/a/<b..>")]));
        assert!(ranked_collisions(&[(2, "/a/c/<b..>"), (2, "/a/<b..>")]));
        assert!(ranked_collisions(&[(2, "/<b..>"), (2, "/a/<b..>")]));
    }

    macro_rules! assert_ranked_routing {
        (to: $to:expr, with: $routes:expr, expect: $($want:expr),+) => ({
            let router = router_with_ranked_routes(&$routes);
            let routed_to = matches(&router, Get, $to);
            let expected = &[$($want),+];
            assert!(routed_to.len() == expected.len());
            for (got, expected) in routed_to.iter().zip(expected.iter()) {
                assert_eq!(got.rank, expected.0);
                assert_eq!(got.uri.to_string(), expected.1.to_string());
            }
        })
    }

    #[test]
    fn test_ranked_routing() {
        assert_ranked_routing!(
            to: "/a/b",
            with: [(1, "/a/<b>"), (2, "/a/<b>")],
            expect: (1, "/a/<b>"), (2, "/a/<b>")
        );

        assert_ranked_routing!(
            to: "/b/b",
            with: [(1, "/a/<b>"), (2, "/b/<b>"), (3, "/b/b")],
            expect: (2, "/b/<b>"), (3, "/b/b")
        );

        assert_ranked_routing!(
            to: "/b/b",
            with: [(2, "/b/<b>"), (1, "/a/<b>"), (3, "/b/b")],
            expect: (2, "/b/<b>"), (3, "/b/b")
        );

        assert_ranked_routing!(
            to: "/b/b",
            with: [(3, "/b/b"), (2, "/b/<b>"), (1, "/a/<b>")],
            expect: (2, "/b/<b>"), (3, "/b/b")
        );

        assert_ranked_routing!(
            to: "/b/b",
            with: [(1, "/a/<b>"), (2, "/b/<b>"), (0, "/b/b")],
            expect: (0, "/b/b"), (2, "/b/<b>")
        );

        assert_ranked_routing!(
            to: "/profile/sergio/edit",
            with: [(1, "/<a>/<b>/edit"), (2, "/profile/<d>"), (0, "/<a>/<b>/<c>")],
            expect: (0, "/<a>/<b>/<c>"), (1, "/<a>/<b>/edit")
        );

        assert_ranked_routing!(
            to: "/profile/sergio/edit",
            with: [(0, "/<a>/<b>/edit"), (2, "/profile/<d>"), (5, "/<a>/<b>/<c>")],
            expect: (0, "/<a>/<b>/edit"), (5, "/<a>/<b>/<c>")
        );

        assert_ranked_routing!(
            to: "/a/b",
            with: [(0, "/a/b"), (1, "/a/<b..>")],
            expect: (0, "/a/b"), (1, "/a/<b..>")
        );

        assert_ranked_routing!(
            to: "/a/b/c/d/e/f",
            with: [(1, "/a/<b..>"), (2, "/a/b/<c..>")],
            expect: (1, "/a/<b..>"), (2, "/a/b/<c..>")
        );
    }

    macro_rules! assert_default_ranked_routing {
        (to: $to:expr, with: $routes:expr, expect: $($want:expr),+) => ({
            let router = router_with_routes(&$routes);
            let routed_to = matches(&router, Get, $to);
            let expected = &[$($want),+];
            assert!(routed_to.len() == expected.len());
            for (got, expected) in routed_to.iter().zip(expected.iter()) {
                assert_eq!(got.uri.to_string(), expected.to_string());
            }
        })
    }

    #[test]
    fn test_default_ranked_routing() {
        assert_default_ranked_routing!(
            to: "/a/b?v=1",
            with: ["/a/<b>", "/a/b"],
            expect: "/a/b", "/a/<b>"
        );

        assert_default_ranked_routing!(
            to: "/a/b?v=1",
            with: ["/a/<b>", "/a/b", "/a/b?<v>"],
            expect: "/a/b?<v>", "/a/b", "/a/<b>"
        );

        assert_default_ranked_routing!(
            to: "/a/b?v=1",
            with: ["/a/<b>", "/a/b", "/a/b?<v>", "/a/<b>?<v>"],
            expect: "/a/b?<v>", "/a/b", "/a/<b>?<v>", "/a/<b>"
        );

        assert_default_ranked_routing!(
            to: "/a/b",
            with: ["/a/<b>", "/a/b", "/a/b?<v>", "/a/<b>?<v>"],
            expect: "/a/b?<v>", "/a/b", "/a/<b>?<v>", "/a/<b>"
        );

        assert_default_ranked_routing!(
            to: "/a/b?c",
            with: ["/a/b", "/a/b?<c>", "/a/b?c", "/a/<b>?c", "/a/<b>?<c>", "/<a>/<b>"],
            expect: "/a/b?c", "/a/b?<c>", "/a/b", "/a/<b>?c", "/a/<b>?<c>", "/<a>/<b>"
        );
    }
}
