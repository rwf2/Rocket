use crate::uri::{Origin, Authority, Absolute, Asterisk};
use crate::parse::uri::*;

macro_rules! assert_parse_eq {
    ($($from:expr => $to:expr),+ $(,)?) => (
        $(
            let expected = $to;
            match from_str($from) {
                Ok(output) => {
                    if output != expected {
                        println!("Failure on: {:?}", $from);
                        assert_eq!(output, expected, "{} != {}", output, expected);
                    }
                }
                Err(e) => {
                    println!("{:?} failed to parse!", $from);
                    panic!("Error: {}", e);
                }
            }
        )+
    );
}

macro_rules! assert_no_parse {
    ($($from:expr),+ $(,)?) => (
        $(
            if let Ok(uri) = from_str($from) {
                println!("{:?} parsed unexpectedly!", $from);
                panic!("Parsed as: {:?}", uri);
            }
        )+
    );

    ($($from:expr),+,) => (assert_no_parse!($($from),+))
}

macro_rules! assert_parse {
    ($($from:expr),+ $(,)?) => (
        $(
            if let Err(e) = from_str($from) {
                println!("{:?} failed to parse", $from);
                panic!("{}", e);
            }
        )+
    );
}

macro_rules! assert_displays_eq {
    ($($string:expr),+ $(,)?) => (
        $(
            let string = $string.into();
            match from_str(string) {
                Ok(output) => {
                    let output_string = output.to_string();
                    if output_string != string {
                        println!("Failure on: {:?}", $string);
                        println!("Got: {:?}", output_string);
                        println!("Parsed as: {:?}", output);
                        panic!("failed");
                    }
                }
                Err(e) => {
                    println!("{:?} failed to parse!", $string);
                    panic!("Error: {}", e);
                }
            }
        )+
    );

    ($($string:expr),+,) => (assert_parse_eq!($($string),+))
}

#[test]
#[should_panic]
fn test_assert_parse_eq() {
    assert_parse_eq!("*" => Origin::path_only("*"));
}

#[test]
#[should_panic]
fn test_assert_parse_eq_consecutive() {
    assert_parse_eq! {
        "/" => Origin::ROOT,
        "/" => Asterisk
    };
}

#[test]
#[should_panic]
fn test_assert_no_parse() {
    assert_no_parse!("/");
}

#[test]
fn bad_parses() {
    assert_no_parse! {
        "://z7:77777777777777777777777777777`77777777777",

        // from #1621
        ":/",

        // almost URIs
        ":/",
        "://",
        "::",
        ":::",
        "a://a::",
    };
}

#[test]
fn test_parse_issue_924_samples() {
    assert_parse!("/path?param={value}",
        "/path/?param={value}",
        "/some/path/?param={forgot-to-replace-placeholder}",
        "/path?param={value}&onemore={value}",
        "/some/path/?tags=[]", "/some/path/?tags=[rocket,is,perfect]",
        "/some/path/?tags=[rocket|is\\perfect^`]&users={arenot}",
        "/rocket/@user/",
        "/rocket/@user/?tags=[rocket,%F0%9F%98%8B]",
        "/rocket/?username=@sergio&tags=[rocket,%F0%9F%98%8B]",
        "/rocket/?Key+With+Spaces=value+too",
        "/rocket/?Key+With+\'",
        "/rocket/?query=%3E5",
    );

    assert_no_parse!("/rocket/?query=>5");
}

#[test]
fn single_byte() {
    assert_parse_eq!(
        "*" => Asterisk,
        "/" => Origin::ROOT,
        "." => Authority::new(None, ".", None),
        "_" => Authority::new(None, "_", None),
        "1" => Authority::new(None, "1", None),
        "b" => Authority::new(None, "b", None),
        "%" => Authority::new(None, "%", None),
        "?" => Reference::new(None, None, "", "", None),
        "#" => Reference::new(None, None, "", None, ""),
        ":" => Authority::new(None, "", 0),
        "@" => Authority::new("", "", None),
    );

    assert_no_parse!["\\", "^"];
}

#[test]
fn origin() {
    assert_parse_eq!(
        "/a/b/c" => Origin::path_only("/a/b/c"),
        "//" => Origin::path_only("//"),
        "///" => Origin::path_only("///"),
        "////" => Origin::path_only("////"),
        "/a/b/c?" => Origin::new("/a/b/c", Some("")),
        "/a/b/c?abc" => Origin::new("/a/b/c", Some("abc")),
        "/a/b/c???" => Origin::new("/a/b/c", Some("??")),
        "/a/b/c?a?b?" => Origin::new("/a/b/c", Some("a?b?")),
        "/a/b/c?a?b?/c" => Origin::new("/a/b/c", Some("a?b?/c")),
        "/?abc" => Origin::new("/", Some("abc")),
        "/hi%20there?a=b&c=d" => Origin::new("/hi%20there", Some("a=b&c=d")),
        "/c/d/fa/b/c?abc" => Origin::new("/c/d/fa/b/c", Some("abc")),
        "/xn--ls8h?emoji=poop" => Origin::new("/xn--ls8h", Some("emoji=poop")),
        "/?t=[rocket|is\\here^`]&{ok}" => Origin::new("/", Some("t=[rocket|is\\here^`]&{ok}")),
    );
}

#[test]
fn authority() {
    assert_parse_eq!(
        "@:" => Authority::new("", "", 0),
        "abc" => Authority::new(None, "abc", None),
        "@abc" => Authority::new("", "abc", None),
        "a@b" => Authority::new("a", "b", None),
        "a@" => Authority::new("a", "", None),
        ":@" => Authority::new(":", "", None),
        ":@:" => Authority::new(":", "", 0),
        "sergio:benitez@spark" => Authority::new("sergio:benitez", "spark", None),
        "a:b:c@1.2.3:12121" => Authority::new("a:b:c", "1.2.3", 12121),
        "sergio@spark" => Authority::new("sergio", "spark", None),
        "sergio@spark:230" => Authority::new("sergio", "spark", 230),
        "sergio@[1::]:230" => Authority::new("sergio", "[1::]", 230),
        "rocket.rs:8000" => Authority::new(None, "rocket.rs", 8000),
        "[1::2::3]:80" => Authority::new(None, "[1::2::3]", 80),
        "bar:" => Authority::new(None, "bar", 0), // could be absolute too
    );
}

#[test]
fn absolute() {
    assert_parse_eq! {
        "http:/" => Absolute::new("http", None, "/", None),
        "http://" => Absolute::new("http", Authority::new(None, "", None), "", None),
        "http:///" => Absolute::new("http", Authority::new(None, "", None), "/", None),
        "http://a.com:8000" => Absolute::new("http", Authority::new(None, "a.com", 8000), "", None),
        "http://foo:8000" => Absolute::new("http", Authority::new(None, "foo", 8000), "", None),
        "foo:bar" => Absolute::new("foo", None, "bar", None),
        "ftp:::" => Absolute::new("ftp", None, "::", None),
        "ftp:::?bar" => Absolute::new("ftp", None, "::", "bar"),
        "http://:::@a.b.c.:8000" =>
            Absolute::new("http", Authority::new(":::", "a.b.c.", 8000), "", None),
        "http://sergio:pass@foo.com:8000" =>
            Absolute::new("http", Authority::new("sergio:pass", "foo.com", 8000), "", None),
        "foo:/sergio/pass?hi" => Absolute::new("foo", None, "/sergio/pass", "hi"),
        "foo:?hi" => Absolute::new("foo", None, "", "hi"),
        "foo:a/b" => Absolute::new("foo", None, "a/b", None),
        "foo:a/b?" => Absolute::new("foo", None, "a/b", ""),
        "foo:a/b?hi" => Absolute::new("foo", None, "a/b", "hi"),
        "foo:/a/b" => Absolute::new("foo", None, "/a/b", None),
        "abc://u:p@foo.com:123/a/b?key=value&key2=value2" =>
            Absolute::new("abc",
                Authority::new("u:p", "foo.com", 123),
                "/a/b", "key=value&key2=value2"),
        "ftp://foo.com:21/abc" =>
            Absolute::new("ftp", Authority::new(None, "foo.com", 21), "/abc", None),
        "http://rocket.rs/abc" =>
            Absolute::new("http", Authority::new(None, "rocket.rs", None), "/abc", None),
        "http://s:b@rocket.rs/abc" =>
            Absolute::new("http", Authority::new("s:b", "rocket.rs", None), "/abc", None),
        "http://rocket.rs/abc?q" =>
            Absolute::new("http", Authority::new(None, "rocket.rs", None), "/abc", "q"),
        "http://rocket.rs" =>
            Absolute::new("http", Authority::new(None, "rocket.rs", None), "", None),
        "git://s::@rocket.rs:443/abc?q" =>
            Absolute::new("git", Authority::new("s::", "rocket.rs", 443), "/abc", "q"),
        "git://:@rocket.rs:443/abc?q" =>
            Absolute::new("git", Authority::new(":", "rocket.rs", 443), "/abc", "q"),
        "a://b?test" => Absolute::new("a", Authority::new(None, "b", None), "", "test"),
        "a://b:?test" => Absolute::new("a", Authority::new(None, "b", 0), "", "test"),
        "a://b:1?test" => Absolute::new("a", Authority::new(None, "b", 1), "", "test"),
    };
}

#[test]
fn reference() {
    assert_parse_eq!(
        "*#" => Reference::new(None, None, "*", None, ""),
        "*#h" => Reference::new(None, None, "*", None, "h"),
        "@/" => Reference::new(None, None, "@/", None, None),
        "@?" => Reference::new(None, None, "@", "", None),
        "@?#" => Reference::new(None, None, "@", "", ""),
        "@#foo" => Reference::new(None, None, "@", None, "foo"),
        "foo/bar" => Reference::new(None, None, "foo/bar", None, None),
        "foo/bar?baz" => Reference::new(None, None, "foo/bar", "baz", None),
        "foo/bar?baz#cat" => Reference::new(None, None, "foo/bar", "baz", "cat"),
        "a?b#c" => Reference::new(None, None, "a", "b", "c"),
        "?#" => Reference::new(None, None, "", "", ""),
        "ftp:foo/bar?baz#" => Reference::new("ftp", None, "foo/bar", "baz", ""),
        "ftp:bar#" => Reference::new("ftp", None, "bar", None, ""),
        "ftp:?bar#" => Reference::new("ftp", None, "", "bar", ""),
        "ftp:::?bar#" => Reference::new("ftp", None, "::", "bar", ""),
        "#foo" => Reference::new(None, None, "", None, "foo"),
        "a:/#" => Reference::new("a", None, "/", None, ""),
        "a:/?a#" => Reference::new("a", None, "/", "a", ""),
        "a:/?a#b" => Reference::new("a", None, "/", "a", "b"),
        "a:?a#b" => Reference::new("a", None, "", "a", "b"),
        "a://?a#b" => Reference::new("a", Authority::new(None, "", None), "", "a", "b"),
        "a://:?a#b" => Reference::new("a", Authority::new(None, "", 0), "", "a", "b"),
        "a://:2000?a#b" => Reference::new("a", Authority::new(None, "", 2000), "", "a", "b"),
        "a://a:2000?a#b" => Reference::new("a", Authority::new(None, "a", 2000), "", "a", "b"),
        "a://a:@2000?a#b" => Reference::new("a", Authority::new("a:", "2000", None), "", "a", "b"),
        "a://a:@:80?a#b" => Reference::new("a", Authority::new("a:", "", 80), "", "a", "b"),
        "a://a:@b:80?a#b" => Reference::new("a", Authority::new("a:", "b", 80), "", "a", "b"),
    );
}

#[test]
fn display() {
    assert_displays_eq! {
        "abc", "@):0", "[a]",
        "http://rocket.rs", "http://a:b@rocket.rs", "git://a@b:800/foo?bar",
        "git://a@b:800/foo?bar#baz",
        "a:b", "a@b", "a?b", "a?b#c",
    }
}
