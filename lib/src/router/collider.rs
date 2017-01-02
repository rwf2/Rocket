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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use router::Collider;
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

    fn ct_route(m: Method, s: &str, ct: &str) -> Route {
        let mut route_a = Route::new(m, s, dummy_handler);
        route_a.format = ContentType::from_str(ct).expect("Whoops!");
        route_a
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

    fn r_ct_ct_collide(m1: Method, ct1: &str, m2: Method, ct2: &str) -> bool {
        let a_route = ct_route(m1, "a", ct1);
        let b_route = ct_route(m2, "a", ct2);
        a_route.collides_with(&b_route)
    }

    #[test]
    fn test_route_content_type_colliions() {
        assert!(r_ct_ct_collide(Get, "application/json", Get, "application/json"));
        assert!(r_ct_ct_collide(Get, "*/json", Get, "application/json"));
        assert!(r_ct_ct_collide(Get, "*/json", Get, "application/*"));
        assert!(r_ct_ct_collide(Get, "text/html", Get, "text/*"));

        assert!(!r_ct_ct_collide(Get, "text/html", Get, "application/*"));
        assert!(!r_ct_ct_collide(Get, "application/html", Get, "text/*"));
        assert!(!r_ct_ct_collide(Get, "*/json", Get, "text/html"));
    }
}
