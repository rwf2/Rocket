use pear::{ParseResult, ParseError};
use pear::parsers::*;

use http::parse::checkers::is_whitespace;
use http::parse::media_type::media_type;
use http::{MediaType, Accept, QMediaType};

fn q<'a>(_: &'a str, media_type: &MediaType) -> ParseResult<&'a str, Option<f32>> {
    match media_type.params().next() {
        Some(("q", value)) if value.len() <= 5 => match value.parse::<f32>().ok() {
            Some(q) if q > 1. => ParseError::custom("accept", "q value must be <= 1"),
            Some(q) if q < 0. => ParseError::custom("accept", "q value must be > 0"),
            Some(q) => ParseResult::Done(Some(q)),
            None => ParseError::custom("accept", "q value must be float")
        },
        _ => ParseResult::Done(None)
    }
}

#[parser]
fn accept<'a>(input: &mut &'a str) -> ParseResult<&'a str, Accept> {
    let mut media_types = vec![];
    repeat_while!(eat(','), {
        skip_while(is_whitespace);
        let media_type = media_type();
        let weight = q(&media_type);
        media_types.push(QMediaType(media_type, weight));
    });

    Accept::new(media_types)
}

pub fn parse_accept(mut input: &str) -> Result<Accept, ParseError<&str>> {
    parse!(&mut input, (accept(), eof()).0).into()
}

#[cfg(test)]
mod test {
    use http::MediaType;
    use super::parse_accept;

    macro_rules! assert_parse {
        ($string:expr) => ({
            match parse_accept($string) {
                Ok(accept) => accept,
                Err(e) => panic!("{:?} failed to parse: {}", $string, e)
            }
        });
    }

    macro_rules! assert_parse_eq {
        ($string:expr, [$($mt:expr),*]) => ({
            let expected = vec![$($mt),*];
            let result = assert_parse!($string);
            for (i, wmt) in result.iter().enumerate() {
                assert_eq!(wmt.media_type(), &expected[i]);
            }
        });
    }

    #[test]
    fn check_does_parse() {
        assert_parse!("text/html");
        assert_parse!("*/*, a/b; q=1.0; v=1, application/something, a/b");
        assert_parse!("a/b, b/c");
        assert_parse!("text/*");
        assert_parse!("text/*; q=1");
        assert_parse!("text/*; q=1; level=2");
        assert_parse!("audio/*; q=0.2, audio/basic");
        assert_parse!("text/plain; q=0.5, text/html, text/x-dvi; q=0.8, text/x-c");
        assert_parse!("text/*, text/html, text/html;level=1, */*");
        assert_parse!("text/*;q=0.3, text/html;q=0.7, text/html;level=1, \
               text/html;level=2;q=0.4, */*;q=0.5");
    }

    #[test]
    fn check_parse_eq() {
        assert_parse_eq!("text/html", [MediaType::HTML]);
        assert_parse_eq!("text/html, application/json",
                         [MediaType::HTML, MediaType::JSON]);
        assert_parse_eq!("text/html; charset=utf-8; v=1, application/json",
                         [MediaType::HTML, MediaType::JSON]);
        assert_parse_eq!("text/html, text/html; q=0.1, text/html; q=0.2",
                         [MediaType::HTML, MediaType::HTML, MediaType::HTML]);
    }
}
