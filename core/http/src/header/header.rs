use core::str;
use std::borrow::{Borrow, Cow};
use std::fmt;

use headers::{Header as HHeader, HeaderValue};
use indexmap::IndexMap;

use crate::uncased::{Uncased, UncasedStr};

/// Simple representation of an HTTP header.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Header<'h> {
    /// The name of the header.
    pub name: Uncased<'h>,
    /// The value of the header.
    pub value: Cow<'h, str>,
}

impl<'h> Header<'h> {
    /// Constructs a new header. This method should be used rarely and only for
    /// non-standard headers. Instead, prefer to use the `Into<Header>`
    /// implementations of many types, including
    /// [`ContentType`](crate::ContentType) and all of the headers in
    /// [`http::hyper::header`](hyper::header).
    ///
    /// # Examples
    ///
    /// Create a custom header with name `X-Custom-Header` and value `custom
    /// value`.
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Header;
    ///
    /// let header = Header::new("X-Custom-Header", "custom value");
    /// assert_eq!(header.to_string(), "X-Custom-Header: custom value");
    /// ```
    ///
    /// Use a `String` as a value to do the same.
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Header;
    ///
    /// let value = format!("{} value", "custom");
    /// let header = Header::new("X-Custom-Header", value);
    /// assert_eq!(header.to_string(), "X-Custom-Header: custom value");
    /// ```
    #[inline(always)]
    pub fn new<'a: 'h, 'b: 'h, N, V>(name: N, value: V) -> Header<'h>
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>
    {
        Header {
            name: Uncased::new(name),
            value: value.into()
        }
    }

    /// Returns `true` if `name` is a valid header name.
    ///
    /// This implements a simple (i.e, correct but not particularly performant)
    /// header "field-name" checker as defined in RFC 7230.
    ///
    /// ```text
    ///     header-field   = field-name ":" OWS field-value OWS
    ///     field-name     = token
    ///     token          = 1*tchar
    ///     tchar          = "!" / "#" / "$" / "%" / "&" / "'" / "*"
    ///                    / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
    ///                    / DIGIT / ALPHA
    ///                    ; any VCHAR, except delimiters
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Header;
    ///
    /// assert!(!Header::is_valid_name(""));
    /// assert!(!Header::is_valid_name("some header"));
    /// assert!(!Header::is_valid_name("some()"));
    /// assert!(!Header::is_valid_name("[SomeHeader]"));
    /// assert!(!Header::is_valid_name("<"));
    /// assert!(!Header::is_valid_name(""));
    /// assert!(!Header::is_valid_name("header,here"));
    ///
    /// assert!(Header::is_valid_name("Some#Header"));
    /// assert!(Header::is_valid_name("Some-Header"));
    /// assert!(Header::is_valid_name("This-Is_A~Header"));
    /// ```
    #[doc(hidden)]
    pub const fn is_valid_name(name: &str) -> bool {
        const fn is_tchar(b: &u8) -> bool {
            b.is_ascii_alphanumeric() || matches!(*b,
                b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' |
                b'.' | b'^' | b'_' | b'`' | b'|' | b'~')
        }

        let mut i = 0;
        let bytes = name.as_bytes();
        while i < bytes.len() {
            if !is_tchar(&bytes[i]) {
                return false
            }

            i += 1;
        }

        i > 0
    }

    /// Returns `true` if `val` is a valid header value.
    ///
    /// If `allow_empty` is `true`, this function returns `true` for empty
    /// values. Otherwise, this function returns `false` for empty values.
    ///
    /// This implements a simple (i.e, correct but not particularly performant)
    /// header "field-content" checker as defined in RFC 7230 without support
    /// for obsolete (`obs-`) syntax:
    ///
    ///   field-value    = *(field-content)
    ///   field-content  = field-vchar [ 1*( SP / HTAB ) field-vchar ]
    ///   field-vchar    = VCHAR
    ///   VCHAR          = %x21-7E ; visible (printing) characters
    ///
    /// Note that this is a generic checker. Specific headers may have stricter
    /// requirements. For example, the `Server` header does not allow delimiters
    /// in its values.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Header;
    ///
    /// assert!(!Header::is_valid_value("", false));
    /// assert!(!Header::is_valid_value(" " , false));
    /// assert!(!Header::is_valid_value(" hi", false));
    /// assert!(!Header::is_valid_value("a\nbc", false));
    /// assert!(!Header::is_valid_value("\nbc", false));
    /// assert!(!Header::is_valid_value("\n", false));
    /// assert!(!Header::is_valid_value("\t", false));
    /// assert!(!Header::is_valid_value("\r", false));
    /// assert!(!Header::is_valid_value("a\nb\nc", false));
    /// assert!(!Header::is_valid_value("a\rb\rc", false));
    ///
    /// assert!(Header::is_valid_value("", true));
    /// assert!(Header::is_valid_value("a", false));
    /// assert!(Header::is_valid_value("a", true));
    /// assert!(Header::is_valid_value("abc", false));
    /// assert!(Header::is_valid_value("abc", true));
    /// assert!(Header::is_valid_value("a b c", false));
    /// assert!(Header::is_valid_value("a b c", true));
    /// ```
    #[doc(hidden)]
    pub const fn is_valid_value(val: &str, allow_empty: bool) -> bool {
        const fn is_valid_start(b: &u8) -> bool {
            b.is_ascii_graphic()
        }

        const fn is_valid_continue(b: &u8) -> bool {
            is_valid_start(b) || *b == b' ' || *b == b'\t'
        }

        let mut i = 0;
        let bytes = val.as_bytes();
        while i < bytes.len() {
            match i {
                0 if !is_valid_start(&bytes[i]) => return false,
                _ if i > 0 && !is_valid_continue(&bytes[i]) => return false,
                _ => { /* ok */ }
            };

            i += 1;
        }

        allow_empty || i > 0
    }

    /// Returns the name of this header.
    ///
    /// # Example
    ///
    /// A case-sensitive equality check:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Header;
    ///
    /// let value = format!("{} value", "custom");
    /// let header = Header::new("X-Custom-Header", value);
    /// assert_eq!(header.name().as_str(), "X-Custom-Header");
    /// assert_ne!(header.name().as_str(), "X-CUSTOM-HEADER");
    /// ```
    ///
    /// A case-insensitive equality check:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Header;
    ///
    /// let header = Header::new("X-Custom-Header", "custom value");
    /// assert_eq!(header.name(), "X-Custom-Header");
    /// assert_eq!(header.name(), "X-CUSTOM-HEADER");
    /// ```
    #[inline(always)]
    pub fn name(&self) -> &UncasedStr {
        &self.name
    }

    /// Returns the value of this header.
    ///
    /// # Example
    ///
    /// A case-sensitive equality check:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Header;
    ///
    /// let header = Header::new("X-Custom-Header", "custom value");
    /// assert_eq!(header.value(), "custom value");
    /// ```
    #[inline(always)]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Header<'_> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

/// A collection of headers, mapping a header name to its many ordered values.
///
/// # Case-Insensitivity
///
/// All header names, including those passed in to `HeaderMap` methods and those
/// stored in an existing `HeaderMap`, are treated case-insensitively. This
/// means that, for instance, a look for a header by the name of "aBC" will
/// returns values for headers of names "AbC", "ABC", "abc", and so on.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HeaderMap<'h> {
    headers: IndexMap<Uncased<'h>, Vec<Cow<'h, str>>>
}

impl<'h> HeaderMap<'h> {
    /// Returns an empty header collection.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let map = HeaderMap::new();
    /// ```
    #[inline(always)]
    pub fn new() -> HeaderMap<'h> {
        HeaderMap { headers: IndexMap::new() }
    }

    /// Returns true if `self` contains a header with the name `name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{HeaderMap, ContentType};
    ///
    /// let mut map = HeaderMap::new();
    /// map.add(ContentType::HTML);
    ///
    /// assert!(map.contains("Content-Type"));
    /// assert!(!map.contains("Accepts"));
    /// ```
    #[inline]
    pub fn contains<N: AsRef<str>>(&self, name: N) -> bool {
        self.headers.get(UncasedStr::new(name.as_ref())).is_some()
    }

    /// Returns the number of _values_ stored in the map.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    /// assert_eq!(map.len(), 0);
    ///
    /// map.add_raw("X-Custom", "value_1");
    /// assert_eq!(map.len(), 1);
    ///
    /// map.replace_raw("X-Custom", "value_2");
    /// assert_eq!(map.len(), 1);
    ///
    /// map.add_raw("X-Custom", "value_1");
    /// assert_eq!(map.len(), 2);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.headers.iter().flat_map(|(_, values)| values.iter()).count()
    }

    /// Returns `true` if there are no headers stored in the map. Otherwise
    /// returns `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let map = HeaderMap::new();
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    /// Returns an iterator over all of the values stored in `self` for the
    /// header with name `name`. The headers are returned in FIFO order.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    /// map.add_raw("X-Custom", "value_1");
    /// map.add_raw("X-Custom", "value_2");
    ///
    /// assert_eq!(map.len(), 2);
    ///
    /// let mut values = map.get("X-Custom");
    /// assert_eq!(values.next(), Some("value_1"));
    /// assert_eq!(values.next(), Some("value_2"));
    /// assert_eq!(values.next(), None);
    /// ```
    #[inline]
    pub fn get(&self, name: &str) -> impl Iterator<Item=&str> {
        self.headers
            .get(UncasedStr::new(name))
            .into_iter()
            .flat_map(|values| values.iter().map(|val| val.borrow()))
    }

    /// Returns the _first_ value stored for the header with name `name` if
    /// there is one.
    ///
    /// # Examples
    ///
    /// Retrieve the first value when one exists:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    /// map.add_raw("X-Custom", "value_1");
    /// map.add_raw("X-Custom", "value_2");
    ///
    /// assert_eq!(map.len(), 2);
    ///
    /// let first_value = map.get_one("X-Custom");
    /// assert_eq!(first_value, Some("value_1"));
    /// ```
    ///
    /// Attempt to retrieve a value that doesn't exist:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    /// map.add_raw("X-Custom", "value_1");
    ///
    /// let first_value = map.get_one("X-Other");
    /// assert_eq!(first_value, None);
    /// ```
    #[inline]
    pub fn get_one<'a>(&'a self, name: &str) -> Option<&'a str> {
        self.headers.get(UncasedStr::new(name))
            .and_then(|values| {
                if !values.is_empty() { Some(values[0].borrow()) }
                else { None }
            })
    }

    /// Replace any header that matches the name of `header.name` with `header`.
    /// If there is no such header in `self`, add `header`. If the matching
    /// header had multiple values, all of the values are removed, and only the
    /// value in `header` will remain.
    ///
    /// # Example
    ///
    /// Replace a header that doesn't yet exist:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{HeaderMap, ContentType};
    ///
    /// let mut map = HeaderMap::new();
    /// map.replace(ContentType::JSON);
    ///
    /// assert!(map.get_one("Content-Type").is_some());
    /// ```
    ///
    /// Replace a header that already exists:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{HeaderMap, ContentType};
    ///
    /// let mut map = HeaderMap::new();
    ///
    /// map.replace(ContentType::JSON);
    /// assert_eq!(map.get_one("Content-Type"), Some("application/json"));
    ///
    /// map.replace(ContentType::GIF);
    /// assert_eq!(map.get_one("Content-Type"), Some("image/gif"));
    /// assert_eq!(map.len(), 1);
    /// ```
    ///
    /// An example of case-insensitivity.
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{HeaderMap, Header, ContentType};
    ///
    /// let mut map = HeaderMap::new();
    ///
    /// map.replace(ContentType::JSON);
    /// assert_eq!(map.get_one("Content-Type"), Some("application/json"));
    ///
    /// map.replace(Header::new("CONTENT-type", "image/gif"));
    /// assert_eq!(map.get_one("Content-Type"), Some("image/gif"));
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline(always)]
    pub fn replace<'p: 'h, H: Into<Header<'p>>>(&mut self, header: H) -> bool {
        let header = header.into();
        self.headers.insert(header.name, vec![header.value]).is_some()
    }

    /// A convenience method to replace a header using a raw name and value.
    /// Aliases `replace(Header::new(name, value))`. Should be used rarely.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    ///
    /// map.replace_raw("X-Custom", "value_1");
    /// assert_eq!(map.get_one("X-Custom"), Some("value_1"));
    ///
    /// map.replace_raw("X-Custom", "value_2");
    /// assert_eq!(map.get_one("X-Custom"), Some("value_2"));
    /// assert_eq!(map.len(), 1);
    /// ```
    #[inline(always)]
    pub fn replace_raw<'a: 'h, 'b: 'h, N, V>(&mut self, name: N, value: V) -> bool
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>
    {
        self.replace(Header::new(name, value))
    }

    /// Replaces all of the values for a header with name `name` with `values`.
    /// This a low-level method and should rarely be used.
    ///
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    /// map.add_raw("X-Custom", "value_1");
    /// map.add_raw("X-Custom", "value_2");
    ///
    /// let vals: Vec<_> = map.get("X-Custom").map(|s| s.to_string()).collect();
    /// assert_eq!(vals, vec!["value_1", "value_2"]);
    ///
    /// map.replace_all("X-Custom", vec!["value_3".into(), "value_4".into()]);
    /// let vals: Vec<_> = map.get("X-Custom").collect();
    /// assert_eq!(vals, vec!["value_3", "value_4"]);
    /// ```
    #[inline(always)]
    pub fn replace_all<'n, 'v: 'h, H>(&mut self, name: H, values: Vec<Cow<'v, str>>)
        where 'n: 'h, H: Into<Cow<'n, str>>
    {
        self.headers.insert(Uncased::new(name), values);
    }

    /// Adds `header` into the map. If a header with `header.name` was
    /// previously added, that header will have one more value.
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{Cookie, HeaderMap};
    ///
    /// let mut map = HeaderMap::new();
    ///
    /// map.add(&Cookie::new("a", "b"));
    /// assert_eq!(map.get("Set-Cookie").count(), 1);
    ///
    /// map.add(&Cookie::new("c", "d"));
    /// assert_eq!(map.get("Set-Cookie").count(), 2);
    /// ```
    #[inline(always)]
    pub fn add<'p: 'h, H: Into<Header<'p>>>(&mut self, header: H) {
        let header = header.into();
        self.headers.entry(header.name).or_default().push(header.value);
    }

    /// A convenience method to add a header using a raw name and value.
    /// Aliases `add(Header::new(name, value))`. Should be used rarely.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    ///
    /// map.add_raw("X-Custom", "value_1");
    /// assert_eq!(map.get("X-Custom").count(), 1);
    ///
    /// map.add_raw("X-Custom", "value_2");
    /// let values: Vec<_> = map.get("X-Custom").collect();
    /// assert_eq!(values, vec!["value_1", "value_2"]);
    /// ```
    #[inline(always)]
    pub fn add_raw<'a: 'h, 'b: 'h, N, V>(&mut self, name: N, value: V)
        where N: Into<Cow<'a, str>>, V: Into<Cow<'b, str>>
    {
        self.add(Header::new(name, value))
    }

    /// Adds all of the values to a header with name `name`. This a low-level
    /// method and should rarely be used. `values` will be empty when this
    /// method returns.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    ///
    /// let mut values = vec!["value_1".into(), "value_2".into()];
    /// map.add_all("X-Custom", &mut values);
    /// assert_eq!(map.get("X-Custom").count(), 2);
    /// assert_eq!(values.len(), 0);
    ///
    /// let mut values = vec!["value_3".into(), "value_4".into()];
    /// map.add_all("X-Custom", &mut values);
    /// assert_eq!(map.get("X-Custom").count(), 4);
    /// assert_eq!(values.len(), 0);
    ///
    /// let values: Vec<_> = map.get("X-Custom").collect();
    /// assert_eq!(values, vec!["value_1", "value_2", "value_3", "value_4"]);
    /// ```
    #[inline(always)]
    pub fn add_all<'n, H>(&mut self, name: H, values: &mut Vec<Cow<'h, str>>)
        where 'n:'h, H: Into<Cow<'n, str>>
    {
        self.headers.entry(Uncased::new(name))
            .or_default()
            .append(values)
    }

    /// Remove all of the values for header with name `name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    /// map.add_raw("X-Custom", "value_1");
    /// map.add_raw("X-Custom", "value_2");
    /// map.add_raw("X-Other", "other");
    ///
    /// assert_eq!(map.len(), 3);
    ///
    /// map.remove("X-Custom");
    /// assert_eq!(map.len(), 1);
    #[inline(always)]
    pub fn remove(&mut self, name: &str) {
        self.headers.swap_remove(UncasedStr::new(name));
    }

    /// Removes all of the headers stored in this map and returns a vector
    /// containing them. Header names are returned in no specific order, but all
    /// values for a given header name are grouped together, and values are in
    /// FIFO order.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{HeaderMap, Header};
    /// use std::collections::HashSet;
    ///
    /// // The headers we'll be storing.
    /// let all_headers = vec![
    ///     Header::new("X-Custom", "value_1"),
    ///     Header::new("X-Custom", "value_2"),
    ///     Header::new("X-Other", "other")
    /// ];
    ///
    /// // Create a map, store all of the headers.
    /// let mut map = HeaderMap::new();
    /// for header in all_headers.clone() {
    ///     map.add(header)
    /// }
    ///
    /// assert_eq!(map.len(), 3);
    ///
    /// // Now remove them all, ensure the map is empty.
    /// let removed_headers = map.remove_all();
    /// assert!(map.is_empty());
    ///
    /// // Create two sets: what we expect and got. Ensure they're equal.
    /// let expected_set: HashSet<_> = all_headers.into_iter().collect();
    /// let actual_set: HashSet<_> = removed_headers.into_iter().collect();
    /// assert_eq!(expected_set, actual_set);
    /// ```
    #[inline(always)]
    pub fn remove_all(&mut self) -> Vec<Header<'h>> {
        let old_map = std::mem::replace(self, HeaderMap::new());
        old_map.into_iter().collect()
    }

    /// Returns an iterator over all of the `Header`s stored in the map. Header
    /// names are returned in no specific order, but all values for a given
    /// header name are grouped together, and values are in FIFO order.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{HeaderMap, Header};
    ///
    /// // The headers we'll be storing.
    /// let all_headers = vec![
    ///     Header::new("X-Custom", "value_0"),
    ///     Header::new("X-Custom", "value_1"),
    ///     Header::new("X-Other", "other"),
    ///     Header::new("X-Third", "third"),
    /// ];
    ///
    /// // Create a map, store all of the headers.
    /// let mut map = HeaderMap::new();
    /// for header in all_headers {
    ///     map.add(header)
    /// }
    ///
    /// // Ensure there are three headers via the iterator.
    /// assert_eq!(map.iter().count(), 4);
    ///
    /// // Actually iterate through them.
    /// let mut custom = 0;
    /// for header in map.iter() {
    ///     match header.name().as_str() {
    ///         "X-Other" => assert_eq!(header.value(), "other"),
    ///         "X-Third" => assert_eq!(header.value(), "third"),
    ///         "X-Custom" => {
    ///             assert_eq!(header.value(), format!("value_{custom}"));
    ///             custom += 1;
    ///         },
    ///         _ => unreachable!("there are only three uniquely named headers")
    ///     }
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item=Header<'_>> {
        self.headers.iter().flat_map(|(key, values)| {
            values.iter().map(move |val| {
                Header::new(key.as_str(), &**val)
            })
        })
    }

    /// Consumes `self` and returns an iterator over all of the headers stored
    /// in the map in the way they are stored. This is a low-level mechanism and
    /// should likely not be used.
    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    #[inline]
    pub fn into_iter_raw(self) -> impl Iterator<Item=(Uncased<'h>, Vec<Cow<'h, str>>)> {
        self.headers.into_iter()
    }
}

/// Consumes `self` and returns an iterator over all of the `Header`s stored
/// in the map. Header names are returned in no specific order, but all
/// values for a given header name are grouped together, and values are in
/// FIFO order.
///
/// # Example
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::{HeaderMap, Header};
///
/// // The headers we'll be storing.
/// let all_headers = vec![
///     Header::new("X-Custom", "value_0"),
///     Header::new("X-Custom", "value_1"),
///     Header::new("X-Other", "other"),
///     Header::new("X-Third", "third"),
/// ];
///
/// // Create a map, store all of the headers.
/// let mut map = HeaderMap::new();
/// for header in all_headers {
///     map.add(header)
/// }
///
/// // Ensure there are three headers via the iterator.
/// assert_eq!(map.iter().count(), 4);
///
/// // Actually iterate through them.
/// let mut custom = 0;
/// for header in map.into_iter() {
///     match header.name().as_str() {
///         "X-Other" => assert_eq!(header.value(), "other"),
///         "X-Third" => assert_eq!(header.value(), "third"),
///         "X-Custom" => {
///             assert_eq!(header.value(), format!("value_{custom}"));
///             custom += 1;
///         },
///         _ => unreachable!("there are only three uniquely named headers")
///     }
/// }
/// ```
impl<'h> IntoIterator for HeaderMap<'h> {
    type Item = Header<'h>;

    type IntoIter = IntoIter<'h>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            headers: self.headers.into_iter(),
            current: None,
        }
    }
}

/// Owned iterator over [`Header`]s in a [`HeaderMap`].
///
/// See [`HeaderMap::into_iter()`] for details.
pub struct IntoIter<'h> {
    headers: indexmap::map::IntoIter<Uncased<'h>, Vec<Cow<'h, str>>>,
    current: Option<(Uncased<'h>, std::vec::IntoIter<Cow<'h, str>>)>,
}

impl<'h> Iterator for IntoIter<'h> {
    type Item = Header<'h>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((name, values)) = &mut self.current {
                if let Some(value) = values.next() {
                    return Some(Header { name: name.clone(), value });
                }
            }

            let (name, values) = self.headers.next()?;
            self.current = Some((name, values.into_iter()));
        }
    }
}

impl From<cookie::Cookie<'_>> for Header<'static> {
    fn from(cookie: cookie::Cookie<'_>) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}

impl From<&cookie::Cookie<'_>> for Header<'static> {
    fn from(cookie: &cookie::Cookie<'_>) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}

/// A destination for `HeaderValue`s that can be used to accumulate
/// a single header value using from hyperium headers' decode protocol.
#[derive(Default)]
struct HeaderValueDestination {
    value: Option<HeaderValue>,
    count: usize,
}

impl <'r>HeaderValueDestination {
    fn into_value(self) -> HeaderValue {
        if let Some(value) = self.value {
            // TODO: if value.count > 1, then log that multiple header values are
            // generated by the typed header, but that the dropped.
            value
        } else {
            // Perhaps log that the typed header didn't create any values.
            // This won't happen in the current implementation (headers 0.4.0).
            HeaderValue::from_static("")
        }
    }

    fn into_header_string(self) -> Cow<'static, str> {
        let value = self.into_value();
        // TODO: Optimize if we know this is a static reference.
        value.to_str().unwrap_or("").to_string().into()
    }
}

impl Extend<HeaderValue> for HeaderValueDestination {
    fn extend<T: IntoIterator<Item = HeaderValue>>(&mut self, iter: T) {
        for value in iter {
            self.count += 1;
            if self.value.is_none() {
                self.value = Some(value)
            }
        }
    }
}

macro_rules! import_typed_headers {
($($name:ident),*) => ($(
    pub use headers::$name;

    impl ::std::convert::From<self::$name> for Header<'static> {
        fn from(header: self::$name) -> Self {
            let mut destination = HeaderValueDestination::default();
            header.encode(&mut destination);
            let name = self::$name::name();
            Header::new(name.as_str(), destination.into_header_string())
        }
    }
)*)
}

macro_rules! import_generic_typed_headers {
($($name:ident<$bound:ident>),*) => ($(
    pub use headers::$name;

    impl <T1: 'static + $bound>::std::convert::From<self::$name<T1>>
        for Header<'static> {
        fn from(header: self::$name<T1>) -> Self {
            let mut destination = HeaderValueDestination::default();
            header.encode(&mut destination);
            let name = self::$name::<T1>::name();
            Header::new(name.as_str(), destination.into_header_string())
        }
    }
)*)
}

// The following headers from 'headers' 0.4 are not imported, since they are
// provided by other Rocket features.

// * ContentType, // Content-Type header, defined in RFC7231
// * Cookie, // Cookie header, defined in RFC6265
// * Host, // The Host header.
// * Location, // Location header, defined in RFC7231
// * SetCookie, // Set-Cookie header, defined RFC6265

import_typed_headers! {
  AcceptRanges, // Accept-Ranges header, defined in RFC7233
  AccessControlAllowCredentials, // Access-Control-Allow-Credentials header, part of CORS
  AccessControlAllowHeaders, // Access-Control-Allow-Headers header, part of CORS
  AccessControlAllowMethods, // Access-Control-Allow-Methods header, part of CORS
  AccessControlAllowOrigin, // The Access-Control-Allow-Origin response header, part of CORS
  AccessControlExposeHeaders, // Access-Control-Expose-Headers header, part of CORS
  AccessControlMaxAge, // Access-Control-Max-Age header, part of CORS
  AccessControlRequestHeaders, // Access-Control-Request-Headers header, part of CORS
  AccessControlRequestMethod, // Access-Control-Request-Method header, part of CORS
  Age, // Age header, defined in RFC7234
  Allow, // Allow header, defined in RFC7231
  CacheControl, // Cache-Control header, defined in RFC7234 with extensions in RFC8246
  Connection, // Connection header, defined in RFC7230
  ContentDisposition, // A Content-Disposition header, (re)defined in RFC6266.
  ContentEncoding, // Content-Encoding header, defined in RFC7231
  ContentLength, // Content-Length header, defined in RFC7230
  ContentLocation, // Content-Location header, defined in RFC7231
  ContentRange, // Content-Range, described in RFC7233
  Date, // Date header, defined in RFC7231
  ETag, // ETag header, defined in RFC7232
  Expect, // The Expect header.
  Expires, // Expires header, defined in RFC7234
  IfMatch, // If-Match header, defined in RFC7232
  IfModifiedSince, // If-Modified-Since header, defined in RFC7232
  IfNoneMatch, // If-None-Match header, defined in RFC7232
  IfRange, // If-Range header, defined in RFC7233
  IfUnmodifiedSince, // If-Unmodified-Since header, defined in RFC7232
  LastModified, // Last-Modified header, defined in RFC7232
  Origin, // The Origin header.
  Pragma, // The Pragma header defined by HTTP/1.0.
  Range, // Range header, defined in RFC7233
  Referer, // Referer header, defined in RFC7231
  ReferrerPolicy, // Referrer-Policy header, part of Referrer Policy
  RetryAfter, // The Retry-After header.
  SecWebsocketAccept, // The Sec-Websocket-Accept header.
  SecWebsocketKey, // The Sec-Websocket-Key header.
  SecWebsocketVersion, // The Sec-Websocket-Version header.
  Server, // Server header, defined in RFC7231
  StrictTransportSecurity, // StrictTransportSecurity header, defined in RFC6797
  Te, // TE header, defined in RFC7230
  TransferEncoding, // Transfer-Encoding header, defined in RFC7230
  Upgrade, // Upgrade header, defined in RFC7230
  UserAgent, // User-Agent header, defined in RFC7231
  Vary // Vary header, defined in RFC7231
}

import_generic_typed_headers! {
    Authorization<Credentials>, // Authorization header, defined in RFC7235
    ProxyAuthorization<Credentials> // Proxy-Authorization header, defined in RFC7235
}

pub use headers::authorization::Credentials;

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::HeaderMap;

        #[test]
    fn add_typed_header() {
        use super::LastModified;
        let mut map = HeaderMap::new();
        map.add(LastModified::from(SystemTime::now()));
        assert!(map.get_one("last-modified").unwrap().contains("GMT"));
    }

    #[test]
    fn case_insensitive_add_get() {
        let mut map = HeaderMap::new();
        map.add_raw("content-type", "application/json");

        let ct = map.get_one("Content-Type");
        assert_eq!(ct, Some("application/json"));

        let ct2 = map.get_one("CONTENT-TYPE");
        assert_eq!(ct2, Some("application/json"))
    }

    #[test]
    fn case_insensitive_multiadd() {
        let mut map = HeaderMap::new();
        map.add_raw("x-custom", "a");
        map.add_raw("X-Custom", "b");
        map.add_raw("x-CUSTOM", "c");

        let vals: Vec<_> = map.get("x-CuStOm").collect();
        assert_eq!(vals, vec!["a", "b", "c"]);
    }
}
