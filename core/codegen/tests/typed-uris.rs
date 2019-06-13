#![feature(proc_macro_hygiene, decl_macro)]
#![allow(dead_code, unused_variables)]

#[macro_use] extern crate rocket;

use std::path::PathBuf;

use rocket::http::{RawStr, Cookies};
use rocket::http::uri::{Origin, FromUriParam, Query};
use rocket::request::Form;

#[derive(FromForm, UriDisplayQuery)]
struct User<'a> {
    name: &'a RawStr,
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
fn guard_1(cookies: Cookies<'_>, id: i32) { }

#[post("/<id>/<name>")]
fn guard_2(name: String, cookies: Cookies<'_>, id: i32) { }

#[post("/a/<id>/hi/<name>/hey")]
fn guard_3(id: i32, name: String, cookies: Cookies<'_>) { }

#[post("/<id>", data = "<form>")]
fn no_uri_display_okay(id: i32, form: Form<Second>) { }

#[post("/name/<name>?<foo>&bar=10&<bar>&<query..>", data = "<user>", rank = 2)]
fn complex<'r>(
    foo: usize,
    name: &RawStr,
    query: Form<User<'r>>,
    user: Form<User<'r>>,
    bar: &RawStr,
    cookies: Cookies<'_>
) {  }

#[post("/a/<path..>")]
fn segments(path: PathBuf) { }

#[post("/a/<id>/then/<path..>")]
fn param_and_segments(path: PathBuf, id: usize) { }

#[post("/a/<id>/then/<path..>")]
fn guarded_segments(cookies: Cookies<'_>, path: PathBuf, id: usize) { }

macro assert_uri_eq($($uri:expr => $expected:expr,)+) {
    $(assert_eq!($uri, Origin::parse($expected).expect("valid origin URI"));)+
}

#[test]
fn check_simple_unnamed() {
    assert_uri_eq! {
        uri!(simple: 100) => "/100",
        uri!(simple: -23) => "/-23",
        uri!(unused_param: 1, 2) => "/1/2",
    }

    // The "flipped" test ensures that the order of parameters depends on the
    // route's URI, not on the order in the function signature.
    assert_uri_eq! {
        uri!(simple2: 100, "hello".to_string()) => "/100/hello",
        uri!(simple2: 1349, "hey".to_string()) => "/1349/hey",
        uri!(simple2_flipped: 100, "hello".to_string()) => "/100/hello",
    }

    // Ensure that `.from_uri_param()` is called.
    assert_uri_eq! {
        uri!(simple2: 100, "hello") => "/100/hello",
        uri!(simple2_flipped: 1349, "hey") => "/1349/hey",
    }

    // Ensure that the `UriDisplay` trait is being used.
    assert_uri_eq! {
        uri!(simple2: 100, "hello there") => "/100/hello%20there",
        uri!(simple2_flipped: 100, "hello there") => "/100/hello%20there",
    }

    // Ensure that query parameters are handled properly.
    assert_uri_eq! {
        uri!(simple3: 100) => "/?id=100",
        uri!(simple3: 1349) => "/?id=1349",
        uri!(simple4: 100, "bob") => "/?id=100&name=bob",
        uri!(simple4: 1349, "Bob Anderson") => "/?id=1349&name=Bob%20Anderson",
        uri!(simple4: -2, "@M+s&OU=") => "/?id=-2&name=@M%2Bs%26OU%3D",
        uri!(simple4_flipped: 100, "bob") => "/?id=100&name=bob",
        uri!(simple4_flipped: 1349, "Bob Anderson") => "/?id=1349&name=Bob%20Anderson",
    }
}

#[test]
fn check_simple_named() {
    assert_uri_eq! {
        uri!(simple: id = 100) => "/100",
        uri!(simple: id = -23) => "/-23",
        uri!(unused_param: used = 1, _unused = 2) => "/1/2",
    }

    assert_uri_eq! {
        uri!(simple2: id = 100, name = "hello".to_string()) => "/100/hello",
        uri!(simple2: name = "hi".to_string(), id = 123) => "/123/hi",
        uri!(simple2_flipped: id = 1349, name = "hey".to_string()) => "/1349/hey",
        uri!(simple2_flipped: name = "hello".to_string(), id = 100) => "/100/hello",
    }

    // Ensure that `.from_uri_param()` is called.
    assert_uri_eq! {
        uri!(simple2: id = 100, name = "hello") => "/100/hello",
        uri!(simple2: id = 100, name = "hi") => "/100/hi",
        uri!(simple2: id = 1349, name = "hey") => "/1349/hey",
        uri!(simple2: name = "hello", id = 100) => "/100/hello",
        uri!(simple2: name = "hi", id = 100) => "/100/hi",
        uri!(simple2_flipped: id = 1349, name = "hey") => "/1349/hey",
    }

    // Ensure that the `UriDisplay` trait is being used.
    assert_uri_eq! {
        uri!(simple2: id = 100, name = "hello there") => "/100/hello%20there",
        uri!(simple2: name = "hello there", id = 100) => "/100/hello%20there",
        uri!(simple2_flipped: id = 100, name = "hello there") => "/100/hello%20there",
        uri!(simple2_flipped: name = "hello there", id = 100) => "/100/hello%20there",
    }

    // Ensure that query parameters are handled properly.
    assert_uri_eq! {
        uri!(simple3: id = 100) => "/?id=100",
        uri!(simple3: id = 1349) => "/?id=1349",
        uri!(simple4: id = 100, name = "bob") => "/?id=100&name=bob",
        uri!(simple4: id = 1349, name = "Bob A") => "/?id=1349&name=Bob%20A",
        uri!(simple4: name = "Bob A", id = 1349) => "/?id=1349&name=Bob%20A",
        uri!(simple4_flipped: id = 1349, name = "Bob A") => "/?id=1349&name=Bob%20A",
        uri!(simple4_flipped: name = "Bob A", id = 1349) => "/?id=1349&name=Bob%20A",
    }
}

#[test]
fn check_mount_point() {
    assert_uri_eq! {
        uri!("/mount", simple: 100) => "/mount/100",
        uri!("/mount", simple: id = 23) => "/mount/23",
        uri!("/another", simple: 100) => "/another/100",
        uri!("/another", simple: id = 23) => "/another/23",
    }

    assert_uri_eq! {
        uri!("/a", simple2: 100, "hey") => "/a/100/hey",
        uri!("/b", simple2: id = 23, name = "hey") => "/b/23/hey",
    }
}

#[test]
fn check_guards_ignored() {
    assert_uri_eq! {
        uri!("/mount", guard_1: 100) => "/mount/100",
        uri!("/mount", guard_2: 2938, "boo") => "/mount/2938/boo",
        uri!("/mount", guard_3: 340, "Bob") => "/mount/a/340/hi/Bob/hey",
        uri!(guard_1: 100) => "/100",
        uri!(guard_2: 2938, "boo") => "/2938/boo",
        uri!(guard_3: 340, "Bob") => "/a/340/hi/Bob/hey",
        uri!("/mount", guard_1: id = 100) => "/mount/100",
        uri!("/mount", guard_2: id = 2938, name = "boo") => "/mount/2938/boo",
        uri!("/mount", guard_3: id = 340, name = "Bob") => "/mount/a/340/hi/Bob/hey",
        uri!(guard_1: id = 100) => "/100",
        uri!(guard_2: name = "boo", id = 2938) => "/2938/boo",
        uri!(guard_3: name = "Bob", id = 340) => "/a/340/hi/Bob/hey",
    }
}

#[test]
fn check_with_segments() {
    assert_uri_eq! {
        uri!(segments: PathBuf::from("one/two/three")) => "/a/one/two/three",
        uri!(segments: path = PathBuf::from("one/two/three")) => "/a/one/two/three",
        uri!("/c", segments: PathBuf::from("one/tw o/")) => "/c/a/one/tw%20o",
        uri!("/c", segments: path = PathBuf::from("one/tw o/")) => "/c/a/one/tw%20o",
        uri!(segments: PathBuf::from("one/ tw?o/")) => "/a/one/%20tw%3Fo",
        uri!(param_and_segments: 10, PathBuf::from("a/b")) => "/a/10/then/a/b",
        uri!(param_and_segments: id = 10, path = PathBuf::from("a/b"))
            => "/a/10/then/a/b",
        uri!(guarded_segments: 10, PathBuf::from("a/b")) => "/a/10/then/a/b",
        uri!(guarded_segments: id = 10, path = PathBuf::from("a/b"))
            => "/a/10/then/a/b",
    }

    // Now check the `from_uri_param()` conversions for `PathBuf`.
    assert_uri_eq! {
        uri!(segments: "one/two/three") => "/a/one/two/three",
        uri!("/oh", segments: path = "one/two/three") => "/oh/a/one/two/three",
        uri!(segments: "one/ tw?o/") => "/a/one/%20tw%3Fo",
        uri!(param_and_segments: id = 10, path = "a/b") => "/a/10/then/a/b",
        uri!(guarded_segments: 10, "a/b") => "/a/10/then/a/b",
        uri!(guarded_segments: id = 10, path = "a/b") => "/a/10/then/a/b",
    }
}

#[test]
fn check_complex() {
    assert_uri_eq! {
        uri!(complex: "no idea", 10, "high", ("A B C", "a c")) =>
            "/name/no%20idea?foo=10&bar=10&bar=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex: "Bob", 248, "?", User { name: "Robert".into(), nickname: "Bob".into() }) =>
            "/name/Bob?foo=248&bar=10&bar=%3F&name=Robert&nickname=Bob",
        uri!(complex: "Bob", 248, "a a", &User { name: "Robert".into(), nickname: "B".into() }) =>
            "/name/Bob?foo=248&bar=10&bar=a%20a&name=Robert&nickname=B",
        uri!(complex: "no idea", 248, "", &User { name: "A B".into(), nickname: "A".into() }) =>
            "/name/no%20idea?foo=248&bar=10&bar=&name=A%20B&nickname=A",
        uri!(complex: "hi", 3, "b", &User { name: "A B C".into(), nickname: "a b".into() }) =>
            "/name/hi?foo=3&bar=10&bar=b&name=A%20B%20C&nickname=a%20b",
        uri!(complex: name = "no idea", foo = 10, bar = "high", query = ("A B C", "a c")) =>
            "/name/no%20idea?foo=10&bar=10&bar=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex: foo = 10, name = "no idea", bar = "high", query = ("A B C", "a c")) =>
            "/name/no%20idea?foo=10&bar=10&bar=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex: query = ("A B C", "a c"), foo = 10, name = "no idea", bar = "high", ) =>
            "/name/no%20idea?foo=10&bar=10&bar=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex: query = ("A B C", "a c"), foo = 10, name = "no idea", bar = "high") =>
            "/name/no%20idea?foo=10&bar=10&bar=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex: query = *&("A B C", "a c"), foo = 10, name = "no idea", bar = "high") =>
            "/name/no%20idea?foo=10&bar=10&bar=high&name=A%20B%20C&nickname=a%20c",
        uri!(complex: foo = 3, name = "hi", bar = "b",
                query = &User { name: "A B C".into(), nickname: "a b".into() }) =>
                "/name/hi?foo=3&bar=10&bar=b&name=A%20B%20C&nickname=a%20b",
        uri!(complex: query = &User { name: "A B C".into(), nickname: "a b".into() },
                 foo = 3, name = "hi", bar = "b") =>
                "/name/hi?foo=3&bar=10&bar=b&name=A%20B%20C&nickname=a%20b",
    }

    // Ensure variables are correctly processed.
    let user = User { name: "Robert".into(), nickname: "Bob".into() };
    assert_uri_eq! {
        uri!(complex: "complex", 0, "high", &user) =>
            "/name/complex?foo=0&bar=10&bar=high&name=Robert&nickname=Bob",
        uri!(complex: "complex", 0, "high", &user) =>
            "/name/complex?foo=0&bar=10&bar=high&name=Robert&nickname=Bob",
        uri!(complex: "complex", 0, "high", user) =>
            "/name/complex?foo=0&bar=10&bar=high&name=Robert&nickname=Bob",
    }
}

#[test]
fn check_location_promotion() {
    struct S1(String);
    struct S2 { name: String };

    let s1 = S1("Bob".into());
    let s2 = S2 { name: "Bob".into() };

    assert_uri_eq! {
        uri!(simple2: 1, &S1("A".into()).0) => "/1/A",
        uri!(simple2: 1, &mut S1("A".into()).0) => "/1/A",
        uri!(simple2: 1, S1("A".into()).0) => "/1/A",
        uri!(simple2: 1, &S2 { name: "A".into() }.name) => "/1/A",
        uri!(simple2: 1, &mut S2 { name: "A".into() }.name) => "/1/A",
        uri!(simple2: 1, S2 { name: "A".into() }.name) => "/1/A",
        uri!(simple2: 1, &s1.0) => "/1/Bob",
        uri!(simple2: 1, &s2.name) => "/1/Bob",
        uri!(simple2: 2, &s1.0) => "/2/Bob",
        uri!(simple2: 2, &s2.name) => "/2/Bob",
        uri!(simple2: 2, s1.0) => "/2/Bob",
        uri!(simple2: 2, s2.name) => "/2/Bob",
    }

    let mut s1 = S1("Bob".into());
    let mut s2 = S2 { name: "Bob".into() };
    assert_uri_eq! {
        uri!(simple2: 1, &mut S1("A".into()).0) => "/1/A",
        uri!(simple2: 1, S1("A".into()).0) => "/1/A",
        uri!(simple2: 1, &mut S2 { name: "A".into() }.name) => "/1/A",
        uri!(simple2: 1, S2 { name: "A".into() }.name) => "/1/A",
        uri!(simple2: 1, &mut s1.0) => "/1/Bob",
        uri!(simple2: 1, &mut s2.name) => "/1/Bob",
        uri!(simple2: 2, &mut s1.0) => "/2/Bob",
        uri!(simple2: 2, &mut s2.name) => "/2/Bob",
        uri!(simple2: 2, s1.0) => "/2/Bob",
        uri!(simple2: 2, s2.name) => "/2/Bob",
    }
}

#[test]
fn check_scoped() {
    assert_uri_eq!{
        uri!(typed_uris::simple: 100) => "/typed_uris/100",
        uri!(typed_uris::simple: id = 100) => "/typed_uris/100",
        uri!(typed_uris::deeper::simple: 100) => "/typed_uris/deeper/100",
    }
}

mod typed_uris {
    use super::assert_uri_eq;

    #[post("/typed_uris/<id>")]
    fn simple(id: i32) { }

    #[test]
    fn check_simple_scoped() {
        assert_uri_eq! {
            uri!(simple: id = 100) => "/typed_uris/100",
            uri!(crate::simple: id = 100) => "/100",
            uri!("/mount", crate::simple: id = 100) => "/mount/100",
            uri!(crate::simple2: id = 100, name = "hello") => "/100/hello",
        }
    }

    pub mod deeper {
        use super::assert_uri_eq;

        #[post("/typed_uris/deeper/<id>")]
        fn simple(id: i32) { }

        #[test]
        fn check_deep_scoped() {
            assert_uri_eq! {
                uri!(super::simple: id = 100) => "/typed_uris/100",
                uri!(crate::simple: id = 100) => "/100",
            }
        }
    }
}

#[derive(FromForm, UriDisplayQuery)]
struct Third<'r> {
    one: String,
    two: &'r RawStr,
}

#[post("/<foo>/<bar>?<q1>&<rest..>")]
fn optionals(
    foo: Option<usize>,
    bar: Result<String, &RawStr>,
    q1: Result<usize, &RawStr>,
    rest: Option<Form<Third<'_>>>
) { }

#[test]
fn test_optional_uri_parameters() {
    assert_uri_eq! {
        uri!(optionals:
            foo = 10,
            bar = &"hi there",
            q1 = 10,
            rest = Third { one: "hi there".into(), two: "a b".into() }
        ) => "/10/hi%20there?q1=10&one=hi%20there&two=a%20b",

        uri!(optionals:
            foo = &10,
            bar = &"hi there",
            q1 = &10,
            rest = &Third { one: "hi there".into(), two: "a b".into() }
        ) => "/10/hi%20there?q1=10&one=hi%20there&two=a%20b",

        uri!(optionals:
            foo = &mut 10,
            bar = &mut "hi there",
            q1 = &mut 10,
            rest = &mut Third { one: "hi there".into(), two: "a b".into() }
        ) => "/10/hi%20there?q1=10&one=hi%20there&two=a%20b",

        uri!(optionals:
            foo = 10,
            bar = &"hi there",
            q1 = _,
            rest = Third { one: "hi there".into(), two: "a b".into() }
        ) => "/10/hi%20there?one=hi%20there&two=a%20b",

        uri!(optionals:
            foo = 10,
            bar = &"hi there",
            q1 = 10,
            rest = _
        ) => "/10/hi%20there?q1=10",

        uri!(optionals:
            foo = 10,
            bar = &"hi there",
            q1 = _,
            rest = _,
        ) => "/10/hi%20there",
    }
}
