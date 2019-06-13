use std::ops::Deref;
use std::str::FromStr;
use std::fmt;

use smallvec::SmallVec;

use crate::{Header, MediaType};
use crate::ext::IntoCollection;
use crate::parse::parse_accept;

/// A `MediaType` with an associated quality value.
#[derive(Debug, Clone, PartialEq)]
pub struct QMediaType(pub MediaType, pub Option<f32>);

impl QMediaType {
    /// Retrieve the weight of the media type, if there is any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{MediaType, QMediaType};
    ///
    /// let q_type = QMediaType(MediaType::HTML, Some(0.3));
    /// assert_eq!(q_type.weight(), Some(0.3));
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
    /// use rocket::http::{MediaType, QMediaType};
    ///
    /// let q_type = QMediaType(MediaType::HTML, Some(0.3));
    /// assert_eq!(q_type.weight_or(0.9), 0.3);
    ///
    /// let q_type = QMediaType(MediaType::HTML, None);
    /// assert_eq!(q_type.weight_or(0.9), 0.9);
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
    /// use rocket::http::{MediaType, QMediaType};
    ///
    /// let q_type = QMediaType(MediaType::HTML, Some(0.3));
    /// assert_eq!(q_type.media_type(), &MediaType::HTML);
    /// ```
    #[inline(always)]
    pub fn media_type(&self) -> &MediaType {
        &self.0
    }
}

impl From<MediaType> for QMediaType {
    #[inline(always)]
    fn from(media_type: MediaType) -> QMediaType {
        QMediaType(media_type, None)
    }
}

impl Deref for QMediaType {
    type Target = MediaType;

    #[inline(always)]
    fn deref(&self) -> &MediaType {
        &self.0
    }
}

// FIXME: `Static` is needed for `const` items. Need `const SmallVec::new`.
#[derive(Debug, Clone)]
pub enum AcceptParams {
    Static(&'static [QMediaType]),
    Dynamic(SmallVec<[QMediaType; 1]>)
}

impl pear::parsers::Collection for AcceptParams {
    type Item = QMediaType;

    fn new() -> Self {
        AcceptParams::Dynamic(SmallVec::new())
    }

    fn add(&mut self, item: Self::Item) {
        match *self {
            AcceptParams::Static(..) => panic!("can't add to static collection!"),
            AcceptParams::Dynamic(ref mut v) => v.push(item)
        }
    }
}

impl PartialEq for AcceptParams {
    fn eq(&self, other: &AcceptParams) -> bool {
        #[inline(always)]
        fn inner_types(params: &AcceptParams) -> &[QMediaType] {
            match *params {
                AcceptParams::Static(params) => params,
                AcceptParams::Dynamic(ref vec) => vec,
            }
        }

        inner_types(self) == inner_types(other)
    }
}

/// The HTTP Accept header.
///
/// An `Accept` header is composed of zero or more media types, each of which
/// may have an optional quality value (a [`QMediaType`]). The header is sent by
/// an HTTP client to describe the formats it accepts as well as the order in
/// which it prefers different formats.
///
/// # Usage
///
/// The Accept header of an incoming request can be retrieved via the
/// [`Request::accept()`] method. The [`preferred()`] method can be used to
/// retrieve the client's preferred media type.
///
/// [`Request::accept`]: rocket::Request::accept()
/// [`preferred()`]: Accept::preferred()
///
/// An `Accept` type with a single, common media type can be easily constructed
/// via provided associated constants.
///
/// ## Example
///
/// Construct an `Accept` header with a single `application/json` media type:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::Accept;
///
/// # #[allow(unused_variables)]
/// let accept_json = Accept::JSON;
/// ```
///
/// # Header
///
/// `Accept` implements `Into<Header>`. As such, it can be used in any context
/// where an `Into<Header>` is expected:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::Accept;
/// use rocket::response::Response;
///
/// # #[allow(unused_variables)]
/// let response = Response::build().header(Accept::JSON).finalize();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Accept(crate AcceptParams);

macro_rules! accept_constructor {
    ($($name:ident ($check:ident): $str:expr, $t:expr,
        $s:expr $(; $k:expr => $v:expr)*,)+) => {
        $(
            #[doc="An `Accept` header with the single media type for <b>"]
            #[doc=$str] #[doc="</b>: <i>"]
            #[doc=$t] #[doc="/"] #[doc=$s]
            #[doc="</i>"]
            #[allow(non_upper_case_globals)]
            pub const $name: Accept = Accept(
                AcceptParams::Static(&[QMediaType(MediaType::$name, None)])
            );
         )+
    };
}

impl<T: IntoCollection<MediaType>> From<T> for Accept {
    #[inline(always)]
    fn from(items: T) -> Accept {
        Accept(AcceptParams::Dynamic(items.mapped(|item| item.into())))
    }
}

impl Accept {
    /// Constructs a new `Accept` header from one or more media types.
    ///
    /// The `items` parameter may be of type `QMediaType`, `&[QMediaType]`, or
    /// `Vec<QMediaType>`. To prevent additional allocations, prefer to provide
    /// inputs of type `QMediaType` and `Vec<QMediaType>`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{QMediaType, MediaType, Accept};
    ///
    /// // Construct an `Accept` via a `Vec<QMediaType>`.
    /// let json_then_html = vec![MediaType::JSON.into(), MediaType::HTML.into()];
    /// let accept = Accept::new(json_then_html);
    /// assert_eq!(accept.preferred().media_type(), &MediaType::JSON);
    ///
    /// // Construct an `Accept` via an `&[QMediaType]`.
    /// let accept = Accept::new(&[MediaType::JSON.into(), MediaType::HTML.into()]);
    /// assert_eq!(accept.preferred().media_type(), &MediaType::JSON);
    ///
    /// // Construct an `Accept` via a `QMediaType`.
    /// let accept = Accept::new(QMediaType(MediaType::JSON, None));
    /// assert_eq!(accept.preferred().media_type(), &MediaType::JSON);
    /// ```
    #[inline(always)]
    pub fn new<T: IntoCollection<QMediaType>>(items: T) -> Accept {
        Accept(AcceptParams::Dynamic(items.into_collection()))
    }

    // TODO: Implement this.
    // #[inline(always)]
    // pub fn add<M: Into<QMediaType>>(&mut self, media_type: M) {
    //     self.0.push(media_type.into());
    // }

    /// Retrieve the client's preferred media type. This method follows [RFC
    /// 7231 5.3.2]. If the list of media types is empty, this method returns a
    /// media type of any with no quality value: (`*/*`).
    ///
    /// [RFC 7231 5.3.2]: https://tools.ietf.org/html/rfc7231#section-5.3.2
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{QMediaType, MediaType, Accept};
    ///
    /// let media_types = vec![
    ///     QMediaType(MediaType::JSON, Some(0.3)),
    ///     QMediaType(MediaType::HTML, Some(0.9))
    /// ];
    ///
    /// let accept = Accept::new(media_types);
    /// assert_eq!(accept.preferred().media_type(), &MediaType::HTML);
    /// ```
    pub fn preferred(&self) -> &QMediaType {
        static ANY: QMediaType = QMediaType(MediaType::Any, None);

        // See https://tools.ietf.org/html/rfc7231#section-5.3.2.
        let mut all = self.iter();
        let mut preferred = all.next().unwrap_or(&ANY);
        for media_type in all {
            if media_type.weight().is_none() && preferred.weight().is_some() {
                // Media types without a `q` parameter are preferred.
                preferred = media_type;
            } else if media_type.weight_or(0.0) > preferred.weight_or(1.0) {
                // Prefer media types with a greater weight, but if one doesn't
                // have a weight, prefer the one we already have.
                preferred = media_type;
            } else if media_type.specificity() > preferred.specificity() {
                // Prefer more specific media types over less specific ones. IE:
                // text/html over application/*.
                preferred = media_type;
            } else if media_type == preferred {
                // Finally, all other things being equal, prefer a media type
                // with more parameters over one with fewer. IE: text/html; a=b
                // over text/html.
                if media_type.params().count() > preferred.params().count() {
                    preferred = media_type;
                }
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
    /// use rocket::http::{QMediaType, MediaType, Accept};
    ///
    /// let accept = Accept::new(QMediaType(MediaType::XML, None));
    /// assert_eq!(accept.first(), Some(&MediaType::XML.into()));
    /// ```
    #[inline(always)]
    pub fn first(&self) -> Option<&QMediaType> {
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
    /// use rocket::http::{QMediaType, MediaType, Accept};
    ///
    /// let qmedia_types = vec![
    ///     QMediaType(MediaType::JSON, Some(0.3)),
    ///     QMediaType(MediaType::HTML, Some(0.9))
    /// ];
    ///
    /// let accept = Accept::new(qmedia_types.clone());
    ///
    /// let mut iter = accept.iter();
    /// assert_eq!(iter.next(), Some(&qmedia_types[0]));
    /// assert_eq!(iter.next(), Some(&qmedia_types[1]));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline(always)]
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&'a QMediaType> + 'a {
        let slice = match self.0 {
            AcceptParams::Static(slice) => slice,
            AcceptParams::Dynamic(ref vec) => &vec[..],
        };

        slice.iter()
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
    pub fn media_types<'a>(&'a self) -> impl Iterator<Item=&'a MediaType> + 'a {
        self.iter().map(|weighted_mt| weighted_mt.media_type())
    }

    known_media_types!(accept_constructor);
}

impl fmt::Display for Accept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, media_type) in self.iter().enumerate() {
            if i >= 1 {
                write!(f, ", {}", media_type.0)?;
            } else {
                write!(f, "{}", media_type.0)?;
            }
        }

        Ok(())
    }
}

impl FromStr for Accept {
    // Ideally we'd return a `ParseError`, but that requires a lifetime.
    type Err = String;

    #[inline]
    fn from_str(raw: &str) -> Result<Accept, String> {
        parse_accept(raw).map_err(|e| e.to_string())
    }
}

/// Creates a new `Header` with name `Accept` and the value set to the HTTP
/// rendering of this `Accept` header.
impl Into<Header<'static>> for Accept {
    #[inline(always)]
    fn into(self) -> Header<'static> {
        Header::new("Accept", self.to_string())
    }
}

#[cfg(test)]
mod test {
    use crate::{Accept, MediaType};

    macro_rules! assert_preference {
        ($string:expr, $expect:expr) => (
            let accept: Accept = $string.parse().expect("accept string parse");
            let expected: MediaType = $expect.parse().expect("media type parse");
            let preferred = accept.preferred();
            assert_eq!(preferred.media_type().to_string(), expected.to_string());
        )
    }

    #[test]
    fn test_preferred() {
        assert_preference!("text/*", "text/*");
        assert_preference!("text/*, text/html", "text/html");
        assert_preference!("text/*; q=0.1, text/html", "text/html");
        assert_preference!("text/*; q=1, text/html", "text/html");
        assert_preference!("text/html, text/*", "text/html");
        assert_preference!("text/*, text/html", "text/html");
        assert_preference!("text/html, text/*; q=1", "text/html");
        assert_preference!("text/html; q=1, text/html", "text/html");
        assert_preference!("text/html, text/*; q=0.1", "text/html");

        assert_preference!("text/html, application/json", "text/html");
        assert_preference!("text/html, application/json; q=1", "text/html");
        assert_preference!("application/json; q=1, text/html", "text/html");

        assert_preference!("text/*, application/json", "application/json");
        assert_preference!("*/*, text/*", "text/*");
        assert_preference!("*/*, text/*, text/plain", "text/plain");

        assert_preference!("a/b; q=0.1, a/b; q=0.2", "a/b; q=0.2");
        assert_preference!("a/b; q=0.1, b/c; q=0.2", "b/c; q=0.2");
        assert_preference!("a/b; q=0.5, b/c; q=0.2", "a/b; q=0.5");

        assert_preference!("a/b; q=0.5, b/c; q=0.2, c/d", "c/d");
        assert_preference!("a/b; q=0.5; v=1, a/b", "a/b");

        assert_preference!("a/b; v=1, a/b; v=1; c=2", "a/b; v=1; c=2");
        assert_preference!("a/b; v=1; c=2, a/b; v=1", "a/b; v=1; c=2");
        assert_preference!("a/b; q=0.5; v=1, a/b; q=0.5; v=1; c=2",
            "a/b; q=0.5; v=1; c=2");
        assert_preference!("a/b; q=0.6; v=1, a/b; q=0.5; v=1; c=2",
            "a/b; q=0.6; v=1");
    }
}
