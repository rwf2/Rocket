use pear::macros::{parser, parse_error};
use pear::combinators::{series, surrounded};

use crate::{Accept, QMediaType};
use crate::parse::checkers::is_whitespace;
use crate::parse::media_type::media_type;

type Input<'a> = pear::input::Pear<pear::input::Cursor<&'a str>>;
type Result<'a, T> = pear::input::Result<T, Input<'a>>;

#[parser]
fn weighted_media_type<'a>(input: &mut Input<'a>) -> Result<'a, QMediaType> {
    let media_type = media_type()?;
    let q = match media_type.params().next() {
        Some((name, value)) if name  == "q" => Some(value),
        _ => None
    };

    let weight = match q {
        Some(value) if value.len() <= 5 => match value.parse::<f32>().ok() {
            Some(q) if q > 1. => parse_error!("q value must be <= 1")?,
            Some(q) if q < 0. => parse_error!("q value must be > 0")?,
            Some(q) => Some(q),
            None => parse_error!("invalid media-type weight")?
        },
        _ => None
    };

    QMediaType(media_type, weight)
}

#[parser]
fn accept<'a>(input: &mut Input<'a>) -> Result<'a, Accept> {
    let vec = series(|i| surrounded(i, weighted_media_type, is_whitespace), ',')?;
    Accept(std::borrow::Cow::Owned(vec))
}

pub fn parse_accept(input: &str) -> Result<'_, Accept> {
    parse!(accept: Input::new(input))
}

#[cfg(test)]
mod test {
    use crate::MediaType;
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
