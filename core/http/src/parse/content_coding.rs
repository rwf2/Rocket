use std::borrow::Cow;

use pear::input::Extent;
use pear::macros::{parser, parse};
use pear::parsers::*;
use pear::combinators::surrounded;

use crate::header::{ContentCoding, Source};
use crate::parse::checkers::{is_valid_token, is_whitespace};

type Input<'a> = pear::input::Pear<pear::input::Cursor<&'a str>>;
type Result<'a, T> = pear::input::Result<T, Input<'a>>;

#[parser]
fn coding_param<'a>(input: &mut Input<'a>) -> Result<'a, Extent<&'a str>> {
    let _ = (take_some_while_until(|c| matches!(c, 'Q' | 'q'), '=')?, eat('=')?).0;
    let value = take_some_while_until(|c| matches!(c, '0'..='9' | '.'), ';')?;

    value
}

#[parser]
pub fn content_coding<'a>(input: &mut Input<'a>) -> Result<'a, ContentCoding> {
    let (coding, weight) = {
        let coding = take_some_while_until(is_valid_token, ';')?;
        let weight = match eat(input, ';') {
            Ok(_) => surrounded(coding_param, is_whitespace)?,
            Err(_) => Extent {start: 0, end: 0, values: ""},
        };

        (coding, weight)
    };

    let weight = match weight.len() {
        len if len > 0 && len <= 5 => match weight.parse::<f32>().ok() {
            Some(q) if q > 1. => parse_error!("q value must be <= 1")?,
            Some(q) if q < 0. => parse_error!("q value must be > 0")?,
            Some(q) => Some(q),
            None => parse_error!("invalid content coding weight")?
        },
        _ => None,
    };

    ContentCoding {
        weight: weight,
        source: Source::Custom(Cow::Owned(input.start.to_string())),
        coding: coding.into(),
    }
}

pub fn parse_content_coding(input: &str) -> Result<'_, ContentCoding> {
    parse!(content_coding: Input::new(input))
}

#[cfg(test)]
mod test {
    use crate::ContentCoding;
    use super::parse_content_coding;

    macro_rules! assert_no_parse {
        ($string:expr) => ({
            let result: Result<_, _> = parse_content_coding($string).into();
            if result.is_ok() {
                panic!("{:?} parsed unexpectedly.", $string)
            }
        });
    }

    macro_rules! assert_parse {
        ($string:expr) => ({
            match parse_content_coding($string) {
                Ok(content_coding) => content_coding,
                Err(e) => panic!("{:?} failed to parse: {}", $string, e)
            }
        });
    }

    macro_rules! assert_parse_eq {
        (@full $string:expr, $result:expr, $weight:expr) => ({
            let result = assert_parse!($string);
            assert_eq!(result, $result);

            assert_eq!(*result.weight(), $weight);
        });

        (from: $string:expr, into: $result:expr)
            => (assert_parse_eq!(@full $string, $result, None));
        (from: $string:expr, into: $result:expr, weight: $weight:literal)
            => (assert_parse_eq!(@full $string, $result, Some($weight)));
    }

    #[test]
    fn check_does_parse() {
        assert_parse!("*");
        assert_parse!("rar");
        assert_parse!("gzip");
        assert_parse!("identity");
    }

    #[test]
    fn check_parse_eq() {
        assert_parse_eq!(from: "gzip", into: ContentCoding::GZIP);
        assert_parse_eq!(from: "gzip; q=1", into: ContentCoding::GZIP, weight: 1f32);

        assert_parse_eq!(from: "*", into: ContentCoding::Any);
        assert_parse_eq!(from: "rar", into: ContentCoding::new("rar"));
    }

    #[test]
    fn check_params_do_parse() {
        assert_parse!("*; q=1");
    }
    
    #[test]
    fn test_bad_parses() {
        assert_no_parse!("*; q=1;");
        assert_no_parse!("*; q=1; q=2");
    }
}
