use std::borrow::Cow;

use pear::{ParseError, ParseResult};
use pear::parsers::*;
use pear::combinators::*;
use smallvec::SmallVec;

use http::{MediaType, MediaParams};
use http::parse::checkers::{is_whitespace, is_valid_token};
use http::parse::IndexedStr;

#[parser]
fn quoted_string<'a>(input: &mut &'a str) -> ParseResult<&'a str, &'a str> {
    eat('"');

    let mut is_escaped = false;
    let inner = take_while(|c| {
        if is_escaped { is_escaped = false; return true; }
        if c == '\\' { is_escaped = true; return true; }
        c != '"'
    });

    eat('"');
    inner
}

#[parser]
pub fn media_type<'a>(input: &mut &'a str) -> ParseResult<&'a str, MediaType> {
    let source: &str = *input;

    let top = take_some_while(|c| is_valid_token(c) && c != '/');
    eat('/');
    let sub = take_some_while(is_valid_token);

    let mut params = SmallVec::new();
    switch_repeat! {
        surrounded(|i| eat(i, ';'), is_whitespace) => {
            let key = take_some_while(|c| is_valid_token(c) && c != '=');
            eat('=');

            let value = switch! {
                peek('"') => quoted_string(),
                _ => take_some_while(|c| is_valid_token(c) && c != ';')
            };

            let indexed_key = IndexedStr::from(key, source).expect("key");
            let indexed_val = IndexedStr::from(value, source).expect("val");
            params.push((indexed_key, indexed_val))
        },
        _ => break
    }

    MediaType {
        source: Some(Cow::Owned(source.to_string())),
        top: IndexedStr::from(top, source).expect("top in source"),
        sub: IndexedStr::from(sub, source).expect("sub in source"),
        params: MediaParams::Dynamic(params)
    }
}

pub fn parse_media_type(mut input: &str) -> Result<MediaType, ParseError<&str>> {
    parse!(&mut input, (media_type(), eof()).0).into()
}

#[cfg(test)]
mod test {
    use http::MediaType;
    use super::parse_media_type;

    macro_rules! assert_no_parse {
        ($string:expr) => ({
            let result: Result<_, _> = parse_media_type($string).into();
            if result.is_ok() {
                panic!("{:?} parsed unexpectedly.", $string)
            }
        });
    }

    macro_rules! assert_parse {
        ($string:expr) => ({
            match parse_media_type($string) {
                Ok(media_type) => media_type,
                Err(e) => panic!("{:?} failed to parse: {}", $string, e)
            }
        });
    }

    macro_rules! assert_parse_eq {
        (@full $string:expr, $result:expr, $(($k:expr, $v:expr)),*) => ({
            let result = assert_parse!($string);
            assert_eq!(result, $result);

            let expected_params: Vec<(&str, &str)> = vec![$(($k, $v)),*];
            if expected_params.len() > 0 {
                assert_eq!(result.params().count(), expected_params.len());
                let all_params = result.params().zip(expected_params.iter());
                for ((key, val), &(ekey, eval)) in all_params {
                    assert_eq!(key, ekey);
                    assert_eq!(val, eval);
                }
            }
        });

        (from: $string:expr, into: $result:expr)
            => (assert_parse_eq!(@full $string, $result, ));
        (from: $string:expr, into: $result:expr, params: $(($key:expr, $val:expr)),*)
            => (assert_parse_eq!(@full $string, $result, $(($key, $val)),*));
    }

    #[test]
    fn check_does_parse() {
        assert_parse!("text/html");
        assert_parse!("a/b");
        assert_parse!("*/*");
    }

    #[test]
    fn check_parse_eq() {
        assert_parse_eq!(from: "text/html", into: MediaType::HTML);
        assert_parse_eq!(from: "text/html; charset=utf-8", into: MediaType::HTML);
        assert_parse_eq!(from: "text/html", into: MediaType::HTML);

        assert_parse_eq!(from: "a/b", into: MediaType::new("a", "b"));
        assert_parse_eq!(from: "*/*", into: MediaType::Any);
        assert_parse_eq!(from: "application/pdf", into: MediaType::PDF);
        assert_parse_eq!(from: "application/json", into: MediaType::JSON);
        assert_parse_eq!(from: "image/svg+xml", into: MediaType::SVG);

        assert_parse_eq!(from: "*/json", into: MediaType::new("*", "json"));
        assert_parse_eq! {
            from: "application/*; param=1",
            into: MediaType::new("application", "*")
        };
    }

    #[test]
    fn check_param_eq() {
        assert_parse_eq! {
            from: "text/html; a=b; b=c; c=d",
            into: MediaType::new("text", "html"),
            params: ("a", "b"), ("b", "c"), ("c", "d")
        };

        assert_parse_eq! {
            from: "text/html;a=b;b=c;     c=d; d=e",
            into: MediaType::new("text", "html"),
            params: ("a", "b"), ("b", "c"), ("c", "d"), ("d", "e")
        };

        assert_parse_eq! {
            from: "text/html; charset=utf-8",
            into: MediaType::new("text", "html"),
            params: ("charset", "utf-8")
        };

        assert_parse_eq! {
            from: "application/*; param=1",
            into: MediaType::new("application", "*"),
            params: ("param", "1")
        };

        assert_parse_eq! {
            from: "*/*;q=0.5;b=c;c=d",
            into: MediaType::Any,
            params: ("q", "0.5"), ("b", "c"), ("c", "d")
        };

        assert_parse_eq! {
            from: "multipart/form-data; boundary=----WebKitFormBoundarypRshfItmvaC3aEuq",
            into: MediaType::FormData,
            params: ("boundary", "----WebKitFormBoundarypRshfItmvaC3aEuq")
        };

        assert_parse_eq! {
            from: r#"*/*; a="hello, world!@#$%^&*();;hi""#,
            into: MediaType::Any,
            params: ("a", "hello, world!@#$%^&*();;hi")
        };

        assert_parse_eq! {
            from: r#"application/json; a=";,;""#,
            into: MediaType::JSON,
            params: ("a", ";,;")
        };

        assert_parse_eq! {
            from: r#"application/json; a=";,;"; b=c"#,
            into: MediaType::JSON,
            params: ("a", ";,;"), ("b", "c")
        };

        assert_parse_eq! {
            from: r#"application/json; b=c; a=";.,.;""#,
            into: MediaType::JSON,
            params: ("b", "c"), ("a", ";.,.;")
        };

        assert_parse_eq! {
            from: r#"*/*; a="a"; b="b"; a=a; b=b; c=c"#,
            into: MediaType::Any,
            params: ("a", "a"), ("b", "b"), ("a", "a"), ("b", "b"), ("c", "c")
        };
    }

    #[test]
    fn check_params_do_parse() {
        assert_parse!("*/*; q=1; q=2");
        assert_parse!("*/*; q=1;q=2;q=3;a=v;c=1;da=1;sdlkldsadasd=uhisdcb89");
        assert_parse!("*/*; q=1; q=2");
        assert_parse!("*/*; q=1; q=2; a=b;c=d;    e=f; a=s;a=e");
        assert_parse!("*/*; q=1; q=2 ; a=b");
        assert_parse!("*/*; q=1; q=2; hello=\"world !\"");
    }

    #[test]
    fn test_bad_parses() {
        assert_no_parse!("*&_/*)()");
        assert_no_parse!("/json");
        assert_no_parse!("text/");
        assert_no_parse!("text//");
        assert_no_parse!("/");
        assert_no_parse!("*/");
        assert_no_parse!("/*");
        assert_no_parse!("///");
        assert_no_parse!("application//json");
        assert_no_parse!("application///json");
        assert_no_parse!("a/b;");
        assert_no_parse!("*/*; a=b;;");
        assert_no_parse!("*/*; a=b;a");
        assert_no_parse!("*/*; a=b; ");
        assert_no_parse!("*/*; a=b;");
        assert_no_parse!("*/*; a = b");
        assert_no_parse!("*/*; a= b");
        assert_no_parse!("*/*; a =b");
        assert_no_parse!(r#"*/*; a="b"#);
        assert_no_parse!(r#"*/*; a="b; c=d"#);
        assert_no_parse!(r#"*/*; a="b; c=d"#);
        assert_no_parse!("*/*;a=@#$%^&*()");
        assert_no_parse!("*/*;;");
        assert_no_parse!("*/*;=;");
        assert_no_parse!("*/*=;");
        assert_no_parse!("*/*=;=");
    }
}
