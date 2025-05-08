use std::borrow::Cow;
use std::ops::Deref;
use std::str::FromStr;
use std::fmt;

use crate::{ContentCoding, Header};
use crate::parse::parse_accept_encoding;

/// The HTTP Accept-Encoding header.
///
/// An `AcceptEncoding` header is composed of zero or more content codings, each of which
/// may have an optional weight value (a [`QContentCoding`]). The header is sent by
/// an HTTP client to describe the formats it accepts as well as the order in
/// which it prefers different formats.
///
/// # Usage
///
/// The Accept-Encoding header of an incoming request can be retrieved via the
/// [`Request::accept_encoding()`] method. The [`preferred()`] method can be used to
/// retrieve the client's preferred content coding.
///
/// [`Request::accept_encoding()`]: rocket::Request::accept_encoding()
/// [`preferred()`]: AcceptEncoding::preferred()
///
/// An `AcceptEncoding` type with a single, common content coding can be easily constructed
/// via provided associated constants.
///
/// ## Example
///
/// Construct an `AcceptEncoding` header with a single `gzip` content coding:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::AcceptEncoding;
///
/// # #[allow(unused_variables)]
/// let accept_gzip = AcceptEncoding::GZIP;
/// ```
///
/// # Header
///
/// `AcceptEncoding` implements `Into<Header>`. As such, it can be used in any context
/// where an `Into<Header>` is expected:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::AcceptEncoding;
/// use rocket::response::Response;
///
/// let response = Response::build().header(AcceptEncoding::GZIP).finalize();
/// ```
#[derive(Debug, Clone)]
pub struct AcceptEncoding(pub(crate) Cow<'static, [QContentCoding]>);

/// A `ContentCoding` with an associated quality value.
#[derive(Debug, Clone, PartialEq)]
pub struct QContentCoding(pub ContentCoding, pub Option<f32>);

macro_rules! accept_encoding_constructor {
    ($($name:ident ($check:ident): $str:expr, $c:expr,)+) => {
        $(
            #[doc="An `AcceptEncoding` header with the single content coding for"]
            #[doc=concat!("**", $str, "**: ", "_", $c, "_")]
            #[allow(non_upper_case_globals)]
            pub const $name: AcceptEncoding = AcceptEncoding({
                const INNER: &[QContentCoding] = &[QContentCoding(ContentCoding::$name, None)];
                Cow::Borrowed(INNER)
            });
         )+
    };
}

impl AcceptEncoding {
    /// Constructs a new `AcceptEncoding` header from one or more media types.
    ///
    /// The `items` parameter may be of type `QContentCoding`, `[QContentCoding]`,
    /// `&[QContentCoding]` or `Vec<QContentCoding>`. To prevent additional allocations,
    /// prefer to provide inputs of type `QContentCoding`, `[QContentCoding]`, or
    /// `Vec<QContentCoding>`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{QContentCoding, ContentCoding, AcceptEncoding};
    ///
    /// // Construct an `Accept` via a `Vec<QMediaType>`.
    /// let gzip_then_deflate = vec![ContentCoding::GZIP, ContentCoding::DEFLATE];
    /// let accept = AcceptEncoding::new(gzip_then_deflate);
    /// assert_eq!(accept.preferred().media_type(), &ContentCoding::GZIP);
    ///
    /// // Construct an `Accept` via an `[QMediaType]`.
    /// let accept = Accept::new([MediaType::JSON.into(), MediaType::HTML.into()]);
    /// assert_eq!(accept.preferred().media_type(), &MediaType::JSON);
    ///
    /// // Construct an `Accept` via a `QMediaType`.
    /// let accept = Accept::new(QMediaType(MediaType::JSON, None));
    /// assert_eq!(accept.preferred().media_type(), &MediaType::JSON);
    /// ```
    #[inline(always)]
    pub fn new<T: IntoIterator<Item = M>, M: Into<QContentCoding>>(items: T) -> AcceptEncoding {
        AcceptEncoding(items.into_iter().map(|v| v.into()).collect())
    }

    // TODO: Implement this.
    // #[inline(always)]
    // pub fn add<M: Into<QContentCoding>>(&mut self, content_coding: M) {
    //     self.0.push(content_coding.into());
    // }

    /// Retrieve the client's preferred content coding. This method follows [RFC
    /// 7231 5.3.4]. If the list of content codings is empty, this method returns a
    /// content coding of any with no quality value: (`*`).
    ///
    /// [RFC 7231 5.3.4]: https://tools.ietf.org/html/rfc7231#section-5.3.4
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{QContentCoding, ContentCoding, AcceptEncoding};
    ///
    /// let qcontent_codings = vec![
    ///     QContentCoding(MediaType::DEFLATE, Some(0.3)),
    ///     QContentCoding(MediaType::GZIP, Some(0.9)),
    /// ];
    ///
    /// let accept = AcceptEncoding::new(qcontent_codings);
    /// assert_eq!(accept.preferred().content_coding(), &MediaType::GZIP);
    /// ```
    pub fn preferred(&self) -> &QContentCoding {
        static ANY: QContentCoding = QContentCoding(ContentCoding::Any, None);

        // See https://tools.ietf.org/html/rfc7231#section-5.3.4.
        let mut all = self.iter();
        let mut preferred = all.next().unwrap_or(&ANY);
        for content_coding in all {
            if content_coding.weight().is_none() && preferred.weight().is_some() {
                // Content coding without a `q` parameter are preferred.
                preferred = content_coding;
            } else if content_coding.weight_or(0.0) > preferred.weight_or(1.0) {
                // Prefer content coding with a greater weight, but if one doesn't
                // have a weight, prefer the one we already have.
                preferred = content_coding;
            }
        }

        preferred
    }

    /// Retrieve the first media type in `self`, if any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{QContentCoding, ContentCoding, AcceptEncoding};
    ///
    /// let accept_encoding = AcceptEncoding::new(QContentCoding(ContentCoding::GZIP, None));
    /// assert_eq!(accept_encoding.first(), Some(&ContentCoding::GZIP.into()));
    /// ```
    #[inline(always)]
    pub fn first(&self) -> Option<&QContentCoding> {
        self.iter().next()
    }

    /// Returns an iterator over all of the (quality) media types in `self`.
    /// Media types are returned in the order in which they appear in the
    /// header.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{QContentCoding, ContentCoding, AcceptEncoding};
    ///
    /// let qcontent_codings = vec![
    ///     QContentCoding(MediaType::DEFLATE, Some(0.3))
    ///     QContentCoding(MediaType::GZIP, Some(0.9)),
    /// ];
    ///
    /// let accept_encoding = AcceptEncoding::new(qcontent_codings.clone());
    ///
    /// let mut iter = accept.iter();
    /// assert_eq!(iter.next(), Some(&qcontent_codings[0]));
    /// assert_eq!(iter.next(), Some(&qcontent_codings[1]));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item=&'_ QContentCoding> + '_ {
        self.0.iter()
    }

    /// Returns an iterator over all of the (bare) media types in `self`. Media
    /// types are returned in the order in which they appear in the header.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{QMediaType, MediaType, Accept};
    ///
    /// let qmedia_types = vec![
    ///     QMediaType(MediaType::JSON, Some(0.3)),
    ///     QMediaType(MediaType::HTML, Some(0.9))
    /// ];
    ///
    /// let accept = Accept::new(qmedia_types.clone());
    ///
    /// let mut iter = accept.media_types();
    /// assert_eq!(iter.next(), Some(qmedia_types[0].media_type()));
    /// assert_eq!(iter.next(), Some(qmedia_types[1].media_type()));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline(always)]
    pub fn content_codings(&self) -> impl Iterator<Item=&'_ ContentCoding> + '_ {
        self.iter().map(|weighted_cc| weighted_cc.content_coding())
    }

    known_content_codings!(accept_encoding_constructor);
}

impl<T: IntoIterator<Item = ContentCoding>> From<T> for AcceptEncoding {
    #[inline(always)]
    fn from(items: T) -> AcceptEncoding {
        AcceptEncoding::new(items.into_iter().map(QContentCoding::from))
    }
}

impl PartialEq for AcceptEncoding {
    fn eq(&self, other: &AcceptEncoding) -> bool {
        self.iter().eq(other.iter())
    }
}

impl fmt::Display for AcceptEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, content_coding) in self.iter().enumerate() {
            if i >= 1 {
                write!(f, ", {}", content_coding.0)?;
            } else {
                write!(f, "{}", content_coding.0)?;
            }
        }

        Ok(())
    }
}

impl FromStr for AcceptEncoding {
    // Ideally we'd return a `ParseError`, but that requires a lifetime.
    type Err = String;

    #[inline]
    fn from_str(raw: &str) -> Result<AcceptEncoding, String> {
        parse_accept_encoding(raw).map_err(|e| e.to_string())
    }
}

/// Creates a new `Header` with name `Accept-Encoding` and the value set to the HTTP
/// rendering of this `Accept` header.
impl From<AcceptEncoding> for Header<'static> {
    #[inline(always)]
    fn from(val: AcceptEncoding) -> Self {
        Header::new("Accept-Encoding", val.to_string())
    }
}

impl QContentCoding {
    /// Retrieve the weight of the media type, if there is any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{ContentCoding, QContentCoding};
    ///
    /// let q_coding = QContentCoding(ContentCoding::GZIP, Some(0.3));
    /// assert_eq!(q_coding.weight(), Some(0.3));
    /// ```
    #[inline(always)]
    pub fn weight(&self) -> Option<f32> {
        self.1
    }

    /// Retrieve the weight of the media type or a given default value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{ContentCoding, QContentCoding};
    ///
    /// let q_coding = QContentCoding(ContentCoding::GZIP, Some(0.3));
    /// assert_eq!(q_coding.weight_or(0.9), 0.3);
    ///
    /// let q_coding = QContentCoding(ContentCoding::GZIP, None);
    /// assert_eq!(q_coding.weight_or(0.9), 0.9);
    /// ```
    #[inline(always)]
    pub fn weight_or(&self, default: f32) -> f32 {
        self.1.unwrap_or(default)
    }

    /// Borrow the internal `MediaType`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{ContentCoding, QContentCoding};
    ///
    /// let q_coding = QContentCoding(ContentCoding::GZIP, Some(0.3));
    /// assert_eq!(q_coding.content_coding(), &ContentCoding::GZIP);
    /// ```
    #[inline(always)]
    pub fn content_coding(&self) -> &ContentCoding {
        &self.0
    }
}

impl From<ContentCoding> for QContentCoding {
    #[inline(always)]
    fn from(content_coding: ContentCoding) -> QContentCoding {
        QContentCoding(content_coding, None)
    }
}

impl Deref for QContentCoding {
    type Target = ContentCoding;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use crate::{AcceptEncoding, ContentCoding};

    #[track_caller]
    fn assert_preference(string: &str, expect: &str) {
        let ae: AcceptEncoding = string.parse().expect("accept_encoding string parse");
        let expected: ContentCoding = expect.parse().expect("content coding parse");
        let preferred = ae.preferred();
        let actual = preferred.content_coding();
        if *actual != expected {
            panic!("mismatch for {}: expected {}, got {}", string, expected, actual)
        }
    }

    #[test]
    fn test_preferred() {
        assert_preference("deflate", "deflate");
        assert_preference("gzip, deflate", "gzip");
        assert_preference("deflate; q=0.1, gzip", "gzip");
        assert_preference("gzip; q=1, gzip", "gzip");

        assert_preference("gzip, deflate; q=1", "gzip");
        assert_preference("deflate; q=1, gzip", "gzip");

        assert_preference("gzip; q=0.1, gzip; q=0.2", "gzip; q=0.2");
        assert_preference("rar; q=0.1, compress; q=0.2", "compress; q=0.2");
        assert_preference("rar; q=0.5, compress; q=0.2", "rar; q=0.5");

        assert_preference("rar; q=0.5, compress; q=0.2, nonsense", "nonsense");
    }
}
