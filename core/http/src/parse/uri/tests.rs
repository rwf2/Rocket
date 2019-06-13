use crate::uri::{Uri, Origin, Authority, Absolute};
use crate::parse::uri::*;
use crate::uri::Host::*;

macro_rules! assert_parse_eq {
    ($($from:expr => $to:expr),+) => (
        $(
            let expected = $to.into();
            match from_str($from) {
                Ok(output) => {
                    if output != expected {
                        println!("Failure on: {:?}", $from);
                        assert_eq!(output, expected);
                    }
                }
                Err(e) => {
                    println!("{:?} failed to parse!", $from);
                    panic!("Error: {}", e);
                }
            }
        )+
    );

    ($($from:expr => $to:expr),+,) => (assert_parse_eq!($($from => $to),+))
}

macro_rules! assert_no_parse {
    ($($from:expr),+) => (
        $(
            if let Ok(uri) = from_str($from) {
                println!("{:?} parsed unexpectedly!", $from);
                panic!("Parsed as: {:?}", uri);
            }
        )+
    );

    ($($from:expr),+,) => (assert_no_parse!($($from),+))
}

macro_rules! assert_displays_eq {
    ($($string:expr),+) => (
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

fn uri_origin<'a>(path: &'a str, query: Option<&'a str>) -> Uri<'a> {
    Uri::Origin(Origin::new(path, query))
}

#[test]
#[should_panic]
fn test_assert_parse_eq() {
    assert_parse_eq!("*" => uri_origin("*", None));
}

#[test]
#[should_panic]
fn test_assert_parse_eq_consecutive() {
    assert_parse_eq!("/" => uri_origin("/", None), "/" => Uri::Asterisk);
}

#[test]
#[should_panic]
fn test_assert_no_parse() {
    assert_no_parse!("/");
}

#[test]
fn bad_parses() {
    assert_no_parse!("://z7:77777777777777777777777777777`77777777777");
}

#[test]
fn single_byte() {
    assert_parse_eq!(
        "*" => Uri::Asterisk,
        "/" => uri_origin("/", None),
        "." => Authority::new(None, Raw("."), None),
        "_" => Authority::new(None, Raw("_"), None),
        "1" => Authority::new(None, Raw("1"), None),
        "b" => Authority::new(None, Raw("b"), None),
    );

    assert_no_parse!("?", "#", "%");
}

#[test]
fn origin() {
    assert_parse_eq!(
        "/a/b/c" => uri_origin("/a/b/c", None),
        "/a/b/c?" => uri_origin("/a/b/c", Some("")),
        "/a/b/c?abc" => uri_origin("/a/b/c", Some("abc")),
        "/a/b/c???" => uri_origin("/a/b/c", Some("??")),
        "/a/b/c?a?b?" => uri_origin("/a/b/c", Some("a?b?")),
        "/a/b/c?a?b?/c" => uri_origin("/a/b/c", Some("a?b?/c")),
        "/?abc" => uri_origin("/", Some("abc")),
        "/hi%20there?a=b&c=d" => uri_origin("/hi%20there", Some("a=b&c=d")),
        "/c/d/fa/b/c?abc" => uri_origin("/c/d/fa/b/c", Some("abc")),
        "/xn--ls8h?emoji=poop" => uri_origin("/xn--ls8h", Some("emoji=poop")),
    );
}

#[test]
fn authority() {
    assert_parse_eq!(
        "abc" => Authority::new(None, Raw("abc"), None),
        "@abc" => Authority::new(Some(""), Raw("abc"), None),
        "sergio:benitez@spark" => Authority::new(Some("sergio:benitez"), Raw("spark"), None),
        "a:b:c@1.2.3:12121" => Authority::new(Some("a:b:c"), Raw("1.2.3"), Some(12121)),
        "sergio@spark" => Authority::new(Some("sergio"), Raw("spark"), None),
        "sergio@spark:230" => Authority::new(Some("sergio"), Raw("spark"), Some(230)),
        "sergio@[1::]:230" => Authority::new(Some("sergio"), Bracketed("1::"), Some(230)),
        "google.com:8000" => Authority::new(None, Raw("google.com"), Some(8000)),
        "[1::2::3]:80" => Authority::new(None, Bracketed("1::2::3"), Some(80)),
    );
}

#[test]
fn absolute() {
    assert_parse_eq! {
        "http://foo.com:8000" => Absolute::new(
            "http",
            Some(Authority::new(None, Raw("foo.com"), Some(8000))),
            None
        ),
        "http://foo:8000" => Absolute::new(
            "http",
            Some(Authority::new(None, Raw("foo"), Some(8000))),
            None,
        ),
        "foo:bar" => Absolute::new(
            "foo",
            None,
            Some(Origin::new::<_, &str>("bar", None)),
        ),
        "http://sergio:pass@foo.com:8000" => Absolute::new(
            "http",
            Some(Authority::new(Some("sergio:pass"), Raw("foo.com"), Some(8000))),
            None,
        ),
        "foo:/sergio/pass?hi" => Absolute::new(
            "foo",
            None,
            Some(Origin::new("/sergio/pass", Some("hi"))),
        ),
        "bar:" => Absolute::new(
            "bar",
            None,
            Some(Origin::new::<_, &str>("", None)),
        ),
        "foo:?hi" => Absolute::new(
            "foo",
            None,
            Some(Origin::new("", Some("hi"))),
        ),
        "foo:a/b?hi" => Absolute::new(
            "foo",
            None,
            Some(Origin::new("a/b", Some("hi"))),
        ),
        "foo:a/b" => Absolute::new(
            "foo",
            None,
            Some(Origin::new::<_, &str>("a/b", None)),
        ),
        "foo:/a/b" => Absolute::new(
            "foo",
            None,
            Some(Origin::new::<_, &str>("/a/b", None))
        ),
        "abc://u:p@foo.com:123/a/b?key=value&key2=value2" => Absolute::new(
            "abc",
            Some(Authority::new(Some("u:p"), Raw("foo.com"), Some(123))),
            Some(Origin::new("/a/b", Some("key=value&key2=value2"))),
        ),
        "ftp://foo.com:21/abc" => Absolute::new(
            "ftp",
            Some(Authority::new(None, Raw("foo.com"), Some(21))),
            Some(Origin::new::<_, &str>("/abc", None)),
        ),
        "http://google.com/abc" => Absolute::new(
            "http",
            Some(Authority::new(None, Raw("google.com"), None)),
            Some(Origin::new::<_, &str>("/abc", None)),
         ),
        "http://google.com" => Absolute::new(
            "http",
            Some(Authority::new(None, Raw("google.com"), None)),
            None
        ),
        "http://foo.com?test" => Absolute::new(
            "http",
            Some(Authority::new(None, Raw("foo.com"), None,)),
            Some(Origin::new("", Some("test"))),
        ),
        "http://google.com/abc?hi" => Absolute::new(
            "http",
            Some(Authority::new(None, Raw("google.com"), None,)),
            Some(Origin::new("/abc", Some("hi"))),
        ),
    };
}

#[test]
fn display() {
    assert_displays_eq! {
        "abc", "@):0", "[a]"
    }
}
