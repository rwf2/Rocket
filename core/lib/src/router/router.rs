use std::ops::{Deref, DerefMut};
use std::collections::HashMap;

use crate::catcher::TypedError;
use crate::request::Request;
use crate::http::{Method, Status};
use crate::{Route, Catcher};
use crate::router::Collide;

#[derive(Debug)]
pub(crate) struct Router<T>(T);

#[derive(Debug, Default)]
pub struct Pending {
    pub routes: Vec<Route>,
    pub catchers: Vec<Catcher>,
}

#[derive(Debug, Default)]
pub struct Finalized {
    pub routes: Vec<Route>,
    pub catchers: Vec<Catcher>,
    route_map: HashMap<Method, Vec<usize>>,
    catcher_map: HashMap<Option<u16>, Vec<usize>>,
}

pub type Pair<T> = (T, T);

pub type Collisions = (Vec<Pair<Route>>, Vec<Pair<Catcher>>);

pub type Result<T, E = Collisions> = std::result::Result<T, E>;

impl Router<Pending> {
    pub fn new() -> Self {
        Router(Pending::default())
    }

    pub fn finalize(self) -> Result<Router<Finalized>, Collisions> {
        fn collisions<'a, T>(items: &'a [T]) -> impl Iterator<Item = (T, T)> + 'a
            where T: Collide + Clone + 'a,
        {
            items.iter()
                .enumerate()
                .flat_map(move |(i, a)| {
                    items.iter()
                        .skip(i + 1)
                        .filter(move |b| a.collides_with(b))
                        .map(move |b| (a.clone(), b.clone()))
                })
        }

        let route_collisions: Vec<_> = collisions(&self.routes).collect();
        let catcher_collisions: Vec<_> = collisions(&self.catchers).collect();

        if !route_collisions.is_empty() || !catcher_collisions.is_empty() {
            return Err((route_collisions, catcher_collisions))
        }

        // create the route map
        let mut route_map: HashMap<Method, Vec<usize>> = HashMap::new();
        for (i, route) in self.routes.iter().enumerate() {
            match route.method {
                Some(method) => route_map.entry(method).or_default().push(i),
                None => for method in Method::ALL_VARIANTS {
                    route_map.entry(*method).or_default().push(i);
                }
            }
        }

        // create the catcher map
        let mut catcher_map: HashMap<Option<u16>, Vec<usize>> = HashMap::new();
        for (i, catcher) in self.catchers.iter().enumerate() {
            catcher_map.entry(catcher.code).or_default().push(i);
        }

        // sort routes by rank
        for routes in route_map.values_mut() {
            routes.sort_by_key(|&i| &self.routes[i].rank);
        }

        // sort catchers by rank
        for catchers in catcher_map.values_mut() {
            catchers.sort_by_key(|&i| &self.catchers[i].rank);
        }

        Ok(Router(Finalized {
            routes: self.0.routes,
            catchers: self.0.catchers,
            route_map, catcher_map
        }))
    }
}

impl Router<Finalized> {
    #[track_caller]
    pub fn route<'r, 'a: 'r>(
        &'a self,
        req: &'r Request<'r>
    ) -> impl Iterator<Item = &'a Route> + 'r {
        // Note that routes are presorted by ascending rank on each `add` and
        // that all routes with `None` methods have been cloned into all methods.
        self.route_map.get(&req.method())
            .into_iter()
            .flat_map(move |routes| routes.iter().map(move |&i| &self.routes[i]))
            .filter(move |r| r.matches(req))
    }

    // For many catchers, using aho-corasick or similar should be much faster.
    #[track_caller]
    pub fn catch<'r>(
        &self,
        status: Status,
        error: Option<&'r dyn TypedError<'r>>,
        req: &'r Request<'r>
    ) -> Option<&Catcher> {
        let ty = error.map(|e| e.trait_obj_typeid());
        // Note that catchers are presorted by descending base length.
        self.catcher_map.get(&Some(status.code))
            .map(|catchers| catchers.iter().map(|&i| &self.catchers[i]))
            .and_then(|mut catchers| catchers.find(|c| c.matches(status, ty, req)))
    }

    #[track_caller]
    pub fn catch_any<'r>(
        &self,
        status: Status,
        error: Option<&'r dyn TypedError<'r>>,
        req: &'r Request<'r>
    ) -> Option<&Catcher> {
        let ty = error.map(|e| e.trait_obj_typeid());
        // Note that catchers are presorted by descending base length.
        self.catcher_map.get(&None)
            .map(|catchers| catchers.iter().map(|&i| &self.catchers[i]))
            .and_then(|mut catchers| catchers.find(|c| c.matches(status, ty, req)))
    }
}

impl<T> Deref for Router<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Router<Pending> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod test {
    use transient::TypeId;

    use super::*;

    use crate::catcher;
    use crate::route::dummy_handler;
    use crate::local::blocking::Client;
    use crate::http::{Method::*, uri::Origin};

    fn make_router<I>(routes: I) -> Result<Router<Finalized>, Collisions>
        where I: Iterator<Item = (Option<isize>, &'static str)>
    {
        let mut router = Router::new();
        for (rank, route) in routes {
            let route = Route::ranked(rank, Get, route, dummy_handler);
            router.routes.push(route);
        }

        router.finalize()
    }

    fn router_with_routes(routes: &[&'static str]) -> Router<Finalized> {
        make_router(routes.iter().map(|r| (None, *r))).unwrap()
    }

    fn router_with_ranked_routes(routes: &[(isize, &'static str)]) -> Router<Finalized> {
        make_router(routes.iter().map(|r| (Some(r.0), r.1))).unwrap()
    }

    fn rankless_route_collisions(routes: &[&'static str]) -> bool {
        make_router(routes.iter().map(|r| (Some(0), *r))).is_err()
    }

    fn default_rank_route_collisions(routes: &[&'static str]) -> bool {
        make_router(routes.iter().map(|r| (None, *r))).is_err()
    }

    #[test]
    fn test_rankless_collisions() {
        assert!(rankless_route_collisions(&["/hello", "/hello"]));
        assert!(rankless_route_collisions(&["/<a>", "/hello"]));
        assert!(rankless_route_collisions(&["/<a>", "/<b>"]));
        assert!(rankless_route_collisions(&["/hello/bob", "/hello/<b>"]));
        assert!(rankless_route_collisions(&["/a/b/<c>/d", "/<a>/<b>/c/d"]));

        assert!(rankless_route_collisions(&["/a/b", "/<a..>"]));
        assert!(rankless_route_collisions(&["/a/b/c", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/<a>/b", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/a/<b>", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/a/b/<c>", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/<a..>", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/a/<a..>", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/a/b/<a..>", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/a/b/c/d", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/", "/<a..>"]));
        assert!(rankless_route_collisions(&["/a/<_>", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/a/<_>", "/a/<_..>"]));
        assert!(rankless_route_collisions(&["/foo/bar/baz", "/foo/<_..>"]));

        assert!(rankless_route_collisions(&["/<_>", "/<_>"]));
        assert!(rankless_route_collisions(&["/a/<_>", "/a/b"]));
        assert!(rankless_route_collisions(&["/a/<_>", "/a/<b>"]));
        assert!(rankless_route_collisions(&["/<_..>", "/a/b"]));
        assert!(rankless_route_collisions(&["/<_..>", "/<_>"]));
        assert!(rankless_route_collisions(&["/<_>/b", "/a/b"]));
        assert!(rankless_route_collisions(&["/", "/<foo..>"]));
        assert!(rankless_route_collisions(&["/<_>", "/"]));
    }

    #[test]
    fn test_collisions_normalize() {
        assert!(rankless_route_collisions(&["//hello/", "/hello//"]));
        assert!(rankless_route_collisions(&["/hello///bob", "/hello/<b>"]));
        assert!(rankless_route_collisions(&["/a/<a..>//", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/a/<a..>//", "/a/b//c//d/"]));
        assert!(rankless_route_collisions(&["/<a..>//", "/a//<a..>"]));
        assert!(rankless_route_collisions(&["/a/<a..>/", "/a/bd/e/"]));
        assert!(rankless_route_collisions(&["/<a..>/", "/a/bd/e/"]));
        assert!(rankless_route_collisions(&["//", "/<foo..>"]));
        assert!(rankless_route_collisions(&["/a/<a..>//", "/a/b//c//d/e/"]));
        assert!(rankless_route_collisions(&["/a//<a..>//", "/a/b//c//d/e/"]));
        assert!(rankless_route_collisions(&["///<_>", "/<_>"]));
        assert!(rankless_route_collisions(&["/a/<_>", "///a//b"]));
        assert!(rankless_route_collisions(&["//a///<_>", "/a//<b>"]));
        assert!(rankless_route_collisions(&["//<_..>", "/a/b"]));
        assert!(rankless_route_collisions(&["//<_..>", "/<_>"]));
        assert!(rankless_route_collisions(&["///<a>/", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["///<a..>/", "/a/<a..>"]));
        assert!(rankless_route_collisions(&["/<a..>", "/hello"]));
    }

    #[test]
    fn test_collisions_query() {
        // Query shouldn't affect things when rankless.
        assert!(rankless_route_collisions(&["/hello?<foo>", "/hello"]));
        assert!(rankless_route_collisions(&["/<a>?foo=bar", "/hello?foo=bar&cat=fat"]));
        assert!(rankless_route_collisions(&["/<a>?foo=bar", "/hello?foo=bar&cat=fat"]));
        assert!(rankless_route_collisions(&["/<a>", "/<b>?<foo>"]));
        assert!(rankless_route_collisions(&["/hello/bob?a=b", "/hello/<b>?d=e"]));
        assert!(rankless_route_collisions(&["/<foo>?a=b", "/foo?d=e"]));
        assert!(rankless_route_collisions(&["/<foo>?a=b&<c>", "/<foo>?d=e&<c>"]));
        assert!(rankless_route_collisions(&["/<foo>?a=b&<c>", "/<foo>?d=e"]));
    }

    #[test]
    fn test_no_collisions() {
        assert!(!rankless_route_collisions(&["/a", "/a/"]));
        assert!(!rankless_route_collisions(&["/<a>", "/hello//"]));
        assert!(!rankless_route_collisions(&["/<a>", "/hello///"]));
        assert!(!rankless_route_collisions(&["/hello/", "/hello"]));
        assert!(!rankless_route_collisions(&["//hello/", "/hello"]));
        assert!(!rankless_route_collisions(&["/a/b", "/a/b/c"]));
        assert!(!rankless_route_collisions(&["/a/b/c/d", "/a/b/c/<d>/e"]));
        assert!(!rankless_route_collisions(&["/a/d/<b..>", "/a/b/c"]));
        assert!(!rankless_route_collisions(&["/a/<_>", "/a"]));
        assert!(!rankless_route_collisions(&["/a/<_>", "/<_>"]));
        assert!(!rankless_route_collisions(&["/a/<b>/<c..>", "/a/<c>"]));
        assert!(!rankless_route_collisions(&["/<_>", "/a/<_..>"]));
        assert!(!rankless_route_collisions(&["/foo", "/foo/<_..>"]));
        assert!(!rankless_route_collisions(&["/a/<_..>", "/<_>"]));
        assert!(!rankless_route_collisions(&["/a/<_..>", "/a"]));
        assert!(!rankless_route_collisions(&["/<a>", "/a/<a..>"]));
        assert!(!rankless_route_collisions(&["/a/d/<b..>", "/a/d"]));
    }

    #[test]
    fn test_no_collision_when_ranked() {
        assert!(!default_rank_route_collisions(&["/<_>", "/"]));
        assert!(!default_rank_route_collisions(&["/<a>", "/hello"]));
        assert!(!default_rank_route_collisions(&["/hello/bob", "/hello/<b>"]));
        assert!(!default_rank_route_collisions(&["/a/b/c/d", "/<a>/<b>/c/d"]));
        assert!(!default_rank_route_collisions(&["/hi", "/<hi>"]));
        assert!(!default_rank_route_collisions(&["/a", "/a/<path..>"]));
        assert!(!default_rank_route_collisions(&["/", "/<path..>"]));
        assert!(!default_rank_route_collisions(&["/a/b", "/a/b/<c..>"]));
        assert!(!default_rank_route_collisions(&["/<_>", "/static"]));
        assert!(!default_rank_route_collisions(&["/<_..>", "/static"]));
        assert!(!default_rank_route_collisions(&["/<path..>", "/"]));
        assert!(!default_rank_route_collisions(&["/<_>/<_>", "/foo/bar"]));
        assert!(!default_rank_route_collisions(&["/foo/<_>", "/foo/bar"]));

        assert!(!default_rank_route_collisions(&["/<a>/<b>", "/hello/<b>"]));
        assert!(!default_rank_route_collisions(&["/<a>/<b..>", "/hello/<b>"]));
        assert!(!default_rank_route_collisions(&["/<a..>", "/hello/<b>"]));
        assert!(!default_rank_route_collisions(&["/<a..>", "/hello"]));
        assert!(!default_rank_route_collisions(&["/<a>", "/a/<path..>"]));
        assert!(!default_rank_route_collisions(&["/a/<b>/c", "/<d>/<c..>"]));
        assert!(!default_rank_route_collisions(&["/a/<b>/<c..>", "/a/<c>"]));
    }

    #[test]
    fn test_collision_when_ranked() {
        assert!(default_rank_route_collisions(&["/<a>/b", "/a/<b>"]));
    }

    #[test]
    fn test_collision_when_ranked_query() {
        assert!(default_rank_route_collisions(&["/a?a=b", "/a?c=d"]));
        assert!(default_rank_route_collisions(&["/a?a=b&<b>", "/a?<c>&c=d"]));
        assert!(default_rank_route_collisions(&["/a?a=b&<b..>", "/a?<c>&c=d"]));
    }

    #[test]
    fn test_no_collision_when_ranked_query() {
        assert!(!default_rank_route_collisions(&["/", "/?<c..>"]));
        assert!(!default_rank_route_collisions(&["/hi", "/hi?<c>"]));
        assert!(!default_rank_route_collisions(&["/hi", "/hi?c"]));
        assert!(!default_rank_route_collisions(&["/hi?<c>", "/hi?c"]));
        assert!(!default_rank_route_collisions(&["/<foo>?a=b", "/<foo>?c=d&<d>"]));
    }

    #[track_caller]
    fn matches<'a>(router: &'a Router<Finalized>, method: Method, uri: &'a str) -> Vec<&'a Route> {
        let client = Client::debug_with(vec![]).expect("client");
        let request = client.req(method, Origin::parse(uri).unwrap());
        router.route(&request).collect()
    }

    #[track_caller]
    fn route<'a>(router: &'a Router<Finalized>, method: Method, uri: &'a str) -> Option<&'a Route> {
        matches(router, method, uri).into_iter().next()
    }

    #[test]
    fn test_ok_routing() {
        let router = router_with_routes(&["/hello"]);
        assert!(route(&router, Get, "/hello").is_some());

        let router = router_with_routes(&["/<a>"]);
        assert!(route(&router, Get, "/").is_some());
        assert!(route(&router, Get, "/hello").is_some());
        assert!(route(&router, Get, "/hi").is_some());
        assert!(route(&router, Get, "/bobbbbbbbbbby").is_some());
        assert!(route(&router, Get, "/dsfhjasdf").is_some());

        let router = router_with_routes(&["/<a>/<b>"]);
        assert!(route(&router, Get, "/hello/hi").is_some());
        assert!(route(&router, Get, "/i/a").is_some());
        assert!(route(&router, Get, "/jdlk/asdij").is_some());
        assert!(route(&router, Get, "/a/").is_some());

        let mut router = Router::new();
        router.routes.push(Route::new(Put, "/hello", dummy_handler));
        router.routes.push(Route::new(Post, "/hello", dummy_handler));
        router.routes.push(Route::new(Delete, "/hello", dummy_handler));
        let router = router.finalize().unwrap();
        assert!(route(&router, Put, "/hello").is_some());
        assert!(route(&router, Post, "/hello").is_some());
        assert!(route(&router, Delete, "/hello").is_some());

        let router = router_with_routes(&["/<a..>"]);
        assert!(route(&router, Get, "/").is_some());
        assert!(route(&router, Get, "//").is_some());
        assert!(route(&router, Get, "/hi").is_some());
        assert!(route(&router, Get, "/hello/hi").is_some());
        assert!(route(&router, Get, "/a/b/").is_some());
        assert!(route(&router, Get, "/i/a").is_some());
        assert!(route(&router, Get, "/a/b/c/d/e/f").is_some());

        let router = router_with_routes(&["/foo/<a..>"]);
        assert!(route(&router, Get, "/foo").is_none());
        assert!(route(&router, Get, "/foo/").is_some());
        assert!(route(&router, Get, "/foo///bar").is_some());
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
        assert!(route(&router, Get, "/hello/").is_none());
        assert!(route(&router, Get, "/hello/there/").is_none());
        assert!(route(&router, Get, "/hello/there/").is_none());

        let router = router_with_routes(&["/<a>/<b>"]);
        assert!(route(&router, Get, "/a/b/c").is_none());
        assert!(route(&router, Get, "/a").is_none());
        assert!(route(&router, Get, "/a/b/c/d").is_none());
        assert!(route(&router, Get, "/a/b/").is_none());
        assert!(route(&router, Put, "/hello/hi").is_none());
        assert!(route(&router, Put, "/a/b").is_none());

        let router = router_with_routes(&["/prefix/<a..>"]);
        assert!(route(&router, Get, "/").is_none());
        assert!(route(&router, Get, "/prefi/").is_none());
    }

    /// Asserts that `$to` routes to `$want` given `$routes` are present.
    macro_rules! assert_ranked_match {
        ($routes:expr, $to:expr => $want:expr) => ({
            let router = router_with_routes($routes);
            let route_path = route(&router, Get, $to).unwrap().uri.to_string();
            assert_eq!(route_path, $want.to_string(),
                "\nmatched {} with {}, wanted {} in {:#?}", $to, route_path, $want, router);
        })
    }

    #[test]
    fn test_default_ranking() {
        assert_ranked_match!(&["/hello", "/<name>"], "/hello" => "/hello");
        assert_ranked_match!(&["/<name>", "/hello"], "/hello" => "/hello");
        assert_ranked_match!(&["/<a>", "/hi", "/hi/<b>"], "/hi" => "/hi");
        assert_ranked_match!(&["/<a>/b", "/hi/c"], "/hi/c" => "/hi/c");
        assert_ranked_match!(&["/<a>/<b>", "/hi/a"], "/hi/c" => "/<a>/<b>");
        assert_ranked_match!(&["/hi/a", "/hi/<c>"], "/hi/c" => "/hi/<c>");
        assert_ranked_match!(&["/a", "/a?<b>"], "/a?b=c" => "/a?<b>");
        assert_ranked_match!(&["/a", "/a?<b>"], "/a" => "/a?<b>");
        assert_ranked_match!(&["/a", "/<a>", "/a?<b>", "/<a>?<b>"], "/a" => "/a?<b>");
        assert_ranked_match!(&["/a", "/<a>", "/a?<b>", "/<a>?<b>"], "/b" => "/<a>?<b>");
        assert_ranked_match!(&["/a", "/<a>", "/a?<b>", "/<a>?<b>"], "/b?v=1" => "/<a>?<b>");
        assert_ranked_match!(&["/a", "/<a>", "/a?<b>", "/<a>?<b>"], "/a?b=c" => "/a?<b>");
        assert_ranked_match!(&["/a", "/a?b"], "/a?b" => "/a?b");
        assert_ranked_match!(&["/<a>", "/a?b"], "/a?b" => "/a?b");
        assert_ranked_match!(&["/a", "/<a>?b"], "/a?b" => "/a");
        assert_ranked_match!(&["/a?<c>&b", "/a?<b>"], "/a" => "/a?<b>");
        assert_ranked_match!(&["/a?<c>&b", "/a?<b>"], "/a?b" => "/a?<c>&b");
        assert_ranked_match!(&["/a?<c>&b", "/a?<b>"], "/a?c" => "/a?<b>");
        assert_ranked_match!(&["/", "/<foo..>"], "/" => "/");
        assert_ranked_match!(&["/", "/<foo..>"], "/hi" => "/<foo..>");
        assert_ranked_match!(&["/hi", "/<foo..>"], "/hi" => "/hi");
    }

    fn ranked_collisions(routes: &[(isize, &'static str)]) -> bool {
        make_router(routes.iter().map(|r| (Some(r.0), r.1))).is_err()
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
            assert_eq!(routed_to.len(), expected.len());
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

        assert_ranked_routing!(
            to: "/hi/",
            with: [(1, "/hi/<foo..>"), (0, "/hi/<foo>")],
            expect: (0, "/hi/<foo>"), (1, "/hi/<foo..>")
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

    fn make_router_catches<I>(catchers: I) -> Result<Router<Finalized>, Collisions>
        where I: IntoIterator<Item = (Option<Status>, &'static str, Option<TypeId>)>
    {
        let mut router = Router::new();
        for (status, base, ty) in catchers {
            let mut catcher = Catcher::new(status.map(|s| s.code), catcher::dummy_handler)
                .rebase(Origin::parse(base).unwrap());
            catcher.type_info = ty.map(|t| ("", t));
            router.catchers.push(catcher);
        }

        router.finalize()
    }

    #[test]
    fn test_catcher_collisions() {
        #[derive(transient::Transient)]
        struct A;
        assert!(make_router_catches([
            (Some(Status::new(400)), "/", None),
            (Some(Status::new(400)), "/", None)
        ]).is_err());
        assert!(make_router_catches([
            (Some(Status::new(400)), "/", Some(TypeId::of::<()>())),
            (Some(Status::new(400)), "/", None)
        ]).is_ok());
        assert!(make_router_catches([
            (Some(Status::new(400)), "/", Some(TypeId::of::<()>())),
            (Some(Status::new(400)), "/", Some(TypeId::of::<()>())),
        ]).is_err());
        assert!(make_router_catches([
            (Some(Status::new(400)), "/", Some(TypeId::of::<()>())),
            (Some(Status::new(400)), "/", Some(TypeId::of::<A>())),
        ]).is_ok());
    }

    #[track_caller]
    fn catches<'a>(
        router: &'a Router<Finalized>,
        status: Status,
        uri: &'a str,
        ty: for<'r> fn(&'r ()) -> Option<&'r dyn TypedError<'r>>
    ) -> Option<&'a Catcher> {
        let client = Client::debug_with(vec![]).expect("client");
        let request = client.req(Method::Get, Origin::parse(uri).unwrap());
        router.catch(status, ty(&()), &request)
    }

    #[track_caller]
    fn catches_any<'a>(
        router: &'a Router<Finalized>,
        status: Status,
        uri: &'a str,
        ty: for<'r> fn(&'r ()) -> Option<&'r dyn TypedError<'r>>
    ) -> Option<&'a Catcher> {
        let client = Client::debug_with(vec![]).expect("client");
        let request = client.req(Method::Get, Origin::parse(uri).unwrap());
        router.catch_any(status, ty(&()), &request)
    }

    #[test]
    fn test_catch_vs_catch_any() {
        let router = make_router_catches([(None, "/", None)]).unwrap();

        assert!(catches(&router, Status::BadRequest, "/", |_| None).is_none());
        assert!(catches_any(&router, Status::BadRequest, "/", |_| None).is_some());
    }

    #[test]
    fn test_catch_vs_catch_any_ty() {
        let router = make_router_catches([
            (None, "/", None),
            (None, "/", Some(TypeId::of::<()>()))
        ]).unwrap();

        assert!(
            catches_any(&router, Status::BadRequest, "/", |_| None).unwrap().type_id().is_none()
        );
        assert!(
            catches_any(&router, Status::BadRequest, "/", |_| Some(&()))
                .unwrap()
                .type_id()
                .is_some()
        );
    }
}
