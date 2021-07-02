#![allow(dead_code, unused_variables)]

#[macro_use] extern crate rocket;

use std::path::PathBuf;

use rocket::http::CookieJar;
use rocket::http::uri::fmt::{FromUriParam, Query};
use rocket::form::{Form, error::{Errors, ErrorKind}};

macro_rules! assert_uri_eq {
    ($($uri:expr => $expected:expr,)+) => {
        $(
            let actual = $uri;
            let expected = rocket::http::uri::Uri::parse_any($expected).expect("valid URI");
            if actual != expected {
                panic!("URI mismatch: got {}, expected {}\nGot) {:?}\nExpected) {:?}",
                    actual, expected, actual, expected);
            }
        )+
    };
}

#[derive(FromForm, UriDisplayQuery)]
struct User<'a> {
    name: &'a str,
    nickname: String,
}

impl<'a, 'b> FromUriParam<Query, (&'a str, &'b str)> for User<'a> {
    type Target = User<'a>;
    fn from_uri_param((name, nickname): (&'a str, &'b str)) -> User<'a> {
        User { name: name.into(), nickname: nickname.to_string() }
    }
}

// This one has no `UriDisplay`. It exists to ensure that this file still
// compiles even though it's used a URI parameter's type. As long as a user
// doesn't request a URI from that route, things should be okay.
#[derive(FromForm)]
struct Second {
    nickname: String,
}

#[post("/")]
fn index() { }

#[post("/<id>")]
fn simple(id: i32) { }

#[post("/<id>/<name>")]
fn simple2(id: i32, name: String) { }

#[post("/<id>/<name>")]
fn simple2_flipped(name: String, id: i32) { }

#[post("/?<id>")]
fn simple3(id: i32) { }

#[post("/?<id>&<name>")]
fn simple4(id: i32, name: String) { }

#[post("/?<id>&<name>")]
fn simple4_flipped(name: String, id: i32) { }

#[post("/<used>/<_unused>")]
fn unused_param(used: i32, _unused: i32) { }

#[post("/<id>")]
fn guard_1(cookies: &CookieJar<'_>, id: i32) { }

#[post("/<id>/<name>")]
fn guard_2(name: String, cookies: &CookieJar<'_>, id: i32) { }

#[post("/a/<id>/hi/<name>/hey")]
fn guard_3(id: i32, name: String, cookies: &CookieJar<'_>) { }

#[post("/<id>", data = "<form>")]
fn no_uri_display_okay(id: i32, form: Form<Second>) { }

#[post("/name/<name>?<foo>&type=10&<type>&<query..>", data = "<user>", rank = 2)]
fn complex<'r>(
    foo: usize,
    name: &str,
    query: User<'r>,
    user: Form<User<'r>>,
    r#type: &str,
    cookies: &CookieJar<'_>
) {  }

#[post("/a/<path..>")]
fn segments(path: PathBuf) { }

#[post("/a/<id>/then/<path..>")]
fn param_and_segments(path: PathBuf, id: usize) { }

#[post("/a/<id>/then/<path..>")]
fn guarded_segments(cookies: &CookieJar<'_>, path: PathBuf, id: usize) { }

#[test]
fn check_simple_unnamed() {
    assert_uri_eq! {
        uri!(simple(100)) => "/100",
        uri!(simple(-23)) => "/-23",
        uri!(unused_param(1, 2)) => "/1/2",
    }

    // The "flipped" test ensures that the order of parameters depends on the
    // route's URI, not on the order in the function signature.
    assert_uri_eq! {
        uri!(simple2(100, "hello".to_string())) => "/100/hello",
        uri!(simple2(1349, "hey".to_string())) => "/1349/hey",
        uri!(simple2_flipped(100, "hello".to_string())) => "/100/hello",
    }

    // Ensure that `.from_uri_param()` is called.
    assert_uri_eq! {
        uri!(simple2(100, "hello")) => "/100/hello",
        uri!(simple2_flipped(1349, "hey")) => "/1349/hey",
    }

    // Ensure that the `UriDisplay` trait is being used.
    assert_uri_eq! {
        uri!(simple2(100, "hello there")) => "/100/hello%20there",
        uri!(simple2_flipped(100, "hello there")) => "/100/hello%20there",
    }

    // Ensure that query parameters are handled properly.
    assert_uri_eq! {
        uri!(simple3(100)) => "/?id=100",
        uri!(simple3(1349)) => "/?id=1349",
        uri!(simple4(100, "bob")) => "/?id=100&name=bob",
        uri!(simple4(1349, "Bob Anderson")) => "/?id=1349&name=Bob%20Anderson",
        uri!(simple4(-2, "@M+s&OU=")) => "/?id=-2&name=@M%2Bs%26OU%3D",
        uri!(simple4_flipped(100, "bob")) => "/?id=100&name=bob",
        uri!(simple4_flipped(1349, "Bob Anderson")) => "/?id=1349&name=Bob%20Anderson",
    }
}

#[test]
fn check_simple_named() {
    assert_uri_eq! {
        uri!(simple(id = 100)) => "/100",
        uri!(simple(id = -23)) => "/-23",
        uri!(unused_param(used = 1, _unused = 2)) => "/1/2",
    }

    assert_uri_eq! {
        uri!(simple2(id = 100, name = "hello".to_string())) => "/100/hello",
        uri!(simple2(name = "hi".to_string(), id = 123)) => "/123/hi",
        uri!(simple2_flipped(id = 1349, name = "hey".to_string())) => "/1349/hey",
        uri!(simple2_flipped(name = "hello".to_string(), id = 100)) => "/100/hello",
    }

    // Ensure that `.from_uri_param()` is called.
    assert_uri_eq! {
        uri!(simple2(id = 100, name = "hello")) => "/100/hello",
        uri!(simple2(id = 100, name = "hi")) => "/100/hi",
        uri!(simple2(id = 1349, name = "hey")) => "/1349/hey",
        uri!(simple2(name = "hello", id = 100)) => "/100/hello",
        uri!(simple2(name = "hi", id = 100)) => "/100/hi",
        uri!(simple2_flipped(id = 1349, name = "hey")) => "/1349/hey",
    }

    // Ensure that the `UriDisplay` trait is being used.
    assert_uri_eq! {
        uri!(simple2(id = 100, name = "hello there")) => "/100/hello%20there",
        uri!(simple2(name = "hello there", id = 100)) => "/100/hello%20there",
        uri!(simple2_flipped(id = 100, name = "hello there")) => "/100/hello%20there",
        uri!(simple2_flipped(name = "hello there", id = 100)) => "/100/hello%20there",
    }

    // Ensure that query parameters are handled properly.
    assert_uri_eq! {
        uri!(simple3(id = 100)) => "/?id=100",
        uri!(simple3(id = 1349)) => "/?id=1349",
        uri!(simple4(id = 100, name = "bob")) => "/?id=100&name=bob",
        uri!(simple4(id = 1349, name = "Bob A")) => "/?id=1349&name=Bob%20A",
        uri!(simple4(name = "Bob A", id = 1349)) => "/?id=1349&name=Bob%20A",
        uri!(simple4_flipped(id = 1349, name = "Bob A")) => "/?id=1349&name=Bob%20A",
        uri!(simple4_flipped(name = "Bob A", id = 1349)) => "/?id=1349&name=Bob%20A",
    }
}

#[test]
fn check_route_prefix_suffix() {
    assert_uri_eq! {
        uri!(index) => "/",
        uri!("/", index) => "/",
        uri!("/hi", index) => "/hi",
        uri!("/", simple3(10)) => "/?id=10",
        uri!("/hi", simple3(11)) => "/hi?id=11",
        uri!("/mount", simple(100)) => "/mount/100",
        uri!("/mount", simple(id = 23)) => "/mount/23",
        uri!("/another", simple(100)) => "/another/100",
        uri!("/another", simple(id = 23)) => "/another/23",
    }

    assert_uri_eq! {
        uri!("http://rocket.rs", index) => "http://rocket.rs",
        uri!("http://rocket.rs/", index) => "http://rocket.rs",
        uri!("http://rocket.rs", index) => "http://rocket.rs",
        uri!("http://", index) => "http://",
        uri!("ftp:", index) => "ftp:/",
    }

    assert_uri_eq! {
        uri!("http://rocket.rs", index, "?foo") => "http://rocket.rs?foo",
        uri!("http://rocket.rs/", index, "#bar") => "http://rocket.rs#bar",
        uri!("http://rocket.rs", index, "?bar#baz") => "http://rocket.rs?bar#baz",
        uri!("http://rocket.rs/", index, "?bar#baz") => "http://rocket.rs?bar#baz",
        uri!("http://", index, "?foo") => "http://?foo",
        uri!("http://rocket.rs", simple3(id = 100), "?foo") => "http://rocket.rs?id=100",
        uri!("http://rocket.rs", simple3(id = 100), "?foo#bar") => "http://rocket.rs?id=100#bar",
        uri!(_, simple3(id = 100), "?foo#bar") => "/?id=100#bar",
    }

    let dyn_origin = uri!("/a/b/c");
    let dyn_origin2 = uri!("/a/b/c?foo-bar");
    assert_uri_eq! {
        uri!(dyn_origin.clone(), index) => "/a/b/c",
        uri!(dyn_origin2.clone(), index) => "/a/b/c",
        uri!(dyn_origin.clone(), simple3(10)) => "/a/b/c?id=10",
        uri!(dyn_origin2.clone(), simple3(10)) => "/a/b/c?id=10",
        uri!(dyn_origin.clone(), simple(100)) => "/a/b/c/100",
        uri!(dyn_origin2.clone(), simple(100)) => "/a/b/c/100",
        uri!(dyn_origin.clone(), simple2(100, "hey")) => "/a/b/c/100/hey",
        uri!(dyn_origin2.clone(), simple2(100, "hey")) => "/a/b/c/100/hey",
        uri!(dyn_origin.clone(), simple2(id = 23, name = "hey")) => "/a/b/c/23/hey",
        uri!(dyn_origin2.clone(), simple2(id = 23, name = "hey")) => "/a/b/c/23/hey",
    }

    let dyn_absolute = uri!("http://rocket.rs");
    assert_uri_eq! {
        uri!(dyn_absolute.clone(), index) => "http://rocket.rs",
        uri!(uri!("http://rocket.rs/a/b"), index) => "http://rocket.rs/a/b",
    }

    let dyn_abs = uri!("http://rocket.rs?foo");
    assert_uri_eq! {
        uri!(_, index, dyn_abs.clone()) => "/?foo",
        uri!("http://rocket.rs/", index, dyn_abs.clone()) => "http://rocket.rs?foo",
        uri!("http://rocket.rs", index, dyn_abs.clone()) => "http://rocket.rs?foo",
        uri!("http://", index, dyn_abs.clone()) => "http://?foo",
        uri!(_, simple3(id = 123), dyn_abs) => "/?id=123",
    }

    let dyn_ref = uri!("?foo#bar");
    assert_uri_eq! {
        uri!(_, index, dyn_ref.clone()) => "/?foo#bar",
        uri!("http://rocket.rs/", index, dyn_ref.clone()) => "http://rocket.rs?foo#bar",
        uri!("http://rocket.rs", index, dyn_ref.clone()) => "http://rocket.rs?foo#bar",
        uri!("http://", index, dyn_ref.clone()) => "http://?foo#bar",
        uri!(_, simple3(id = 123), dyn_ref) => "/?id=123#bar",
    }
}

#[test]
fn check_guards_ignored() {
    assert_uri_eq! {
        uri!("/mount", guard_1(100)) => "/mount/100",
        uri!("/mount", guard_2(2938, "boo")) => "/mount/2938/boo",
        uri!("/mount", guard_3(340, "Bob")) => "/mount/a/340/hi/Bob/hey",
        uri!(guard_1(100)) => "/100",
        uri!(guard_2(2938, "boo")) => "/2938/boo",
        uri!(guard_3(340, "Bob")) => "/a/340/hi/Bob/hey",
        uri!("/mount", guard_1(id = 100)) => "/mount/100",
        uri!("/mount", guard_2(id = 2938, name = "boo")) => "/mount/2938/boo",
        uri!("/mount", guard_3(id = 340, name = "Bob")) => "/mount/a/340/hi/Bob/hey",
        uri!(guard_1(id = 100)) => "/100",
        uri!(guard_2(name = "boo", id = 2938)) => "/2938/boo",
        uri!(guard_3(name = "Bob", id = 340)) => "/a/340/hi/Bob/hey",
    }
}

#[test]
fn check_with_segments() {
    assert_uri_eq! {
        uri!(segments(PathBuf::from("one/two/three"))) => "/a/one/two/three",
        uri!(segments(path = PathBuf::from("one/two/three"))) => "/a/one/two/three",
        uri!("/c", segments(PathBuf::from("one/tw o/"))) => "/c/a/one/tw%20o",
        uri!("/c", segments(path = PathBuf::from("one/tw o/"))) => "/c/a/one/tw%20o",
        uri!(segments(PathBuf::from("one/ tw?o/"))) => "/a/one/%20tw%3Fo",
        uri!(param_and_segments(10, PathBuf::from("a/b"))) => "/a/10/then/a/b",
        uri!(param_and_segments(id = 10, path = PathBuf::from("a/b"))) => "/a/10/then/a/b",
        uri!(guarded_segments(10, PathBuf::from("a/b"))) => "/a/10/then/a/b",
        uri!(guarded_segments(id = 10, path = PathBuf::from("a/b"))) => "/a/10/then/a/b",
    }

    // Now check the `from_uri_param()` conversions for `PathBuf`.
    assert_uri_eq! {
        uri!(segments("one/two/three")) => "/a/one/two/three",
        uri!("/", segments(path = "one/two/three")) => "/a/one/two/three",
        uri!("/oh", segments(path = "one/two/three")) => "/oh/a/one/two/three",
        uri!(segments("one/ tw?o/")) => "/a/one/%20tw%3Fo",
        uri!(param_and_segments(id = 10, path = "a/b")) => "/a/10/then/a/b",
        uri!(guarded_segments(10, "a/b")) => "/a/10/then/a/b",
        uri!(guarded_segments(id = 10, path = "a/b")) => "/a/10/then/a/b",
    }
}

#[test]
fn check_complex() {
    assert_uri_eq! {
        uri!(complex("no idea", 10, "high", ("A B C", "a c"))) =>
            "/name/no%20idea?foo=10&type=10&type=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex("Bob", 248, "?", User { name: "Robert".into(), nickname: "Bob".into() })) =>
            "/name/Bob?foo=248&type=10&type=%3F&name=Robert&nickname=Bob",
        uri!(complex("Bob", 248, "a a", &User { name: "Robert".into(), nickname: "B".into() })) =>
            "/name/Bob?foo=248&type=10&type=a%20a&name=Robert&nickname=B",
        uri!(complex("no idea", 248, "", &User { name: "A B".into(), nickname: "A".into() })) =>
            "/name/no%20idea?foo=248&type=10&type=&name=A%20B&nickname=A",
        uri!(complex("hi", 3, "b", &User { name: "A B C".into(), nickname: "a b".into() })) =>
            "/name/hi?foo=3&type=10&type=b&name=A%20B%20C&nickname=a%20b",
        uri!(complex(name = "no idea", foo = 10, r#type = "high", query = ("A B C", "a c"))) =>
            "/name/no%20idea?foo=10&type=10&type=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex(foo = 10, name = "no idea", r#type = "high", query = ("A B C", "a c"))) =>
            "/name/no%20idea?foo=10&type=10&type=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex(query = ("A B C", "a c"), foo = 10, name = "no idea", r#type = "high", )) =>
            "/name/no%20idea?foo=10&type=10&type=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex(query = ("A B C", "a c"), foo = 10, name = "no idea", r#type = "high")) =>
            "/name/no%20idea?foo=10&type=10&type=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex(query = *&("A B C", "a c"), foo = 10, name = "no idea", r#type = "high")) =>
            "/name/no%20idea?foo=10&type=10&type=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex(foo = 3, name = "hi", r#type = "b",
                query = &User { name: "A B C".into(), nickname: "a b".into() })) =>
                "/name/hi?foo=3&type=10&type=b&name=A%20B%20C&nickname=a%20b",
        uri!(complex(query = &User { name: "A B C".into(), nickname: "a b".into() },
                foo = 3, name = "hi", r#type = "b")) =>
                "/name/hi?foo=3&type=10&type=b&name=A%20B%20C&nickname=a%20b",
    }

    // Ensure variables are correctly processed.
    let user = User { name: "Robert".into(), nickname: "Bob".into() };
    assert_uri_eq! {
        uri!(complex("complex", 0, "high", &user)) =>
            "/name/complex?foo=0&type=10&type=high&name=Robert&nickname=Bob",
        uri!(complex("complex", 0, "high", &user)) =>
            "/name/complex?foo=0&type=10&type=high&name=Robert&nickname=Bob",
        uri!(complex("complex", 0, "high", user)) =>
            "/name/complex?foo=0&type=10&type=high&name=Robert&nickname=Bob",
    }
}

#[test]
fn check_location_promotion() {
    struct S1(String);
    struct S2 { name: String }

    let s1 = S1("Bob".into());
    let s2 = S2 { name: "Bob".into() };

    assert_uri_eq! {
        uri!(simple2(1, &S1("A".into()).0)) => "/1/A",
        uri!(simple2(1, &mut S1("A".into()).0)) => "/1/A",
        uri!(simple2(1, S1("A".into()).0)) => "/1/A",
        uri!(simple2(1, &S2 { name: "A".into() }.name)) => "/1/A",
        uri!(simple2(1, &mut S2 { name: "A".into() }.name)) => "/1/A",
        uri!(simple2(1, S2 { name: "A".into() }.name)) => "/1/A",
        uri!(simple2(1, &s1.0)) => "/1/Bob",
        uri!(simple2(1, &s2.name)) => "/1/Bob",
        uri!(simple2(2, &s1.0)) => "/2/Bob",
        uri!(simple2(2, &s2.name)) => "/2/Bob",
        uri!(simple2(2, s1.0)) => "/2/Bob",
        uri!(simple2(2, s2.name)) => "/2/Bob",
    }

    let mut s1 = S1("Bob".into());
    let mut s2 = S2 { name: "Bob".into() };
    assert_uri_eq! {
        uri!(simple2(1, &mut S1("A".into()).0)) => "/1/A",
        uri!(simple2(1, S1("A".into()).0)) => "/1/A",
        uri!(simple2(1, &mut S2 { name: "A".into() }.name)) => "/1/A",
        uri!(simple2(1, S2 { name: "A".into() }.name)) => "/1/A",
        uri!(simple2(1, &mut s1.0)) => "/1/Bob",
        uri!(simple2(1, &mut s2.name)) => "/1/Bob",
        uri!(simple2(2, &mut s1.0)) => "/2/Bob",
        uri!(simple2(2, &mut s2.name)) => "/2/Bob",
        uri!(simple2(2, s1.0)) => "/2/Bob",
        uri!(simple2(2, s2.name)) => "/2/Bob",
    }
}

#[test]
fn check_scoped() {
    assert_uri_eq!{
        uri!(typed_uris::simple(100)) => "/typed_uris/100",
        uri!(typed_uris::simple(id = 100)) => "/typed_uris/100",
        uri!(typed_uris::deeper::simple(100)) => "/typed_uris/deeper/100",
    }
}

mod typed_uris {
    #[post("/typed_uris/<id>")]
    fn simple(id: i32) { }

    #[test]
    fn check_simple_scoped() {
        assert_uri_eq! {
            uri!(simple(id = 100)) => "/typed_uris/100",
            uri!(crate::simple(id = 100)) => "/100",
            uri!("/mount", crate::simple(id = 100)) => "/mount/100",
            uri!(crate::simple2(id = 100, name = "hello")) => "/100/hello",
        }
    }

    pub mod deeper {
        #[post("/typed_uris/deeper/<id>")]
        fn simple(id: i32) { }

        #[test]
        fn check_deep_scoped() {
            assert_uri_eq! {
                uri!(super::simple(id = 100)) => "/typed_uris/100",
                uri!(crate::simple(id = 100)) => "/100",
            }
        }
    }
}

#[derive(FromForm, UriDisplayQuery)]
struct Third<'r> {
    one: String,
    two: &'r str,
}

#[post("/<foo>/<bar>?<q1>&<rest..>")]
fn optionals(
    foo: Option<usize>,
    bar: Option<String>,
    q1: Result<usize, Errors<'_>>,
    rest: Option<Third<'_>>
) { }

#[test]
fn test_optional_uri_parameters() {
    let mut some_10 = Some(10);
    let mut third = Third { one: "hi there".into(), two: "a b".into() };
    assert_uri_eq! {
        uri!(optionals(
            foo = 10,
            bar = &"hi there",
            q1 = Some(10),
            rest = Some(Third { one: "hi there".into(), two: "a b".into() }),
        )) => "/10/hi%20there?q1=10&one=hi%20there&two=a%20b",

        uri!(optionals(
            foo = &10,
            bar = &"hi there",
            q1 = Some(&10),
            rest = Some(&third),
        )) => "/10/hi%20there?q1=10&one=hi%20there&two=a%20b",

        uri!(optionals(
            foo = &mut 10,
            bar = &mut "hi there",
            q1 = some_10.as_mut(),
            rest = Some(&mut third),
        )) => "/10/hi%20there?q1=10&one=hi%20there&two=a%20b",

        uri!(optionals(
            foo = 10,
            bar = &"hi there",
            q1 = _,
            rest = Some(Third { one: "hi there".into(), two: "a b".into() }),
        )) => "/10/hi%20there?one=hi%20there&two=a%20b",

        uri!(optionals(
            foo = 10,
            bar = &"hi there",
            q1 = Some(10),
            rest = _,
        )) => "/10/hi%20there?q1=10",

        uri!(optionals(
            foo = 10,
            bar = &"hi there",
            q1 = Err(ErrorKind::Missing.into()) as Result<usize, _>,
            rest = _,
        )) => "/10/hi%20there",

        uri!(optionals(
            foo = 10,
            bar = &"hi there",
            q1 = None as Option<usize>,
            rest = _
        )) => "/10/hi%20there",

        uri!(optionals(
            foo = 10,
            bar = &"hi there",
            q1 = _,
            rest = None as Option<Third<'_>>,
        )) => "/10/hi%20there",

        uri!(optionals(
            foo = 10,
            bar = &"hi there",
            q1 = _,
            rest = _,
        )) => "/10/hi%20there",
    }
}

#[test]
fn test_simple_ignored() {
    #[get("/<_>")] fn ignore_one() { }
    assert_uri_eq! {
        uri!(ignore_one(100)) => "/100",
        uri!(ignore_one("hello")) => "/hello",
        uri!(ignore_one("cats r us")) => "/cats%20r%20us",
    }

    #[get("/<_>/<_>")] fn ignore_two() { }
    assert_uri_eq! {
        uri!(ignore_two(100, "boop")) => "/100/boop",
        uri!(ignore_two(&"hi", "bop")) => "/hi/bop",
    }

    #[get("/<_>/foo/<_>")] fn ignore_inner_two() { }
    #[get("/hi/<_>/foo")] fn ignore_inner_one_a() { }
    #[get("/hey/hi/<_>/foo/<_>")] fn ignore_inner_two_b() { }

    assert_uri_eq! {
        uri!(ignore_inner_two(100, "boop")) => "/100/foo/boop",
        uri!(ignore_inner_one_a("!?")) => "/hi/!%3F/foo",
        uri!(ignore_inner_two_b(&mut 5, "boo")) => "/hey/hi/5/foo/boo",
    }

    #[get("/<_>/foo/<_>?hi")] fn ignore_with_q() { }
    #[get("/hi/<_>/foo/<_>?hi&<hey>")] fn ignore_with_q2(hey: Option<usize>) { }
    #[get("/hi/<_>/foo/<_>?<hi>&<hey>")] fn ignore_with_q3(hi: &str, hey: &str) { }

    assert_uri_eq! {
        uri!(ignore_with_q(100, "boop")) => "/100/foo/boop?hi",
        uri!(ignore_with_q2("!?", "bop", Some(3usize))) => "/hi/!%3F/foo/bop?hi&hey=3",
        uri!(ignore_with_q3(&mut 5, "boo", "hi b", "ho")) => "/hi/5/foo/boo?hi=hi%20b&hey=ho",
    }
}

#[test]
fn test_maps() {
    use std::collections::{HashMap, BTreeMap};
    use rocket::figment::util::map;

    #[get("/?<bar>")] fn hmap(mut bar: HashMap<String, usize>) {
        let _ = uri!(bmap(&bar));
        let _ = uri!(bmap(&mut bar));
        let _ = uri!(bmap(bar));
    }

    assert_uri_eq! {
        uri!(hmap(map!["foo" => 10])) => "/?bar.k:0=foo&bar.v:0=10",
        uri!(hmap(map!["foo".to_string() => 10])) => "/?bar.k:0=foo&bar.v:0=10",
        uri!(hmap(&map!["foo".to_string() => 10])) => "/?bar.k:0=foo&bar.v:0=10",
        uri!(hmap(&mut map!["foo".to_string() => 10])) => "/?bar.k:0=foo&bar.v:0=10",
        uri!(hmap(&map!["foo" => 10])) => "/?bar.k:0=foo&bar.v:0=10",
    }

    #[get("/?<bar>")] fn bmap(mut bar: BTreeMap<&str, usize>) {
        let _ = uri!(hmap(&bar));
        let _ = uri!(hmap(&mut bar));
        let _ = uri!(hmap(bar));
    }

    assert_uri_eq! {
        uri!(bmap(map!["foo" => 10])) => "/?bar.k:0=foo&bar.v:0=10",
        uri!(bmap(map!["foo".to_string() => 10])) => "/?bar.k:0=foo&bar.v:0=10",
        uri!(bmap(&map!["foo".to_string() => 10])) => "/?bar.k:0=foo&bar.v:0=10",
        uri!(bmap(&mut map!["foo".to_string() => 10])) => "/?bar.k:0=foo&bar.v:0=10",
        uri!(bmap(&map!["foo" => 10])) => "/?bar.k:0=foo&bar.v:0=10",
    }
}

#[test]
fn test_json() {
    use rocket::serde::{Serialize, Deserialize, json::Json};

    #[derive(Serialize, Deserialize, Copy, Clone)]
    #[serde(crate = "rocket::serde")]
    struct Inner<T> {
        foo: Option<T>
    }

    #[get("/?<json>")] fn foo(json: Json<Inner<usize>>) { }

    let mut inner = Inner { foo: Some(10) };
    assert_uri_eq! {
        uri!(foo(inner)) => "/?json=%7B%22foo%22:10%7D",
        uri!(foo(&inner)) => "/?json=%7B%22foo%22:10%7D",
        uri!(foo(&mut inner)) => "/?json=%7B%22foo%22:10%7D",
        uri!(foo(Json(inner))) => "/?json=%7B%22foo%22:10%7D",
        uri!(foo(&Json(inner))) => "/?json=%7B%22foo%22:10%7D",
        uri!(foo(&mut Json(inner))) => "/?json=%7B%22foo%22:10%7D",
    }

    #[get("/?<json>")] fn bar(json: Json<Inner<Inner<&str>>>) { }

    let mut inner = Inner { foo: Some(Inner { foo: Some("hi") }) };
    assert_uri_eq! {
        uri!(bar(inner)) => "/?json=%7B%22foo%22:%7B%22foo%22:%22hi%22%7D%7D",
        uri!(bar(&inner)) => "/?json=%7B%22foo%22:%7B%22foo%22:%22hi%22%7D%7D",
        uri!(bar(&mut inner)) => "/?json=%7B%22foo%22:%7B%22foo%22:%22hi%22%7D%7D",
        uri!(bar(Json(inner))) => "/?json=%7B%22foo%22:%7B%22foo%22:%22hi%22%7D%7D",
        uri!(bar(&Json(inner))) => "/?json=%7B%22foo%22:%7B%22foo%22:%22hi%22%7D%7D",
        uri!(bar(&mut Json(inner))) => "/?json=%7B%22foo%22:%7B%22foo%22:%22hi%22%7D%7D",
    }
}
