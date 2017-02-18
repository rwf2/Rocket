use super::Route;

use http::uri::URI;
use http::ContentType;
use request::Request;

/// The Collider trait is used to determine if two items that can be routed on
/// can match against a given request. That is, if two items `collide`, they
/// will both match against _some_ request.
pub trait Collider<T: ?Sized = Self> {
    fn collides_with(&self, other: &T) -> bool;
}

pub fn index_match_until(break_c: char,
                         a: &str,
                         b: &str,
                         dir: bool)
                         -> Option<(isize, isize)> {
    let (a_len, b_len) = (a.len() as isize, b.len() as isize);
    let (mut i, mut j, delta) = if dir {
        (0, 0, 1)
    } else {
        (a_len - 1, b_len - 1, -1)
    };

    let break_b = break_c as u8;
    while i >= 0 && j >= 0 && i < a_len && j < b_len {
        let (c1, c2) = (a.as_bytes()[i as usize], b.as_bytes()[j as usize]);
        if c1 == break_b || c2 == break_b {
            break;
        } else if c1 != c2 {
            return None;
        } else {
            i += delta;
            j += delta;
        }
    }

    Some((i, j))
}

fn do_match_until(break_c: char, a: &str, b: &str, dir: bool) -> bool {
    index_match_until(break_c, a, b, dir).is_some()
}

impl<'a> Collider<str> for &'a str {
    fn collides_with(&self, other: &str) -> bool {
        let (a, b) = (self, other);
        do_match_until('<', a, b, true) && do_match_until('>', a, b, false)
    }
}

impl<'a, 'b> Collider<URI<'b>> for URI<'a> {
    fn collides_with(&self, other: &URI<'b>) -> bool {
        if self.query().is_some() != other.query().is_some() {
            return false;
        }

        for (seg_a, seg_b) in self.segments().zip(other.segments()) {
            if seg_a.ends_with("..>") || seg_b.ends_with("..>") {
                return true;
            }

            if !seg_a.collides_with(seg_b) {
                return false;
            }
        }

        if self.segment_count() != other.segment_count() {
            return false;
        }

        true
    }
}

impl Collider for ContentType  {
    fn collides_with(&self, other: &ContentType) -> bool {
        let collide = |a, b| a == "*" || b == "*" || a == b;
        collide(&self.ttype, &other.ttype) && collide(&self.subtype, &other.subtype)
    }
}

// This implementation is used at initialization to check if two user routes
// collide before launching. Format collisions works like this:
//   * If route a specifies format, it only gets requests for that format.
//   * If a route doesn't specify format, it gets requests for any format.
impl Collider for Route {
    fn collides_with(&self, b: &Route) -> bool {
        self.method == b.method
            && self.rank == b.rank
            && self.path.collides_with(&b.path)
            && match (self.format.as_ref(), b.format.as_ref()) {
                (Some(ct_a), Some(ct_b)) => ct_a.collides_with(ct_b),
                (Some(_), None) => true,
                (None, Some(_)) => true,
                (None, None) => true
            }
    }
}

// This implementation is used at runtime to check if a given request is
// intended for this Route. Format collisions works like this:
//   * If route a specifies format, it only gets requests for that format.
//   * If a route doesn't specify format, it gets requests for any format.
impl<'r> Collider<Request<'r>> for Route {
    fn collides_with(&self, req: &Request<'r>) -> bool {
        self.method == req.method()
            && req.uri().collides_with(&self.path)
            // FIXME: On payload requests, check Content-Type, else Accept.
            && match (req.content_type().as_ref(), self.format.as_ref()) {
                (Some(ct_a), Some(ct_b)) => ct_a.collides_with(ct_b),
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => true
            }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::Collider;
    use request::Request;
    use data::Data;
    use handler::Outcome;
    use router::route::Route;
    use http::{Method, ContentType};
    use http::uri::URI;
    use http::Method::*;

    type SimpleRoute = (Method, &'static str);

    fn dummy_handler(_req: &Request, _: Data) -> Outcome<'static> {
        Outcome::of("hi")
    }

    fn m_collide(a: SimpleRoute, b: SimpleRoute) -> bool {
        let route_a = Route::new(a.0, a.1.to_string(), dummy_handler);
        route_a.collides_with(&Route::new(b.0, b.1.to_string(), dummy_handler))
    }

    fn unranked_collide(a: &'static str, b: &'static str) -> bool {
        let route_a = Route::ranked(0, Get, a.to_string(), dummy_handler);
        route_a.collides_with(&Route::ranked(0, Get, b.to_string(), dummy_handler))
    }

    fn s_s_collide(a: &'static str, b: &'static str) -> bool {
        URI::new(a).collides_with(&URI::new(b))
    }

    #[test]
    fn simple_collisions() {
        assert!(unranked_collide("a", "a"));
        assert!(unranked_collide("/a", "/a"));
        assert!(unranked_collide("/hello", "/hello"));
        assert!(unranked_collide("/hello", "/hello/"));
        assert!(unranked_collide("/hello/there/how/ar", "/hello/there/how/ar"));
        assert!(unranked_collide("/hello/there", "/hello/there/"));
    }

    #[test]
    fn simple_param_collisions() {
        assert!(unranked_collide("/hello/<name>", "/hello/<person>"));
        assert!(unranked_collide("/hello/<name>/hi", "/hello/<person>/hi"));
        assert!(unranked_collide("/hello/<name>/hi/there", "/hello/<person>/hi/there"));
        assert!(unranked_collide("/<name>/hi/there", "/<person>/hi/there"));
        assert!(unranked_collide("/<name>/hi/there", "/dude/<name>/there"));
        assert!(unranked_collide("/<name>/<a>/<b>", "/<a>/<b>/<c>"));
        assert!(unranked_collide("/<name>/<a>/<b>/", "/<a>/<b>/<c>/"));
        assert!(unranked_collide("/<a..>", "/hi"));
        assert!(unranked_collide("/<a..>", "/hi/hey"));
        assert!(unranked_collide("/<a..>", "/hi/hey/hayo"));
        assert!(unranked_collide("/a/<a..>", "/a/hi/hey/hayo"));
        assert!(unranked_collide("/a/<b>/<a..>", "/a/hi/hey/hayo"));
        assert!(unranked_collide("/a/<b>/<c>/<a..>", "/a/hi/hey/hayo"));
        assert!(unranked_collide("/<b>/<c>/<a..>", "/a/hi/hey/hayo"));
        assert!(unranked_collide("/<b>/<c>/hey/hayo", "/a/hi/hey/hayo"));
    }

    #[test]
    fn medium_param_collisions() {
        assert!(unranked_collide("/hello/<name>", "/hello/bob"));
        assert!(unranked_collide("/<name>", "//bob"));
    }

    #[test]
    fn hard_param_collisions() {
        assert!(unranked_collide("/<name>bob", "/<name>b"));
        assert!(unranked_collide("/a<b>c", "/abc"));
        assert!(unranked_collide("/a<b>c", "/azooc"));
        assert!(unranked_collide("/a<b>", "/a"));
        assert!(unranked_collide("/<b>", "/a"));
        assert!(unranked_collide("/<a>/<b>", "/a/b<c>"));
        assert!(unranked_collide("/<a>/bc<b>", "/a/b<c>"));
        assert!(unranked_collide("/<a>/bc<b>d", "/a/b<c>"));
        assert!(unranked_collide("/<a..>", "///a///"));
    }


    #[test]
    fn query_collisions() {
        assert!(unranked_collide("/?<a>", "/?<a>"));
        assert!(unranked_collide("/a/?<a>", "/a/?<a>"));
        assert!(unranked_collide("/a?<a>", "/a?<a>"));
        assert!(unranked_collide("/<r>?<a>", "/<r>?<a>"));
        assert!(unranked_collide("/a/b/c?<a>", "/a/b/c?<a>"));
        assert!(unranked_collide("/<a>/b/c?<d>", "/a/b/<c>?<d>"));
    }

    #[test]
    fn non_collisions() {
        assert!(!unranked_collide("/a", "/b"));
        assert!(!unranked_collide("/a/b", "/a"));
        assert!(!unranked_collide("/a/b", "/a/c"));
        assert!(!unranked_collide("/a/hello", "/a/c"));
        assert!(!unranked_collide("/hello", "/a/c"));
        assert!(!unranked_collide("/hello/there", "/hello/there/guy"));
        assert!(!unranked_collide("/b<a>/there", "/hi/there"));
        assert!(!unranked_collide("/<a>/<b>c", "/hi/person"));
        assert!(!unranked_collide("/<a>/<b>cd", "/hi/<a>e"));
        assert!(!unranked_collide("/a<a>/<b>", "/b<b>/<a>"));
        assert!(!unranked_collide("/a/<b>", "/b/<b>"));
        assert!(!unranked_collide("/a<a>/<b>", "/b/<b>"));
        assert!(!unranked_collide("/<a..>", "/"));
        assert!(!unranked_collide("/hi/<a..>", "/hi"));
        assert!(!unranked_collide("/hi/<a..>", "/hi/"));
        assert!(!unranked_collide("/<a..>", "//////"));
    }

    #[test]
    fn query_non_collisions() {
        assert!(!unranked_collide("/?<a>", "/"));
        assert!(!unranked_collide("/?<a>", "/hi"));
        assert!(!unranked_collide("/?<a>", "/a"));
        assert!(!unranked_collide("/a?<a>", "/a"));
        assert!(!unranked_collide("/a/b?<a>", "/a/b"));
        assert!(!unranked_collide("/a/b", "/a/b/?<c>"));
    }

    #[test]
    fn method_dependent_non_collisions() {
        assert!(!m_collide((Get, "/"), (Post, "/")));
        assert!(!m_collide((Post, "/"), (Put, "/")));
        assert!(!m_collide((Put, "/a"), (Put, "/")));
        assert!(!m_collide((Post, "/a"), (Put, "/")));
        assert!(!m_collide((Get, "/a"), (Put, "/")));
        assert!(!m_collide((Get, "/hello"), (Put, "/hello")));
    }

    #[test]
    fn test_str_non_collisions() {
        assert!(!s_s_collide("/a", "/b"));
        assert!(!s_s_collide("/a/b", "/a"));
        assert!(!s_s_collide("/a/b", "/a/c"));
        assert!(!s_s_collide("/a/hello", "/a/c"));
        assert!(!s_s_collide("/hello", "/a/c"));
        assert!(!s_s_collide("/hello/there", "/hello/there/guy"));
        assert!(!s_s_collide("/b<a>/there", "/hi/there"));
        assert!(!s_s_collide("/<a>/<b>c", "/hi/person"));
        assert!(!s_s_collide("/<a>/<b>cd", "/hi/<a>e"));
        assert!(!s_s_collide("/a<a>/<b>", "/b<b>/<a>"));
        assert!(!s_s_collide("/a/<b>", "/b/<b>"));
        assert!(!s_s_collide("/a<a>/<b>", "/b/<b>"));
        assert!(!s_s_collide("/a", "/b"));
        assert!(!s_s_collide("/a/b", "/a"));
        assert!(!s_s_collide("/a/b", "/a/c"));
        assert!(!s_s_collide("/a/hello", "/a/c"));
        assert!(!s_s_collide("/hello", "/a/c"));
        assert!(!s_s_collide("/hello/there", "/hello/there/guy"));
        assert!(!s_s_collide("/b<a>/there", "/hi/there"));
        assert!(!s_s_collide("/<a>/<b>c", "/hi/person"));
        assert!(!s_s_collide("/<a>/<b>cd", "/hi/<a>e"));
        assert!(!s_s_collide("/a<a>/<b>", "/b<b>/<a>"));
        assert!(!s_s_collide("/a/<b>", "/b/<b>"));
        assert!(!s_s_collide("/a<a>/<b>", "/b/<b>"));
        assert!(!s_s_collide("/a", "/b"));
        assert!(!s_s_collide("/a/b", "/a"));
        assert!(!s_s_collide("/a/b", "/a/c"));
        assert!(!s_s_collide("/a/hello", "/a/c"));
        assert!(!s_s_collide("/hello", "/a/c"));
        assert!(!s_s_collide("/hello/there", "/hello/there/guy"));
        assert!(!s_s_collide("/b<a>/there", "/hi/there"));
        assert!(!s_s_collide("/<a>/<b>c", "/hi/person"));
        assert!(!s_s_collide("/<a>/<b>cd", "/hi/<a>e"));
        assert!(!s_s_collide("/a<a>/<b>", "/b<b>/<a>"));
        assert!(!s_s_collide("/a/<b>", "/b/<b>"));
        assert!(!s_s_collide("/a<a>/<b>", "/b/<b>"));
        assert!(!s_s_collide("/<a..>", "/"));
        assert!(!s_s_collide("/hi/<a..>", "/hi/"));
        assert!(!s_s_collide("/a/hi/<a..>", "/a/hi/"));
    }

    fn ct_ct_collide(ct1: &str, ct2: &str) -> bool {
        let ct_a = ContentType::from_str(ct1).expect(ct1);
        let ct_b = ContentType::from_str(ct2).expect(ct2);
        ct_a.collides_with(&ct_b)
    }

    #[test]
    fn test_content_type_colliions() {
        assert!(ct_ct_collide("application/json", "application/json"));
        assert!(ct_ct_collide("*/json", "application/json"));
        assert!(ct_ct_collide("*/*", "application/json"));
        assert!(ct_ct_collide("application/*", "application/json"));
        assert!(ct_ct_collide("application/*", "*/json"));
        assert!(ct_ct_collide("something/random", "something/random"));

        assert!(!ct_ct_collide("text/*", "application/*"));
        assert!(!ct_ct_collide("*/text", "*/json"));
        assert!(!ct_ct_collide("*/text", "application/test"));
        assert!(!ct_ct_collide("something/random", "something_else/random"));
        assert!(!ct_ct_collide("something/random", "*/else"));
        assert!(!ct_ct_collide("*/random", "*/else"));
        assert!(!ct_ct_collide("something/*", "random/else"));
    }

    fn r_ct_ct_collide<S1, S2>(m1: Method, ct1: S1, m2: Method, ct2: S2) -> bool
        where S1: Into<Option<&'static str>>, S2: Into<Option<&'static str>>
    {
        let mut route_a = Route::new(m1, "/", dummy_handler);
        if let Some(ct_str) = ct1.into() {
            route_a.format = Some(ct_str.parse::<ContentType>().unwrap());
        }

        let mut route_b = Route::new(m2, "/", dummy_handler);
        if let Some(ct_str) = ct2.into() {
            route_b.format = Some(ct_str.parse::<ContentType>().unwrap());
        }

        route_a.collides_with(&route_b)
    }

    #[test]
    fn test_route_content_type_colliions() {
        assert!(r_ct_ct_collide(Get, "application/json", Get, "application/json"));
        assert!(r_ct_ct_collide(Get, "*/json", Get, "application/json"));
        assert!(r_ct_ct_collide(Get, "*/json", Get, "application/*"));
        assert!(r_ct_ct_collide(Get, "text/html", Get, "text/*"));
        assert!(r_ct_ct_collide(Get, "any/thing", Get, "*/*"));

        assert!(r_ct_ct_collide(Get, None, Get, "text/*"));
        assert!(r_ct_ct_collide(Get, None, Get, "text/html"));
        assert!(r_ct_ct_collide(Get, None, Get, "*/*"));
        assert!(r_ct_ct_collide(Get, "text/html", Get, None));
        assert!(r_ct_ct_collide(Get, "*/*", Get, None));
        assert!(r_ct_ct_collide(Get, "application/json", Get, None));

        assert!(!r_ct_ct_collide(Get, "text/html", Get, "application/*"));
        assert!(!r_ct_ct_collide(Get, "application/html", Get, "text/*"));
        assert!(!r_ct_ct_collide(Get, "*/json", Get, "text/html"));
        assert!(!r_ct_ct_collide(Get, "text/html", Get, "text/css"));
    }

    fn req_route_collide<S1, S2>(m1: Method, ct1: S1, m2: Method, ct2: S2) -> bool
        where S1: Into<Option<&'static str>>, S2: Into<Option<&'static str>>
    {
        let mut req = Request::new(m1, "/");
        if let Some(ct_str) = ct1.into() {
            req.replace_header(ct_str.parse::<ContentType>().unwrap());
        }

        let mut route = Route::new(m2, "/", dummy_handler);
        if let Some(ct_str) = ct2.into() {
            route.format = Some(ct_str.parse::<ContentType>().unwrap());
        }

        route.collides_with(&req)
    }

    #[test]
    fn test_req_route_ct_collisions() {
        assert!(req_route_collide(Get, "application/json", Get, "application/json"));
        assert!(req_route_collide(Get, "application/json", Get, "application/*"));
        assert!(req_route_collide(Get, "application/json", Get, "*/json"));
        assert!(req_route_collide(Get, "text/html", Get, "text/html"));
        assert!(req_route_collide(Get, "text/html", Get, "*/*"));

        assert!(req_route_collide(Get, "text/html", Get, None));
        assert!(req_route_collide(Get, None, Get, None));
        assert!(req_route_collide(Get, "application/json", Get, None));
        assert!(req_route_collide(Get, "x-custom/anything", Get, None));

        assert!(!req_route_collide(Get, "application/json", Get, "text/html"));
        assert!(!req_route_collide(Get, "application/json", Get, "text/*"));
        assert!(!req_route_collide(Get, "application/json", Get, "*/xml"));

        assert!(!req_route_collide(Get, None, Get, "text/html"));
        assert!(!req_route_collide(Get, None, Get, "*/*"));
        assert!(!req_route_collide(Get, None, Get, "application/json"));
    }
}
