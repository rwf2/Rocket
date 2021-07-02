use std::borrow::{Borrow, Cow};
use std::convert::AsRef;
use std::cmp::Ordering;
use std::str::Utf8Error;
use std::fmt;

use ref_cast::RefCast;
use stable_pattern::{Pattern, Searcher, ReverseSearcher, Split, SplitInternal};
use crate::uri::fmt::{DEFAULT_ENCODE_SET, percent_encode, percent_encode_bytes};

use crate::uncased::UncasedStr;

/// A reference to a string inside of a raw HTTP message.
///
/// A `RawStr` is an unsanitzed, unvalidated, and undecoded raw string from an
/// HTTP message. It exists to separate validated string inputs, represented by
/// the `String`, `&str`, and `Cow<str>` types, from unvalidated inputs,
/// represented by `&RawStr`.
///
/// # Validation
///
/// An `&RawStr` should be converted into one of the validated string input
/// types through methods on `RawStr`. These methods are summarized below:
///
///   * **[`url_decode()`]** - used to decode a raw string in a form value
///     context
///   * **[`percent_decode()`], [`percent_decode_lossy()`]** - used to
///     percent-decode a raw string, typically in a URL context
///   * **[`html_escape()`]** - used to decode a string for use in HTML
///     templates
///   * **[`as_str()`]** - used when the `RawStr` is known to be safe in the
///     context of its intended use. Use sparingly and with care!
///   * **[`as_uncased_str()`]** - used when the `RawStr` is known to be safe in
///     the context of its intended, uncased use
///
/// **Note:** Template engines like Tera and Handlebars all functions like
/// [`html_escape()`] on all rendered template outputs by default.
///
/// [`as_str()`]: RawStr::as_str()
/// [`as_uncased_str()`]: RawStr::as_uncased_str()
/// [`url_decode()`]: RawStr::url_decode()
/// [`html_escape()`]: RawStr::html_escape()
/// [`percent_decode()`]: RawStr::percent_decode()
/// [`percent_decode_lossy()`]: RawStr::percent_decode_lossy()
///
/// # Usage
///
/// A `RawStr` is a dynamically sized type (just like `str`). It is always used
/// through a reference an as `&RawStr` (just like &str).
#[repr(transparent)]
#[derive(RefCast, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawStr(str);

impl ToOwned for RawStr {
    type Owned = RawStrBuf;

    fn to_owned(&self) -> Self::Owned {
        RawStrBuf(self.to_string())
    }
}

/// An owned version of [`RawStr`].
#[repr(transparent)]
#[derive(RefCast, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawStrBuf(String);

impl RawStrBuf {
    /// Cost-free conversion from `self` into a `String`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStrBuf;
    ///
    /// let raw = RawStrBuf::from(format!("hello {}", "world"));
    /// let string = raw.into_string();
    /// ```
    pub fn into_string(self) -> String {
        self.0
    }
}

impl RawStr {
    /// Constructs an `&RawStr` from a string-like type at no cost.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Hello, world!");
    ///
    /// // `into` can also be used; note that the type must be specified
    /// let raw_str: &RawStr = "Hello, world!".into();
    /// ```
    pub fn new<S: AsRef<str> + ?Sized>(string: &S) -> &RawStr {
        RawStr::ref_cast(string.as_ref())
    }

    /// Construct a `Cow<RawStr>` from a `Cow<Str>`. Does not allocate.
    ///
    /// See [`RawStr::into_cow_str()`] for the inverse operation.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::borrow::Cow;
    /// use rocket::http::RawStr;
    ///
    /// let cow_str = Cow::from("hello!");
    /// let cow_raw = RawStr::from_cow_str(cow_str);
    /// assert_eq!(cow_raw.as_str(), "hello!");
    /// ```
    pub fn from_cow_str(cow: Cow<'_, str>) -> Cow<'_, RawStr> {
        match cow {
            Cow::Borrowed(b) => Cow::Borrowed(b.into()),
            Cow::Owned(b) => Cow::Owned(b.into()),
        }
    }

    /// Construct a `Cow<str>` from a `Cow<RawStr>`. Does not allocate.
    ///
    /// See [`RawStr::from_cow_str()`] for the inverse operation.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::borrow::Cow;
    /// use rocket::http::RawStr;
    ///
    /// let cow_raw = Cow::from(RawStr::new("hello!"));
    /// let cow_str = RawStr::into_cow_str(cow_raw);
    /// assert_eq!(&*cow_str, "hello!");
    /// ```
    pub fn into_cow_str(cow: Cow<'_, RawStr>) -> Cow<'_, str> {
        match cow {
            Cow::Borrowed(b) => Cow::Borrowed(b.as_str()),
            Cow::Owned(b) => Cow::Owned(b.into_string()),
        }
    }

    /// Percent-decodes `self`.
    fn _percent_decode(&self) -> percent_encoding::PercentDecode<'_> {
        percent_encoding::percent_decode(self.as_bytes())
    }

    /// Returns a percent-decoded version of the string.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the percent encoded values are not valid UTF-8.
    ///
    /// # Example
    ///
    /// With a valid string:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Hello%21");
    /// let decoded = raw_str.percent_decode();
    /// assert_eq!(decoded, Ok("Hello!".into()));
    /// ```
    ///
    /// With an invalid string:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// // Note: Rocket should never hand you a bad `&RawStr`.
    /// let bad_str = unsafe { std::str::from_utf8_unchecked(b"a=\xff") };
    /// let bad_raw_str = RawStr::new(bad_str);
    /// assert!(bad_raw_str.percent_decode().is_err());
    /// ```
    #[inline(always)]
    pub fn percent_decode(&self) -> Result<Cow<'_, str>, Utf8Error> {
        self._percent_decode().decode_utf8()
    }

    /// Returns a percent-decoded version of the string. Any invalid UTF-8
    /// percent-encoded byte sequences will be replaced � U+FFFD, the
    /// replacement character.
    ///
    /// # Example
    ///
    /// With a valid string:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Hello%21");
    /// let decoded = raw_str.percent_decode_lossy();
    /// assert_eq!(decoded, "Hello!");
    /// ```
    ///
    /// With an invalid string:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// // Note: Rocket should never hand you a bad `&RawStr`.
    /// let bad_str = unsafe { std::str::from_utf8_unchecked(b"a=\xff") };
    /// let bad_raw_str = RawStr::new(bad_str);
    /// assert_eq!(bad_raw_str.percent_decode_lossy(), "a=�");
    /// ```
    #[inline(always)]
    pub fn percent_decode_lossy(&self) -> Cow<'_, str> {
        self._percent_decode().decode_utf8_lossy()
    }

    /// Replaces '+' with ' ' in `self`, allocating only when necessary.
    fn _replace_plus(&self) -> Cow<'_, str> {
        let string = self.as_str();
        let mut allocated = String::new(); // this is allocation free
        for i in memchr::memchr_iter(b'+', string.as_bytes()) {
            if allocated.is_empty() {
                allocated = string.into();
            }

            unsafe { allocated.as_bytes_mut()[i] = b' '; }
        }

        match allocated.is_empty() {
            true => Cow::Borrowed(string),
            false => Cow::Owned(allocated)
        }
    }

    /// Returns a percent-encoded version of the string.
    ///
    /// # Example
    ///
    /// With a valid string:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Hello%21");
    /// let decoded = raw_str.percent_decode();
    /// assert_eq!(decoded, Ok("Hello!".into()));
    /// ```
    ///
    /// With an invalid string:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// // NOTE: Rocket will never hand you a bad `&RawStr`.
    /// let bad_str = unsafe { std::str::from_utf8_unchecked(b"a=\xff") };
    /// let bad_raw_str = RawStr::new(bad_str);
    /// assert!(bad_raw_str.percent_decode().is_err());
    /// ```
    #[inline(always)]
    pub fn percent_encode(&self) -> Cow<'_, RawStr> {
        Self::from_cow_str(percent_encode::<DEFAULT_ENCODE_SET>(self))
    }

    /// Returns a percent-encoded version of `bytes`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// // Note: Rocket should never hand you a bad `&RawStr`.
    /// let bytes = &[93, 12, 0, 13, 1];
    /// let encoded = RawStr::percent_encode_bytes(&bytes[..]);
    /// ```
    #[inline(always)]
    pub fn percent_encode_bytes(bytes: &[u8]) -> Cow<'_, RawStr> {
        Self::from_cow_str(percent_encode_bytes::<DEFAULT_ENCODE_SET>(bytes))
    }

    /// Returns a URL-decoded version of the string. This is identical to
    /// percent decoding except that `+` characters are converted into spaces.
    /// This is the encoding used by form values.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the percent encoded values are not valid UTF-8.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Hello%2C+world%21");
    /// let decoded = raw_str.url_decode();
    /// assert_eq!(decoded.unwrap(), "Hello, world!");
    /// ```
    pub fn url_decode(&self) -> Result<Cow<'_, str>, Utf8Error> {
        let string = self._replace_plus();
        match percent_encoding::percent_decode(string.as_bytes()).decode_utf8()? {
            Cow::Owned(s) => Ok(Cow::Owned(s)),
            Cow::Borrowed(_) => Ok(string)
        }
    }

    /// Returns a URL-decoded version of the string.
    ///
    /// Any invalid UTF-8 percent-encoded byte sequences will be replaced �
    /// U+FFFD, the replacement character. This is identical to lossy percent
    /// decoding except that `+` characters are converted into spaces. This is
    /// the encoding used by form values.
    ///
    /// # Example
    ///
    /// With a valid string:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str: &RawStr = "Hello%2C+world%21".into();
    /// let decoded = raw_str.url_decode_lossy();
    /// assert_eq!(decoded, "Hello, world!");
    /// ```
    ///
    /// With an invalid string:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// // Note: Rocket should never hand you a bad `&RawStr`.
    /// let bad_str = unsafe { std::str::from_utf8_unchecked(b"a+b=\xff") };
    /// let bad_raw_str = RawStr::new(bad_str);
    /// assert_eq!(bad_raw_str.url_decode_lossy(), "a b=�");
    /// ```
    pub fn url_decode_lossy(&self) -> Cow<'_, str> {
        let string = self._replace_plus();
        match percent_encoding::percent_decode(string.as_bytes()).decode_utf8_lossy() {
            Cow::Owned(s) => Cow::Owned(s),
            Cow::Borrowed(_) => string
        }
    }

    /// Returns an HTML escaped version of `self`. Allocates only when
    /// characters need to be escaped.
    ///
    /// The following characters are escaped: `&`, `<`, `>`, `"`, `'`, `/`,
    /// <code>`</code>. **This suffices as long as the escaped string is not
    /// used in an execution context such as inside of &lt;script> or &lt;style>
    /// tags!** See the [OWASP XSS Prevention Rules] for more information.
    ///
    /// [OWASP XSS Prevention Rules]: https://www.owasp.org/index.php/XSS_%28Cross_Site_Scripting%29_Prevention_Cheat_Sheet#XSS_Prevention_Rules
    ///
    /// # Example
    ///
    /// Strings with HTML sequences are escaped:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str: &RawStr = "<b>Hi!</b>".into();
    /// let escaped = raw_str.html_escape();
    /// assert_eq!(escaped, "&lt;b&gt;Hi!&lt;&#x2F;b&gt;");
    ///
    /// let raw_str: &RawStr = "Hello, <i>world!</i>".into();
    /// let escaped = raw_str.html_escape();
    /// assert_eq!(escaped, "Hello, &lt;i&gt;world!&lt;&#x2F;i&gt;");
    /// ```
    ///
    /// Strings without HTML sequences remain untouched:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str: &RawStr = "Hello!".into();
    /// let escaped = raw_str.html_escape();
    /// assert_eq!(escaped, "Hello!");
    ///
    /// let raw_str: &RawStr = "大阪".into();
    /// let escaped = raw_str.html_escape();
    /// assert_eq!(escaped, "大阪");
    /// ```
    // NOTE: This is the ~fastest (a table-based implementation is slightly
    // faster) implementation benchmarked for dense-ish escaping. For sparser
    // texts, a regex-based-find solution is much faster.
    pub fn html_escape(&self) -> Cow<'_, str> {
        let mut escaped = false;
        let mut allocated = Vec::new(); // this is allocation free
        for c in self.as_bytes() {
            match *c {
                b'&' | b'<' | b'>' | b'"' | b'\'' | b'/' | b'`' => {
                    if !escaped {
                        let i = (c as *const u8 as usize) - (self.as_ptr() as usize);
                        allocated = Vec::with_capacity(self.len() * 2);
                        allocated.extend_from_slice(&self.as_bytes()[..i]);
                    }

                    match *c {
                        b'&' => allocated.extend_from_slice(b"&amp;"),
                        b'<' => allocated.extend_from_slice(b"&lt;"),
                        b'>' => allocated.extend_from_slice(b"&gt;"),
                        b'"' => allocated.extend_from_slice(b"&quot;"),
                        b'\'' => allocated.extend_from_slice(b"&#x27;"),
                        b'/' => allocated.extend_from_slice(b"&#x2F;"),
                        // Old versions of IE treat a ` as a '.
                        b'`' => allocated.extend_from_slice(b"&#96;"),
                        _ => unreachable!()
                    }

                    escaped = true;
                }
                _ if escaped => allocated.push(*c),
                _ => {  }
            }
        }

        if escaped {
            // This use of `unsafe` is only correct if the bytes in `allocated`
            // form a valid UTF-8 string. We prove this by cases:
            //
            // 1. In the `!escaped` branch, capacity for the vector is first
            //    allocated. No characters are pushed to `allocated` prior to
            //    this branch running since the `escaped` flag isn't set. To
            //    enter this branch, the current byte must be a valid ASCII
            //    character. This implies that every byte preceding forms a
            //    valid UTF-8 string since ASCII characters in UTF-8 are never
            //    part of a multi-byte sequence. Thus, extending the `allocated`
            //    vector with these bytes results in a valid UTF-8 string in
            //    `allocated`.
            //
            // 2. After the `!escaped` branch, `allocated` is extended with a
            //    byte string of valid ASCII characters. Thus, `allocated` is
            //    still a valid UTF-8 string.
            //
            // 3. In the `_ if escaped` branch, the byte is simply pushed into
            //    the `allocated` vector. At this point, `allocated` may contain
            //    an invalid UTF-8 string as we are not a known boundary.
            //    However, note that this byte is part of a known valid string
            //    (`self`). If the byte is not part of a multi-byte sequence, it
            //    is ASCII, and no further justification is needed. If the byte
            //    _is_ part of a multi-byte sequence, it is _not_ ASCII, and
            //    thus will not fall into the escaped character set and it will
            //    be pushed into `allocated` subsequently, resulting in a valid
            //    UTF-8 string in `allocated`.
            unsafe { Cow::Owned(String::from_utf8_unchecked(allocated)) }
        } else {
            Cow::Borrowed(self.as_str())
        }
    }

    /// Returns the length of `self`.
    ///
    /// This length is in bytes, not [`char`]s or graphemes. In other words,
    /// it may not be what a human considers the length of the string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Hello, world!");
    /// assert_eq!(raw_str.len(), 13);
    /// ```
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if `self` has a length of zero bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Hello, world!");
    /// assert!(!raw_str.is_empty());
    ///
    /// let raw_str = RawStr::new("");
    /// assert!(raw_str.is_empty());
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converts `self` into an `&str`.
    ///
    /// This method should be used sparingly. **Only use this method when you
    /// are absolutely certain that doing so is safe.**
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Hello, world!");
    /// assert_eq!(raw_str.as_str(), "Hello, world!");
    /// ```
    #[inline(always)]
    pub const fn as_str(&self) -> &str {
        &self.0
    }

    /// Converts `self` into an `&[u8]`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("hi");
    /// assert_eq!(raw_str.as_bytes(), &[0x68, 0x69]);
    /// ```
    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Converts a string slice to a raw pointer.
    ///
    /// As string slices are a slice of bytes, the raw pointer points to a
    /// [`u8`]. This pointer will be pointing to the first byte of the string
    /// slice.
    ///
    /// The caller must ensure that the returned pointer is never written to.
    /// If you need to mutate the contents of the string slice, use [`as_mut_ptr`].
    ///
    /// [`as_mut_ptr`]: str::as_mut_ptr
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("hi");
    /// let ptr = raw_str.as_ptr();
    /// ```
    pub const fn as_ptr(&self) -> *const u8 {
        self.as_str().as_ptr()
    }

    /// Converts `self` into an `&UncasedStr`.
    ///
    /// This method should be used sparingly. **Only use this method when you
    /// are absolutely certain that doing so is safe.**
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::new("Content-Type");
    /// assert!(raw_str.as_uncased_str() == "content-TYPE");
    /// ```
    #[inline(always)]
    pub fn as_uncased_str(&self) -> &UncasedStr {
        self.as_str().into()
    }

    /// Returns `true` if the given pattern matches a sub-slice of
    /// this string slice.
    ///
    /// Returns `false` if it does not.
    ///
    /// The pattern can be a `&str`, [`char`], a slice of [`char`]s, or a
    /// function or closure that determines if a character matches.
    ///
    /// [`char`]: prim@char
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let bananas = RawStr::new("bananas");
    ///
    /// assert!(bananas.contains("nana"));
    /// assert!(!bananas.contains("apples"));
    /// ```
    #[inline]
    pub fn contains<'a, P: Pattern<'a>>(&'a self, pat: P) -> bool {
        pat.is_contained_in(self.as_str())
    }

    /// Returns `true` if the given pattern matches a prefix of this
    /// string slice.
    ///
    /// Returns `false` if it does not.
    ///
    /// The pattern can be a `&str`, [`char`], a slice of [`char`]s, or a
    /// function or closure that determines if a character matches.
    ///
    /// [`char`]: prim@char
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let bananas = RawStr::new("bananas");
    ///
    /// assert!(bananas.starts_with("bana"));
    /// assert!(!bananas.starts_with("nana"));
    /// ```
    pub fn starts_with<'a, P: Pattern<'a>>(&'a self, pat: P) -> bool {
        pat.is_prefix_of(self.as_str())
    }

    /// Returns `true` if the given pattern matches a suffix of this
    /// string slice.
    ///
    /// Returns `false` if it does not.
    ///
    /// The pattern can be a `&str`, [`char`], a slice of [`char`]s, or a
    /// function or closure that determines if a character matches.
    ///
    /// [`char`]: prim@char
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let bananas = RawStr::new("bananas");
    ///
    /// assert!(bananas.ends_with("anas"));
    /// assert!(!bananas.ends_with("nana"));
    /// ```
    pub fn ends_with<'a, P>(&'a self, pat: P) -> bool
        where P: Pattern<'a>, <P as Pattern<'a>>::Searcher: ReverseSearcher<'a>
    {
        pat.is_suffix_of(self.as_str())
    }


    /// Returns the byte index of the first character of this string slice that
    /// matches the pattern.
    ///
    /// Returns [`None`] if the pattern doesn't match.
    ///
    /// The pattern can be a `&str`, [`char`], a slice of [`char`]s, or a
    /// function or closure that determines if a character matches.
    ///
    /// [`char`]: prim@char
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let s = RawStr::new("Löwe 老虎 Léopard Gepardi");
    ///
    /// assert_eq!(s.find('L'), Some(0));
    /// assert_eq!(s.find('é'), Some(14));
    /// assert_eq!(s.find("pard"), Some(17));
    /// ```
    #[inline]
    pub fn find<'a, P: Pattern<'a>>(&'a self, pat: P) -> Option<usize> {
        pat.into_searcher(self.as_str()).next_match().map(|(i, _)| i)
    }

    /// An iterator over substrings of this string slice, separated by
    /// characters matched by a pattern.
    ///
    /// The pattern can be a `&str`, [`char`], a slice of [`char`]s, or a
    /// function or closure that determines if a character matches.
    ///
    /// [`char`]: prim@char
    ///
    /// # Examples
    ///
    /// Simple patterns:
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let v: Vec<_> = RawStr::new("Mary had a little lamb")
    ///     .split(' ')
    ///     .map(|r| r.as_str())
    ///     .collect();
    ///
    /// assert_eq!(v, ["Mary", "had", "a", "little", "lamb"]);
    /// ```
    #[inline]
    pub fn split<'a, P>(&'a self, pat: P) -> impl Iterator<Item = &'a RawStr>
        where P: Pattern<'a>
    {
        let split: Split<'_, P> = Split(SplitInternal {
            start: 0,
            end: self.len(),
            matcher: pat.into_searcher(self.as_str()),
            allow_trailing_empty: true,
            finished: false,
        });

        split.map(|s| s.into())
    }

    /// Splits `self` into two pieces: the piece _before_ the first byte `b` and
    /// the piece _after_ (not including `b`). Returns the tuple (`before`,
    /// `after`). If `b` is not in `self`, or `b` is not an ASCII characters,
    /// returns the entire string `self` as `before` and the empty string as
    /// `after`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let haystack = RawStr::new("a good boy!");
    ///
    /// let (before, after) = haystack.split_at_byte(b'a');
    /// assert_eq!(before, "");
    /// assert_eq!(after, " good boy!");
    ///
    /// let (before, after) = haystack.split_at_byte(b' ');
    /// assert_eq!(before, "a");
    /// assert_eq!(after, "good boy!");
    ///
    /// let (before, after) = haystack.split_at_byte(b'o');
    /// assert_eq!(before, "a g");
    /// assert_eq!(after, "od boy!");
    ///
    /// let (before, after) = haystack.split_at_byte(b'!');
    /// assert_eq!(before, "a good boy");
    /// assert_eq!(after, "");
    ///
    /// let (before, after) = haystack.split_at_byte(b'?');
    /// assert_eq!(before, "a good boy!");
    /// assert_eq!(after, "");
    ///
    /// let haystack = RawStr::new("");
    /// let (before, after) = haystack.split_at_byte(b' ');
    /// assert_eq!(before, "");
    /// assert_eq!(after, "");
    /// ```
    #[inline]
    pub fn split_at_byte(&self, b: u8) -> (&RawStr, &RawStr) {
        if !b.is_ascii() {
            return (self, &self[0..0]);
        }

        match memchr::memchr(b, self.as_bytes()) {
            // SAFETY: `b` is a character boundary since it's ASCII, `i` is in
            // bounds in `self` (or else None), and i is at most len - 1, so i +
            // 1 is at most len.
            Some(i) => unsafe {
                let s = self.as_str();
                let start = s.get_unchecked(0..i);
                let end = s.get_unchecked((i + 1)..self.len());
                (start.into(), end.into())
            },
            None => (self, &self[0..0])
        }
    }

    /// Returns a string slice with the prefix removed.
    ///
    /// If the string starts with the pattern `prefix`, returns substring after
    /// the prefix, wrapped in `Some`. This method removes the prefix exactly
    /// once.
    ///
    /// If the string does not start with `prefix`, returns `None`.
    ///
    /// The pattern can be a `&str`, `char`, a slice of `char`s, or a function
    /// or closure that determines if a character matches.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// assert_eq!(RawStr::new("foo:bar").strip_prefix("foo:").unwrap(), "bar");
    /// assert_eq!(RawStr::new("foofoo").strip_prefix("foo").unwrap(), "foo");
    /// assert!(RawStr::new("foo:bar").strip_prefix("bar").is_none());
    /// ```
    #[inline]
    pub fn strip_prefix<'a, P: Pattern<'a>>(&'a self, prefix: P) -> Option<&'a RawStr> {
        prefix.strip_prefix_of(self.as_str()).map(RawStr::new)
    }

    /// Returns a string slice with the suffix removed.
    ///
    /// If the string ends with the pattern `suffix`, returns the substring
    /// before the suffix, wrapped in `Some`.  Unlike `trim_end_matches`, this
    /// method removes the suffix exactly once.
    ///
    /// If the string does not end with `suffix`, returns `None`.
    ///
    /// The pattern can be a `&str`, `char`, a slice of `char`s, or a function
    /// or closure that determines if a character matches.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// assert_eq!(RawStr::new("bar:foo").strip_suffix(":foo").unwrap(), "bar");
    /// assert_eq!(RawStr::new("foofoo").strip_suffix("foo").unwrap(), "foo");
    /// assert!(RawStr::new("bar:foo").strip_suffix("bar").is_none());
    /// ```
    #[inline]
    pub fn strip_suffix<'a, P>(&'a self, suffix: P) -> Option<&'a RawStr>
        where P: Pattern<'a>,<P as Pattern<'a>>::Searcher: ReverseSearcher<'a>,
    {
        suffix.strip_suffix_of(self.as_str()).map(RawStr::new)
    }

    /// Parses this string slice into another type.
    ///
    /// Because `parse` is so general, it can cause problems with type
    /// inference. As such, `parse` is one of the few times you'll see
    /// the syntax affectionately known as the 'turbofish': `::<>`. This
    /// helps the inference algorithm understand specifically which type
    /// you're trying to parse into.
    ///
    /// # Errors
    ///
    /// Will return `Err` if it's not possible to parse this string slice into
    /// the desired type.
    ///
    /// # Examples
    ///
    /// Basic usage
    ///
    /// ```
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let four: u32 = RawStr::new("4").parse().unwrap();
    ///
    /// assert_eq!(4, four);
    /// ```
    #[inline]
    pub fn parse<F: std::str::FromStr>(&self) -> Result<F, F::Err> {
        std::str::FromStr::from_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
mod serde {
    use serde_::{ser, de, Serialize, Deserialize};

    use super::*;

    impl Serialize for RawStr {
        fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
            where S: ser::Serializer
        {
            self.as_str().serialize(ser)
        }
    }

    impl<'de: 'a, 'a> Deserialize<'de> for &'a RawStr {
        fn deserialize<D>(de: D) -> Result<Self, D::Error>
            where D: de::Deserializer<'de>
        {
            <&'a str as Deserialize<'de>>::deserialize(de).map(RawStr::new)
        }
    }

}

impl fmt::Debug for RawStr {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> From<&'a str> for &'a RawStr {
    #[inline(always)]
    fn from(string: &'a str) -> &'a RawStr {
        RawStr::new(string)
    }
}

impl<'a> From<&'a RawStr> for Cow<'a, RawStr> {
    fn from(raw: &'a RawStr) -> Self {
        Cow::Borrowed(raw)
    }
}

impl From<RawStrBuf> for Cow<'_, RawStr> {
    fn from(raw: RawStrBuf) -> Self {
        Cow::Owned(raw)
    }
}

macro_rules! impl_partial {
    ($A:ty : $B:ty as $T:ty) => (
        impl PartialEq<$A> for $B {
            #[inline(always)]
            fn eq(&self, other: &$A) -> bool {
                let left: $T = self.as_ref();
                let right: $T = other.as_ref();
                left == right
            }
        }

        impl PartialOrd<$A> for $B {
            #[inline(always)]
            fn partial_cmp(&self, other: &$A) -> Option<Ordering> {
                let left: $T = self.as_ref();
                let right: $T = other.as_ref();
                left.partial_cmp(right)
            }
        }
    );
    ($A:ty : $B:ty) => (impl_partial!($A : $B as &str);)
}

impl_partial!(RawStr : &RawStr);
impl_partial!(&RawStr : RawStr);

impl_partial!(str : RawStr);
impl_partial!(str : &RawStr);
impl_partial!(&str : RawStr);
impl_partial!(&&str : RawStr);

impl_partial!(Cow<'_, str> : RawStr);
impl_partial!(Cow<'_, str> : &RawStr);
impl_partial!(RawStr : Cow<'_, str>);
impl_partial!(&RawStr : Cow<'_, str>);

impl_partial!(Cow<'_, RawStr> : RawStr as &RawStr);
impl_partial!(Cow<'_, RawStr> : &RawStr as &RawStr);
impl_partial!(RawStr : Cow<'_, RawStr> as &RawStr);
impl_partial!(&RawStr : Cow<'_, RawStr> as &RawStr);

impl_partial!(String : RawStr);
impl_partial!(String : &RawStr);

impl_partial!(RawStr : String);
impl_partial!(&RawStr : String);

impl_partial!(RawStr : str);
impl_partial!(RawStr : &str);
impl_partial!(RawStr : &&str);
impl_partial!(&RawStr : str);

impl AsRef<str> for RawStr {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<RawStr> for str {
    #[inline(always)]
    fn as_ref(&self) -> &RawStr {
        RawStr::new(self)
    }
}

impl AsRef<RawStr> for RawStr {
    #[inline(always)]
    fn as_ref(&self) -> &RawStr {
        self
    }
}

impl AsRef<[u8]> for RawStr {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<I: core::slice::SliceIndex<str, Output=str>> core::ops::Index<I> for RawStr {
    type Output = RawStr;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.as_str()[index].into()
    }
}

impl std::borrow::Borrow<str> for RawStr {
    #[inline(always)]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl std::borrow::Borrow<RawStr> for &str {
    #[inline(always)]
    fn borrow(&self) -> &RawStr {
        (*self).into()
    }
}

impl fmt::Display for RawStr {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<RawStr> for RawStrBuf {
    #[inline(always)]
    fn as_ref(&self) -> &RawStr {
        RawStr::new(self.0.as_str())
    }
}

impl Borrow<RawStr> for RawStrBuf {
    #[inline(always)]
    fn borrow(&self) -> &RawStr {
        self.as_ref()
    }
}

impl std::ops::Deref for RawStrBuf {
    type Target = RawStr;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl fmt::Display for RawStrBuf {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for RawStrBuf {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for RawStrBuf {
    #[inline(always)]
    fn from(string: String) -> Self {
        RawStrBuf(string)
    }
}

impl From<&str> for RawStrBuf {
    #[inline(always)]
    fn from(string: &str) -> Self {
        string.to_string().into()
    }
}

impl From<&RawStr> for RawStrBuf {
    #[inline(always)]
    fn from(raw: &RawStr) -> Self {
        raw.to_string().into()
    }
}

#[cfg(test)]
mod tests {
    use super::RawStr;

    #[test]
    fn can_compare() {
        let raw_str = RawStr::new("abc");
        assert_eq!(raw_str, "abc");
        assert_eq!("abc", raw_str.as_str());
        assert_eq!(raw_str, RawStr::new("abc"));
        assert_eq!(raw_str, "abc".to_string());
        assert_eq!("abc".to_string(), raw_str.as_str());
    }
}
