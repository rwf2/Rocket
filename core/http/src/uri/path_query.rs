use std::hash::Hash;
use std::borrow::Cow;

use state::InitCell;

use crate::{RawStr, ext::IntoOwned};
use crate::uri::Segments;
use crate::uri::fmt::{self, Part};
use crate::parse::{IndexedStr, Extent};

// INTERNAL DATA STRUCTURE.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Data<'a, P: Part> {
    pub(crate) value: IndexedStr<'a>,
    pub(crate) decoded_segments: InitCell<Vec<P::Raw>>,
}

impl<'a, P: Part> Data<'a, P> {
    pub(crate) fn raw(value: Extent<&'a [u8]>) -> Self {
        Data { value: value.into(), decoded_segments: InitCell::new() }
    }

    // INTERNAL METHOD.
    #[doc(hidden)]
    pub fn new<S: Into<Cow<'a, str>>>(value: S) -> Self {
        Data {
            value: IndexedStr::from(value.into()),
            decoded_segments: InitCell::new(),
        }
    }
}

/// A URI path: `/foo/bar`, `foo/bar`, etc.
#[derive(Debug, Clone, Copy)]
pub struct Path<'a> {
    pub(crate) source: &'a Option<Cow<'a, str>>,
    pub(crate) data: &'a Data<'a, fmt::Path>,
}

/// A URI query: `?foo&bar`.
#[derive(Debug, Clone, Copy)]
pub struct Query<'a> {
    pub(crate) source: &'a Option<Cow<'a, str>>,
    pub(crate) data: &'a Data<'a, fmt::Query>,
}

fn decode_to_indexed_str<P: fmt::Part>(
    value: &RawStr,
    (indexed, source): (&IndexedStr<'_>, &RawStr)
) -> IndexedStr<'static> {
    let decoded = match P::KIND {
        fmt::Kind::Path => value.percent_decode_lossy(),
        fmt::Kind::Query => value.url_decode_lossy(),
    };

    match decoded {
        Cow::Borrowed(b) if indexed.is_indexed() => {
            let checked = IndexedStr::checked_from(b, source.as_str());
            debug_assert!(checked.is_some(), "\nunindexed {:?} in {:?} {:?}", b, indexed, source);
            checked.unwrap_or_else(|| IndexedStr::from(Cow::Borrowed("")))
        }
        cow => IndexedStr::from(Cow::Owned(cow.into_owned())),
    }
}

impl<'a> Path<'a> {
    /// Returns the raw path value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/foo%20bar%2dbaz");
    /// assert_eq!(uri.path(), "/foo%20bar%2dbaz");
    /// assert_eq!(uri.path().raw(), "/foo%20bar%2dbaz");
    /// ```
    pub fn raw(&self) -> &'a RawStr {
        self.data.value.from_cow_source(self.source).into()
    }

    /// Returns the raw, undecoded path value as an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/foo%20bar%2dbaz");
    /// assert_eq!(uri.path(), "/foo%20bar%2dbaz");
    /// assert_eq!(uri.path().as_str(), "/foo%20bar%2dbaz");
    /// ```
    pub fn as_str(&self) -> &'a str {
        self.raw().as_str()
    }

    /// Whether `self` is normalized, i.e, it has no empty segments except the
    /// last one.
    ///
    /// If `absolute`, then a starting  `/` is required.
    pub(crate) fn is_normalized(&self, absolute: bool) -> bool {
        if absolute && !self.raw().starts_with('/') {
            return false;
        }

        self.raw_segments()
            .rev()
            .skip(1)
            .all(|s| !s.is_empty())
    }

    /// Normalizes `self`. If `absolute`, a starting  `/` is required. If
    /// `trail`, a trailing slash is allowed. Otherwise it is not.
    pub(crate) fn to_normalized(self, absolute: bool, trail: bool) -> Data<'static, fmt::Path> {
        let raw = self.raw().trim();
        let mut path = String::with_capacity(raw.len());

        if absolute || raw.starts_with('/') {
            path.push('/');
        }

        for (i, segment) in self.raw_segments().filter(|s| !s.is_empty()).enumerate() {
            if i != 0 { path.push('/'); }
            path.push_str(segment.as_str());
        }

        if trail && raw.len() > 1 && raw.ends_with('/') && !path.ends_with('/') {
            path.push('/');
        }

        Data {
            value: IndexedStr::from(Cow::Owned(path)),
            decoded_segments: InitCell::new(),
        }
    }

    /// Returns an iterator over the raw, undecoded segments, potentially empty
    /// segments.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/").unwrap();
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &[""]);
    ///
    /// let uri = Origin::parse("//").unwrap();
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &["", ""]);
    ///
    /// let uri = Origin::parse("/foo").unwrap();
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &["foo"]);
    ///
    /// let uri = Origin::parse("/a/").unwrap();
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &["a", ""]);
    ///
    /// // Recall that `uri!()` normalizes static inputs.
    /// let uri = uri!("//");
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &[""]);
    ///
    /// let uri = Origin::parse("/a//b///c/d?query&param").unwrap();
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &["a", "", "b", "", "", "c", "d"]);
    /// ```
    #[inline]
    pub fn raw_segments(&self) -> impl DoubleEndedIterator<Item = &'a RawStr> {
        let raw = self.raw().trim();
        raw.strip_prefix(fmt::Path::DELIMITER)
            .unwrap_or(raw)
            .split(fmt::Path::DELIMITER)
    }

    /// Returns a (smart) iterator over the percent-decoded segments. Empty
    /// segments between non-empty segments are skipped. A trailing slash will
    /// result in an empty segment emitted as the final item.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/").unwrap();
    /// let path_segs: Vec<&str> = uri.path().segments().collect();
    /// assert_eq!(path_segs, &[""]);
    ///
    /// let uri = Origin::parse("/a").unwrap();
    /// let path_segs: Vec<&str> = uri.path().segments().collect();
    /// assert_eq!(path_segs, &["a"]);
    ///
    /// let uri = Origin::parse("/a/").unwrap();
    /// let path_segs: Vec<&str> = uri.path().segments().collect();
    /// assert_eq!(path_segs, &["a", ""]);
    ///
    /// let uri = Origin::parse("/foo/bar").unwrap();
    /// let path_segs: Vec<&str> = uri.path().segments().collect();
    /// assert_eq!(path_segs, &["foo", "bar"]);
    ///
    /// let uri = Origin::parse("/foo///bar").unwrap();
    /// let path_segs: Vec<&str> = uri.path().segments().collect();
    /// assert_eq!(path_segs, &["foo", "bar"]);
    ///
    /// let uri = Origin::parse("/foo///bar//").unwrap();
    /// let path_segs: Vec<&str> = uri.path().segments().collect();
    /// assert_eq!(path_segs, &["foo", "bar", ""]);
    ///
    /// let uri = Origin::parse("/a%20b/b%2Fc/d//e?query=some").unwrap();
    /// let path_segs: Vec<&str> = uri.path().segments().collect();
    /// assert_eq!(path_segs, &["a b", "b/c", "d", "e"]);
    /// ```
    pub fn segments(&self) -> Segments<'a, fmt::Path> {
        let raw = self.raw();
        let cached = self.data.decoded_segments.get_or_init(|| {
            let mut segments = vec![];
            let mut raw_segments = self.raw_segments().peekable();
            while let Some(s) = raw_segments.next() {
                // Only allow an empty segment if it's the last one.
                if s.is_empty() && raw_segments.peek().is_some() {
                    continue;
                }

                segments.push(decode_to_indexed_str::<fmt::Path>(s, (&self.data.value, raw)));
            }

            segments
        });

        Segments::new(raw, cached)
    }
}

impl<'a> Query<'a> {
    /// Returns the raw, undecoded query value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/foo?baz+bar");
    /// assert_eq!(uri.query().unwrap(), "baz+bar");
    /// assert_eq!(uri.query().unwrap().raw(), "baz+bar");
    /// ```
    pub fn raw(&self) -> &'a RawStr {
        self.data.value.from_cow_source(self.source).into()
    }

    /// Returns the raw, undecoded query value as an `&str`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/foo/bar?baz+bar");
    /// assert_eq!(uri.query().unwrap(), "baz+bar");
    /// assert_eq!(uri.query().unwrap().as_str(), "baz+bar");
    /// ```
    pub fn as_str(&self) -> &'a str {
        self.raw().as_str()
    }

    /// Whether `self` is normalized, i.e, it has no empty segments.
    pub(crate) fn is_normalized(&self) -> bool {
        self.raw_segments().all(|s| !s.is_empty())
    }

    /// Normalizes `self`.
    pub(crate) fn to_normalized(self) -> Data<'static, fmt::Query> {
        let mut query = String::with_capacity(self.raw().trim().len());
        for (i, seg) in self.raw_segments().filter(|s| !s.is_empty()).enumerate() {
            if i != 0 { query.push('&'); }
            query.push_str(seg.as_str());
        }

        Data {
            value: IndexedStr::from(Cow::Owned(query)),
            decoded_segments: InitCell::new(),
        }
    }

    /// Returns an iterator over the undecoded, potentially empty `(name,
    /// value)` pairs of this query. If there is no query, the iterator is
    /// empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/").unwrap();
    /// assert!(uri.query().is_none());
    ///
    /// let uri = Origin::parse("/?").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert!(query_segs.is_empty());
    ///
    /// let uri = Origin::parse("/?foo").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert_eq!(query_segs, &["foo"]);
    ///
    /// let uri = Origin::parse("/?a=b&dog").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert_eq!(query_segs, &["a=b", "dog"]);
    ///
    /// let uri = Origin::parse("/?&").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert_eq!(query_segs, &["", ""]);
    ///
    /// // Recall that `uri!()` normalizes, so this is equivalent to `/?`.
    /// let uri = uri!("/?&");
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert!(query_segs.is_empty());
    ///
    /// // These are raw and undecoded. Use `segments()` for decoded variant.
    /// let uri = Origin::parse("/foo/bar?a+b%2F=some+one%40gmail.com&&%26%3D2").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert_eq!(query_segs, &["a+b%2F=some+one%40gmail.com", "", "%26%3D2"]);
    /// ```
    #[inline]
    pub fn raw_segments(&self) -> impl Iterator<Item = &'a RawStr> {
        let query = match self.raw().trim() {
            q if q.is_empty() => None,
            q => Some(q)
        };

        query.map(|p| p.split(fmt::Query::DELIMITER))
            .into_iter()
            .flatten()
    }

    /// Returns a (smart) iterator over the non-empty, url-decoded `(name,
    /// value)` pairs of this query. If there is no query, the iterator is
    /// empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/").unwrap();
    /// assert!(uri.query().is_none());
    ///
    /// let uri = Origin::parse("/foo/bar?a+b%2F=some+one%40gmail.com&&%26%3D2").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().segments().collect();
    /// assert_eq!(query_segs, &[("a b/", "some one@gmail.com"), ("&=2", "")]);
    /// ```
    pub fn segments(&self) -> Segments<'a, fmt::Query> {
        let cached = self.data.decoded_segments.get_or_init(|| {
            let (indexed, query) = (&self.data.value, self.raw());
            self.raw_segments()
                .filter(|s| !s.is_empty())
                .map(|s| s.split_at_byte(b'='))
                .map(|(k, v)| {
                    let key = decode_to_indexed_str::<fmt::Query>(k, (indexed, query));
                    let val = decode_to_indexed_str::<fmt::Query>(v, (indexed, query));
                    (key, val)
                })
                .collect()
        });

        Segments::new(self.raw(), cached)
    }
}

macro_rules! impl_partial_eq {
    ($A:ty = $B:ty) => (
        impl PartialEq<$A> for $B {
            #[inline(always)]
            fn eq(&self, other: &$A) -> bool {
                let left: &RawStr = self.as_ref();
                let right: &RawStr = other.as_ref();
                left == right
            }
        }
    )
}

macro_rules! impl_traits {
    ($T:ident) => (
        impl Hash for $T<'_> {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.raw().hash(state);
            }
        }

        impl Eq for $T<'_> { }

        impl IntoOwned for Data<'_, fmt::$T> {
            type Owned = Data<'static, fmt::$T>;

            fn into_owned(self) -> Self::Owned {
                Data {
                    value: self.value.into_owned(),
                    decoded_segments: self.decoded_segments.map(|v| v.into_owned()),
                }
            }
        }

        impl std::ops::Deref for $T<'_> {
            type Target = RawStr;

            fn deref(&self) -> &Self::Target {
                self.raw()
            }
        }

        impl AsRef<RawStr> for $T<'_> {
            fn as_ref(&self) -> &RawStr {
                self.raw()
            }
        }

        impl AsRef<std::ffi::OsStr> for $T<'_> {
            fn as_ref(&self) -> &std::ffi::OsStr {
                self.raw().as_ref()
            }
        }

        impl std::fmt::Display for $T<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.raw())
            }
        }

        impl_partial_eq!($T<'_> = $T<'_>);
        impl_partial_eq!(str = $T<'_>);
        impl_partial_eq!(&str = $T<'_>);
        impl_partial_eq!($T<'_> = str);
        impl_partial_eq!($T<'_> = &str);
        impl_partial_eq!(RawStr = $T<'_>);
        impl_partial_eq!(&RawStr = $T<'_>);
        impl_partial_eq!($T<'_> = RawStr);
        impl_partial_eq!($T<'_> = &RawStr);
    )
}

impl_traits!(Path);
impl_traits!(Query);
