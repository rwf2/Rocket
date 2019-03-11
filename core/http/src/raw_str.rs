use std::ops::{Deref, DerefMut};
use std::borrow::Cow;
use std::convert::AsRef;
use std::cmp::Ordering;
use std::str::Utf8Error;
use std::fmt;

use uncased::UncasedStr;

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
/// through a reference an as `&RawStr` (just like &str). You'll likely
/// encounter an `&RawStr` as a parameter via [`FromParam`] or as a form value
/// via [`FromFormValue`].
///
/// [`FromParam`]: ::rocket::request::FromParam
/// [`FromFormValue`]: ::rocket::request::FromFormValue
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawStr(str);

impl RawStr {
    /// Constructs an `&RawStr` from an `&str` at no cost.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::RawStr;
    ///
    /// let raw_str = RawStr::from_str("Hello, world!");
    ///
    /// // `into` can also be used; note that the type must be specified
    /// let raw_str: &RawStr = "Hello, world!".into();
    /// ```
    #[inline(always)]
    pub fn from_str(string: &str) -> &RawStr {
        string.into()
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
    /// let raw_str = RawStr::from_str("Hello%21");
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
    /// let bad_str = unsafe { ::std::str::from_utf8_unchecked(b"a=\xff") };
    /// let bad_raw_str = RawStr::from_str(bad_str);
    /// assert!(bad_raw_str.percent_decode().is_err());
    /// ```
    #[inline(always)]
    pub fn percent_decode(&self) -> Result<Cow<str>, Utf8Error> {
        ::percent_encoding::percent_decode(self.as_bytes()).decode_utf8()
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
    /// let raw_str = RawStr::from_str("Hello%21");
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
    /// let bad_str = unsafe { ::std::str::from_utf8_unchecked(b"a=\xff") };
    /// let bad_raw_str = RawStr::from_str(bad_str);
    /// assert_eq!(bad_raw_str.percent_decode_lossy(), "a=�");
    /// ```
    #[inline(always)]
    pub fn percent_decode_lossy(&self) -> Cow<str> {
        ::percent_encoding::percent_decode(self.as_bytes()).decode_utf8_lossy()
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
    /// let raw_str: &RawStr = "Hello%2C+world%21".into();
    /// let decoded = raw_str.url_decode();
    /// assert_eq!(decoded, Ok("Hello, world!".to_string()));
    /// ```
    pub fn url_decode(&self) -> Result<String, Utf8Error> {
        // TODO: Make this more efficient!
        let replaced = self.replace("+", " ");
        RawStr::from_str(replaced.as_str())
            .percent_decode()
            .map(|cow| cow.into_owned())
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
    /// let bad_str = unsafe { ::std::str::from_utf8_unchecked(b"a+b=\xff") };
    /// let bad_raw_str = RawStr::from_str(bad_str);
    /// assert_eq!(bad_raw_str.url_decode_lossy(), "a b=�");
    /// ```
    pub fn url_decode_lossy(&self) -> String {
        let replaced = self.replace("+", " ");
        RawStr::from_str(replaced.as_str())
            .percent_decode_lossy()
            .into_owned()
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
    pub fn html_escape(&self) -> Cow<str> {
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
    /// let raw_str = RawStr::from_str("Hello, world!");
    /// assert_eq!(raw_str.as_str(), "Hello, world!");
    /// ```
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self
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
    /// let raw_str = RawStr::from_str("Content-Type");
    /// assert!(raw_str.as_uncased_str() == "content-TYPE");
    /// ```
    #[inline(always)]
    pub fn as_uncased_str(&self) -> &UncasedStr {
        self.as_str().into()
    }
}

impl<'a> From<&'a str> for &'a RawStr {
    #[inline(always)]
    fn from(string: &'a str) -> &'a RawStr {
        // This is simply a `newtype`-like transformation. The `repr(C)` ensures
        // that this is safe and correct. Note this exact pattern appears often
        // in the standard library.
        unsafe { &*(string as *const str as *const RawStr) }
    }
}

impl PartialEq<str> for RawStr {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<String> for RawStr {
    #[inline(always)]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<String> for &'a RawStr {
    #[inline(always)]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialOrd<str> for RawStr {
    #[inline(always)]
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        (self as &str).partial_cmp(other)
    }
}

impl AsRef<str> for RawStr {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<[u8]> for RawStr {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl ToString for RawStr {
    #[inline(always)]
    fn to_string(&self) -> String {
        String::from(self.as_str())
    }
}

impl Deref for RawStr {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &str {
        &self.0
    }
}

impl DerefMut for RawStr {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut str {
        &mut self.0
    }
}

impl fmt::Display for RawStr {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::RawStr;

    #[test]
    fn can_compare() {
        let raw_str = RawStr::from_str("abc");
        assert_eq!(raw_str, "abc");
        assert_eq!("abc", raw_str.as_str());
        assert_eq!(raw_str, RawStr::from_str("abc"));
        assert_eq!(raw_str, "abc".to_string());
        assert_eq!("abc".to_string(), raw_str.as_str());
    }
}
