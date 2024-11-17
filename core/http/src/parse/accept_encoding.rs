use pear::macros::parser;
use pear::combinators::{series, surrounded};

use crate::{AcceptEncoding, QContentCoding};
use crate::parse::checkers::is_whitespace;
use crate::parse::content_coding::content_coding;

type Input<'a> = pear::input::Pear<pear::input::Cursor<&'a str>>;
type Result<'a, T> = pear::input::Result<T, Input<'a>>;

#[parser]
fn weighted_content_coding<'a>(input: &mut Input<'a>) -> Result<'a, QContentCoding> {
    let content_coding = content_coding()?;
    let weight = match content_coding.weight() {
        Some(v) => Some(*v),
        _ => None
    };

    QContentCoding(content_coding, weight)
}

#[parser]
fn accept_encoding<'a>(input: &mut Input<'a>) -> Result<'a, AcceptEncoding> {
    let vec = series(|i| surrounded(i, weighted_content_coding, is_whitespace), ',')?;
    AcceptEncoding(std::borrow::Cow::Owned(vec))
}

pub fn parse_accept_encoding(input: &str) -> Result<'_, AcceptEncoding> {
    parse!(accept_encoding: Input::new(input))
}

#[cfg(test)]
mod test {
    use crate::ContentCoding;
    use super::parse_accept_encoding;

    macro_rules! assert_parse {
        ($string:expr) => ({
            match parse_accept_encoding($string) {
                Ok(ae) => ae,
                Err(e) => panic!("{:?} failed to parse: {}", $string, e)
            }
        });
    }

    macro_rules! assert_parse_eq {
        ($string:expr, [$($cc:expr),*]) => ({
            let expected = vec![$($cc),*];
            let result = assert_parse!($string);
            for (i, wcc) in result.iter().enumerate() {
                assert_eq!(wcc.content_coding(), &expected[i]);
            }
        });
    }

    #[test]
    fn check_does_parse() {
        assert_parse!("gzip");
        assert_parse!("gzip; q=1");
        assert_parse!("*, gzip; q=1.0, rar, deflate");
        assert_parse!("rar, deflate");
        assert_parse!("deflate;q=0.3, gzip;q=0.7, rar;q=0.4, *;q=0.5");
    }

    #[test]
    fn check_parse_eq() {
        assert_parse_eq!("gzip", [ContentCoding::GZIP]);
        assert_parse_eq!("gzip, deflate",
                         [ContentCoding::GZIP, ContentCoding::new("deflate")]);
        assert_parse_eq!("gzip; q=1, deflate",
                         [ContentCoding::GZIP, ContentCoding::new("deflate")]);
        assert_parse_eq!("gzip, gzip; q=0.1, gzip; q=0.2",
                         [ContentCoding::GZIP, ContentCoding::GZIP, ContentCoding::GZIP]);
    }
}
