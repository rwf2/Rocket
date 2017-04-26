use std::ops::Deref;
use std::str::FromStr;
use std::fmt;

use smallvec::SmallVec;

use ext::IntoCollection;
use http::{Header, MediaType};
use http::parse::parse_accept;

#[derive(Debug, Clone, PartialEq)]
pub struct WeightedMediaType(pub MediaType, pub Option<f32>);

impl WeightedMediaType {
    #[inline(always)]
    pub fn weight(&self) -> Option<f32> {
        self.1
    }

    #[inline(always)]
    pub fn weight_or(&self, default: f32) -> f32 {
        self.1.unwrap_or(default)
    }

    #[inline(always)]
    pub fn media_type(&self) -> &MediaType {
        &self.0
    }

    #[inline(always)]
    pub fn into_media_type(self) -> MediaType {
        self.0
    }
}

impl From<MediaType> for WeightedMediaType {
    #[inline(always)]
    fn from(media_type: MediaType) -> WeightedMediaType {
        WeightedMediaType(media_type, None)
    }
}

impl Deref for WeightedMediaType {
    type Target = MediaType;

    #[inline(always)]
    fn deref(&self) -> &MediaType {
        &self.0
    }
}

// FIXME: `Static` is needed for `const` items. Need `const SmallVec::new`.
#[derive(Debug, PartialEq, Clone)]
pub enum AcceptParams {
    Static(&'static [WeightedMediaType]),
    Dynamic(SmallVec<[WeightedMediaType; 1]>)
}

/// The HTTP Accept header.
#[derive(Debug, Clone, PartialEq)]
pub struct Accept(AcceptParams);

macro_rules! accept_constructor {
    ($($name:ident ($check:ident): $str:expr, $t:expr,
        $s:expr $(; $k:expr => $v:expr)*),+) => {
        $(
            #[doc="An `Accept` header with the single media type for <b>"]
            #[doc=$str] #[doc="</b>: <i>"]
            #[doc=$t] #[doc="/"] #[doc=$s]
            #[doc="</i>"]
            #[allow(non_upper_case_globals)]
            pub const $name: Accept = Accept(
                AcceptParams::Static(&[WeightedMediaType(MediaType::$name, None)])
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
    #[inline(always)]
    pub fn new<T: IntoCollection<WeightedMediaType>>(items: T) -> Accept {
        Accept(AcceptParams::Dynamic(items.into_collection()))
    }

    // FIXME: IMPLEMENT THIS.
    // #[inline(always)]
    // pub fn add<M: Into<WeightedMediaType>>(&mut self, media_type: M) {
    //     self.0.push(media_type.into());
    // }

    pub fn preferred(&self) -> &WeightedMediaType {
        static ANY: WeightedMediaType = WeightedMediaType(MediaType::Any, None);

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

    // */html, text/plain

    #[inline(always)]
    pub fn first(&self) -> Option<&WeightedMediaType> {
        self.iter().next()
    }

    #[inline(always)]
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&'a WeightedMediaType> + 'a {
        let slice = match self.0 {
            AcceptParams::Static(slice) => slice,
            AcceptParams::Dynamic(ref vec) => &vec[..],
        };

        slice.iter()
    }

    #[inline(always)]
    pub fn media_types<'a>(&'a self) -> impl Iterator<Item=&'a MediaType> + 'a {
        self.iter().map(|weighted_mt| weighted_mt.media_type())
    }

    known_media_types!(accept_constructor);
}

impl fmt::Display for Accept {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, media_type) in self.iter().enumerate() {
            if i >= 1 { write!(f, ", ")?; }
            write!(f, "{}", media_type.0)?;
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
    use http::{Accept, MediaType};

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
