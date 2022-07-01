use std::hash::Hash;
use std::borrow::Cow;
use std::fmt::Write;

use state::Storage;

use crate::{RawStr, ext::IntoOwned};
use crate::uri::Segments;
use crate::uri::fmt::{self, Part};
use crate::parse::{IndexedStr, Extent};

// INTERNAL DATA STRUCTURE.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Data<'a, P: Part> {
    pub(crate) value: IndexedStr<'a>,
    pub(crate) decoded_segments: Storage<Vec<P::Raw>>,
}

impl<'a, P: Part> Data<'a, P> {
    pub(crate) fn raw(value: Extent<&'a [u8]>) -> Self {
        Data { value: value.into(), decoded_segments: Storage::new() }
    }

    // INTERNAL METHOD.
    #[doc(hidden)]
    pub fn new<S: Into<Cow<'a, str>>>(value: S) -> Self {
        Data {
            value: IndexedStr::from(value.into()),
            decoded_segments: Storage::new(),
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
            let indexed = IndexedStr::checked_from(b, source.as_str());
            debug_assert!(indexed.is_some());
            indexed.unwrap_or_else(|| IndexedStr::from(Cow::Borrowed("")))
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

    /// Whether `self` is normalized, i.e, it has no empty segments.
    ///
    /// If `absolute`, then a starting  `/` is required.
    pub(crate) fn is_normalized(&self, absolute: bool) -> bool {
        (!absolute || self.raw().starts_with('/'))
            && self.raw_segments().all(|s| !s.is_empty())
    }

    /// Normalizes `self`. If `absolute`, a starting  `/` is required.
    pub(crate) fn to_normalized(self, absolute: bool) -> Data<'static, fmt::Path> {
        let mut path = String::with_capacity(self.raw().len());
        let absolute = absolute || self.raw().starts_with('/');
        for (i, seg) in self.raw_segments().filter(|s| !s.is_empty()).enumerate() {
            if absolute || i != 0 { path.push('/'); }
            let _ = write!(path, "{}", seg);
        }

        if path.is_empty() && absolute {
            path.push('/');
        }

        Data {
            value: IndexedStr::from(Cow::Owned(path)),
            decoded_segments: Storage::new(),
        }
    }

    /// Returns an iterator over the raw, undecoded segments. Segments may be
    /// empty.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/").unwrap();
    /// assert_eq!(uri.path().raw_segments().count(), 0);
    ///
    /// let uri = Origin::parse("//").unwrap();
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &["", ""]);
    ///
    /// // Recall that `uri!()` normalizes static inputs.
    /// let uri = uri!("//");
    /// assert_eq!(uri.path().raw_segments().count(), 0);
    ///
    /// let uri = Origin::parse("/a").unwrap();
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &["a"]);
    ///
    /// let uri = Origin::parse("/a//b///c/d?query&param").unwrap();
    /// let segments: Vec<_> = uri.path().raw_segments().collect();
    /// assert_eq!(segments, &["a", "", "b", "", "", "c", "d"]);
    /// ```
    #[inline(always)]
    pub fn raw_segments(&self) -> impl Iterator<Item = &'a RawStr> {
        let path = match self.raw() {
            p if p.is_empty() || p == "/" => None,
            p if p.starts_with(fmt::Path::DELIMITER) => Some(&p[1..]),
            p => Some(p)
        };

        path.map(|p| p.split(fmt::Path::DELIMITER))
            .into_iter()
            .flatten()
    }

    /// Returns a (smart) iterator over the non-empty, percent-decoded segments.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a%20b/b%2Fc/d//e?query=some").unwrap();
    /// let path_segs: Vec<&str> = uri.path().segments().collect();
    /// assert_eq!(path_segs, &["a b", "b/c", "d", "e"]);
    /// ```
    pub fn segments(&self) -> Segments<'a, fmt::Path> {
        let cached = self.data.decoded_segments.get_or_set(|| {
            let (indexed, path) = (&self.data.value, self.raw());
            self.raw_segments()
                .filter(|r| !r.is_empty())
                .map(|s| decode_to_indexed_str::<fmt::Path>(s, (indexed, path)))
                .collect()
        });

        Segments::new(self.raw(), cached)
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
        !self.is_empty() && self.raw_segments().all(|s| !s.is_empty())
    }

    /// Normalizes `self`.
    pub(crate) fn to_normalized(self) -> Option<Data<'static, fmt::Query>> {
        let mut query = String::with_capacity(self.raw().len());
        for (i, seg) in self.raw_segments().filter(|s| !s.is_empty()).enumerate() {
            if i != 0 { query.push('&'); }
            let _ = write!(query, "{}", seg);
        }

        if query.is_empty() {
            return None;
        }

        Some(Data {
            value: IndexedStr::from(Cow::Owned(query)),
            decoded_segments: Storage::new(),
        })
    }

    /// Returns an iterator over the non-empty, undecoded `(name, value)` pairs
    /// of this query. If there is no query, the iterator is empty. Segments may
    /// be empty.
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
    /// let uri = Origin::parse("/?a=b&dog").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert_eq!(query_segs, &["a=b", "dog"]);
    ///
    /// // This is not normalized, so the query is `""`, the empty string.
    /// let uri = Origin::parse("/?&").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert_eq!(query_segs, &["", ""]);
    ///
    /// // Recall that `uri!()` normalizes.
    /// let uri = uri!("/?&");
    /// assert!(uri.query().is_none());
    ///
    /// // These are raw and undecoded. Use `segments()` for decoded variant.
    /// let uri = Origin::parse("/foo/bar?a+b%2F=some+one%40gmail.com&&%26%3D2").unwrap();
    /// let query_segs: Vec<_> = uri.query().unwrap().raw_segments().collect();
    /// assert_eq!(query_segs, &["a+b%2F=some+one%40gmail.com", "", "%26%3D2"]);
    /// ```
    #[inline]
    pub fn raw_segments(&self) -> impl Iterator<Item = &'a RawStr> {
        let query = match self.raw() {
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
        let cached = self.data.decoded_segments.get_or_set(|| {
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
